#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod svg;

use app::{
    copy_image_to_clipboard, copy_text_to_clipboard, detect_editors, open_in_editor,
    open_in_system_default, AppConfig, AppState, BackgroundStyle,
};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use svg::{
    export_png, ExportBackground, ExportOptions, RenderBackground, SvgEngine, ViewportParams,
};

slint::include_modules!();

type DragAnchor = Option<(f32, f32, f32, f32)>;

fn main() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;

    let engine = Rc::new(RefCell::new(SvgEngine::new()));
    let state = Rc::new(RefCell::new(AppState::default()));
    let drag_start: Rc<RefCell<DragAnchor>> = Rc::new(RefCell::new(None));

    // Detect installed editors (Inkscape, Illustrator, CorelDRAW, Affinity, VS Code)
    let detected_editors = detect_editors();
    let editors_store = Rc::new(detected_editors.clone());

    let editor_models: Vec<EditorInfo> = detected_editors
        .iter()
        .map(|e| EditorInfo {
            id: SharedString::from(&e.id),
            name: SharedString::from(&e.name),
            icon: SharedString::from(&e.icon_text),
        })
        .collect();
    let model_rc: ModelRc<EditorInfo> = Rc::new(VecModel::from(editor_models)).into();
    window.set_available_editors(model_rc);

    // Load persisted user preferences (defaults to Light Mode + White Background)
    let config = Rc::new(RefCell::new(AppConfig::load()));
    let initial_cfg = config.borrow().clone();

    window.set_is_light_mode(initial_cfg.is_light_mode);
    window.set_bg_mode(initial_cfg.bg_mode as i32);
    window.set_export_scale_preset(initial_cfg.export_scale_preset as i32);
    window.set_export_bg_mode(initial_cfg.export_bg_mode as i32);

    state.borrow_mut().bg_style = match initial_cfg.bg_mode {
        0 => BackgroundStyle::Checkerboard,
        2 => BackgroundStyle::Black,
        3 => BackgroundStyle::Custom,
        _ => BackgroundStyle::White,
    };

    // Static embedded banner and icon assets
    const BANNER_BYTES: &[u8] = include_bytes!("../resources/SvagView_Logo_Banner_with_Text.png");
    const APP_ICO_BYTES: &[u8] = include_bytes!("../resources/SvagView_Logo_Icon.ico");

    // Initialize crisp window/UI app icon and Lanczos3 downscaled opening banner
    let scale_factor = window.window().scale_factor();
    let icon_target_px = ((64.0 * scale_factor).round() as u32).clamp(64, 256);
    if let Some(app_icon) = svg::decode_ico_to_slint_image(APP_ICO_BYTES, icon_target_px) {
        window.set_app_icon(app_icon);
    }

    let banner_target_w = (500.0 * scale_factor).round() as u32;
    let banner_target_h = ((banner_target_w as f32) * (794.0 / 3290.0)).round() as u32;
    if let Some(banner_img) =
        svg::create_lanczos_scaled_image(BANNER_BYTES, banner_target_w, banner_target_h)
    {
        window.set_banner_image(banner_img);
    }

    // Helper: Toast notification banner
    let toast_timer: Rc<RefCell<Option<slint::Timer>>> = Rc::new(RefCell::new(None));
    let show_toast = {
        let window_weak = window.as_weak();
        let toast_timer = Rc::clone(&toast_timer);
        move |msg: &str, is_err: bool| {
            let Some(win) = window_weak.upgrade() else {
                return;
            };
            win.set_toast_message(SharedString::from(msg));
            win.set_toast_is_error(is_err);
            win.set_show_toast(true);

            let win_weak = win.as_weak();
            let timer = slint::Timer::default();
            timer.start(
                slint::TimerMode::SingleShot,
                std::time::Duration::from_millis(3500),
                move || {
                    if let Some(w) = win_weak.upgrade() {
                        w.set_show_toast(false);
                    }
                },
            );
            *toast_timer.borrow_mut() = Some(timer);
        }
    };

    // Helper: update display and render SVG live directly to viewport
    let update_view = {
        let window_weak = window.as_weak();
        let engine = Rc::clone(&engine);
        let state = Rc::clone(&state);

        move || {
            let Some(win) = window_weak.upgrade() else {
                return;
            };
            let eng = engine.borrow();
            let st = state.borrow();

            let Some(doc) = eng.current_document() else {
                win.set_has_document(false);
                return;
            };

            let canvas_w = win.get_canvas_width();
            let canvas_h = win.get_canvas_height();

            if canvas_w <= 0.0 || canvas_h <= 0.0 || !canvas_w.is_finite() || !canvas_h.is_finite()
            {
                return;
            }

            let svg_w = doc.metadata.width;
            let svg_h = doc.metadata.height;

            if svg_w <= 0.0 || svg_h <= 0.0 || !svg_w.is_finite() || !svg_h.is_finite() {
                return;
            }

            let scale_factor = {
                let sf = win.window().scale_factor();
                if sf.is_finite() && sf > 0.0 {
                    sf
                } else {
                    1.0
                }
            };
            let pix_w = (canvas_w * scale_factor).round() as u32;
            let pix_h = (canvas_h * scale_factor).round() as u32;

            if pix_w == 0 || pix_h == 0 {
                return;
            }

            let zoom = if st.zoom.is_finite() && st.zoom > 0.0 {
                st.zoom
            } else {
                1.0
            };
            let pan_x = if st.pan_x.is_finite() { st.pan_x } else { 0.0 };
            let pan_y = if st.pan_y.is_finite() { st.pan_y } else { 0.0 };

            let top_left_x = (canvas_w - svg_w * zoom) / 2.0 + pan_x;
            let top_left_y = (canvas_h - svg_h * zoom) / 2.0 + pan_y;

            let phys_art_x = top_left_x * scale_factor;
            let phys_art_y = top_left_y * scale_factor;
            let phys_art_w = svg_w * zoom * scale_factor;
            let phys_art_h = svg_h * zoom * scale_factor;
            let render_scale = zoom * scale_factor;

            let bg = match st.bg_style {
                BackgroundStyle::Checkerboard => RenderBackground::Checkerboard,
                BackgroundStyle::White => RenderBackground::White,
                BackgroundStyle::Black => RenderBackground::Black,
                BackgroundStyle::Custom => RenderBackground::Slate,
            };

            let is_light = win.get_is_light_mode();

            let params = ViewportParams {
                canvas_w: pix_w,
                canvas_h: pix_h,
                svg_x: phys_art_x,
                svg_y: phys_art_y,
                svg_w: phys_art_w,
                svg_h: phys_art_h,
                scale: render_scale,
                bg,
                is_light_mode: is_light,
            };

            match eng.render_viewport(&params) {
                Ok(res) => {
                    win.set_viewport_image(res.image);
                    win.set_render_time_text(SharedString::from(format!(
                        "{:.1} ms",
                        res.render_time_ms
                    )));
                }
                Err(err) => {
                    eprintln!("Viewport render error: {}", err);
                }
            }

            win.set_zoom_text(SharedString::from(format!("{:.0}%", zoom * 100.0)));
        }
    };

    // Helper: load file and configure metadata
    let load_file = {
        let window_weak = window.as_weak();
        let engine = Rc::clone(&engine);
        let state = Rc::clone(&state);
        let update_view = update_view.clone();
        let show_toast = show_toast.clone();

        move |path: PathBuf| {
            let Some(win) = window_weak.upgrade() else {
                return;
            };

            let doc_res = {
                let mut eng = engine.borrow_mut();
                eng.load_file(&path)
            };

            match doc_res {
                Ok(doc) => {
                    let mut st = state.borrow_mut();
                    st.set_current_file(path.clone());

                    win.set_file_name(SharedString::from(&doc.metadata.file_name));
                    win.set_file_path(SharedString::from(&doc.metadata.file_path));
                    win.set_dimensions_text(SharedString::from(format!(
                        "{:.0} × {:.0} px",
                        doc.metadata.width, doc.metadata.height
                    )));
                    win.set_view_box_text(SharedString::from(&doc.metadata.view_box));
                    win.set_file_size_text(SharedString::from(&doc.metadata.file_size_formatted));
                    win.set_meta_parse_time(SharedString::from(format!(
                        "{:.1} ms",
                        doc.parse_time_ms
                    )));

                    let (cur_idx, total) = st.current_file_info();
                    if total > 1 {
                        win.set_folder_counter(SharedString::from(format!(
                            "[ {} / {} ]",
                            cur_idx, total
                        )));
                    } else {
                        win.set_folder_counter(SharedString::from(""));
                    }

                    win.set_meta_paths(doc.metadata.path_count as i32);
                    win.set_meta_groups(doc.metadata.group_count as i32);
                    win.set_meta_texts(doc.metadata.text_count as i32);
                    win.set_meta_gradients(doc.metadata.gradient_count as i32);
                    win.set_meta_clips(doc.metadata.clip_path_count as i32);
                    win.set_meta_masks(doc.metadata.mask_count as i32);
                    win.set_meta_filters(doc.metadata.filter_count as i32);

                    // Update export dimensions display
                    let sw = doc.metadata.width;
                    let sh = doc.metadata.height;
                    let preset = win.get_export_scale_preset();
                    let scale = match preset {
                        2 => 2.0,
                        4 => 4.0,
                        8 => 8.0,
                        _ => 1.0,
                    };
                    win.set_export_dimensions_text(SharedString::from(format!(
                        "{:.0} × {:.0} px",
                        (sw * scale).round(),
                        (sh * scale).round()
                    )));

                    win.set_has_document(true);

                    // Fit to window initially
                    let canvas_w = win.get_canvas_width();
                    let canvas_h = win.get_canvas_height();

                    if canvas_w > 50.0 && canvas_h > 50.0 {
                        st.fit_to_window(
                            canvas_w,
                            canvas_h,
                            doc.metadata.width,
                            doc.metadata.height,
                        );
                        drop(st);
                        update_view();
                    } else {
                        // Window layout may still be initializing; schedule a layout fit
                        let svg_w = doc.metadata.width;
                        let svg_h = doc.metadata.height;
                        let state = Rc::clone(&state);
                        let window_weak = win.as_weak();
                        let update_view = update_view.clone();

                        drop(st);
                        slint::Timer::single_shot(
                            std::time::Duration::from_millis(30),
                            move || {
                                if let Some(w) = window_weak.upgrade() {
                                    let cw = w.get_canvas_width();
                                    let ch = w.get_canvas_height();
                                    if cw > 0.0 && ch > 0.0 {
                                        state.borrow_mut().fit_to_window(cw, ch, svg_w, svg_h);
                                        update_view();
                                    }
                                }
                            },
                        );
                    }
                }
                Err(err) => {
                    eprintln!("Failed to load SVG '{}': {}", path.display(), err);
                    show_toast(&format!("Failed to open SVG: {}", err), true);
                }
            }
        }
    };

    // Open file action
    {
        let load_file = load_file.clone();
        window.on_open_file_clicked(move || {
            let file = rfd::FileDialog::new()
                .add_filter("Scalable Vector Graphics (*.svg, *.svgz)", &["svg", "svgz"])
                .add_filter("All Files (*.*)", &["*"])
                .set_title("Open SVG Image")
                .pick_file();

            if let Some(path) = file {
                load_file(path);
            }
        });
    }

    // Previous file navigation
    {
        let state = Rc::clone(&state);
        let load_file = load_file.clone();
        window.on_prev_file_clicked(move || {
            let prev = {
                let mut st = state.borrow_mut();
                st.prev_file()
            };
            if let Some(p) = prev {
                load_file(p);
            }
        });
    }

    // Next file navigation
    {
        let state = Rc::clone(&state);
        let load_file = load_file.clone();
        window.on_next_file_clicked(move || {
            let next = {
                let mut st = state.borrow_mut();
                st.next_file()
            };
            if let Some(p) = next {
                load_file(p);
            }
        });
    }

    // First file navigation (Home key)
    {
        let state = Rc::clone(&state);
        let load_file = load_file.clone();
        window.on_first_file_clicked(move || {
            let first = {
                let mut st = state.borrow_mut();
                st.first_file()
            };
            if let Some(p) = first {
                load_file(p);
            }
        });
    }

    // Last file navigation (End key)
    {
        let state = Rc::clone(&state);
        let load_file = load_file.clone();
        window.on_last_file_clicked(move || {
            let last = {
                let mut st = state.borrow_mut();
                st.last_file()
            };
            if let Some(p) = last {
                load_file(p);
            }
        });
    }

    // Dismiss toast notification
    {
        let window_weak = window.as_weak();
        window.on_dismiss_toast(move || {
            if let Some(win) = window_weak.upgrade() {
                win.set_show_toast(false);
            }
        });
    }

    // Update dynamic export resolution text when preset changes
    {
        let window_weak = window.as_weak();
        let engine = Rc::clone(&engine);
        let config = Rc::clone(&config);
        window.on_export_scale_changed(move |preset| {
            let Some(win) = window_weak.upgrade() else {
                return;
            };
            {
                let mut cfg = config.borrow_mut();
                cfg.export_scale_preset = preset as u8;
                let _ = cfg.save();
            }
            let eng = engine.borrow();
            if let Some(doc) = eng.current_document() {
                let scale = match preset {
                    2 => 2.0,
                    4 => 4.0,
                    8 => 8.0,
                    _ => 1.0,
                };
                let sw = doc.metadata.width;
                let sh = doc.metadata.height;
                win.set_export_dimensions_text(SharedString::from(format!(
                    "{:.0} × {:.0} px",
                    (sw * scale).round(),
                    (sh * scale).round()
                )));
            }
        });
    }

    // Reload current file
    {
        let state = Rc::clone(&state);
        let load_file = load_file.clone();
        window.on_reload_clicked(move || {
            let cur = {
                let st = state.borrow();
                st.current_path.clone()
            };
            if let Some(p) = cur {
                load_file(p);
            }
        });
    }

    // Copy Image to OS clipboard
    {
        let engine = Rc::clone(&engine);
        let show_toast = show_toast.clone();
        window.on_copy_image_clicked(move || {
            let eng = engine.borrow();
            if let Some(doc) = eng.current_document() {
                let opt = ExportOptions {
                    scale: 2.0,
                    custom_width: None,
                    custom_height: None,
                    background: ExportBackground::Transparent,
                };
                match svg::render_for_export(&doc.tree, &opt) {
                    Ok(pixmap) => match copy_image_to_clipboard(&pixmap) {
                        Ok(()) => show_toast("Copied PNG image to clipboard", false),
                        Err(e) => {
                            eprintln!("Clipboard error: {}", e);
                            show_toast(&format!("Clipboard error: {}", e), true);
                        }
                    },
                    Err(e) => {
                        show_toast(&format!("Failed to render image: {}", e), true);
                    }
                }
            }
        });
    }

    // Copy SVG XML code to clipboard
    {
        let engine = Rc::clone(&engine);
        let show_toast = show_toast.clone();
        window.on_copy_svg_text_clicked(move || {
            let eng = engine.borrow();
            if let Some(doc) = eng.current_document() {
                if let Ok(text) = std::str::from_utf8(&doc.raw_data) {
                    match copy_text_to_clipboard(text) {
                        Ok(()) => show_toast("Copied SVG XML code to clipboard", false),
                        Err(e) => show_toast(&format!("Clipboard error: {}", e), true),
                    }
                } else {
                    show_toast("Document contains non-UTF8 binary data", true);
                }
            }
        });
    }

    // Copy file path to clipboard
    {
        let state = Rc::clone(&state);
        let show_toast = show_toast.clone();
        window.on_copy_path_clicked(move || {
            let st = state.borrow();
            if let Some(p) = &st.current_path {
                match copy_text_to_clipboard(&p.to_string_lossy()) {
                    Ok(()) => show_toast("Copied file path to clipboard", false),
                    Err(e) => show_toast(&format!("Clipboard error: {}", e), true),
                }
            }
        });
    }

    // Export PNG action
    {
        let engine = Rc::clone(&engine);
        let state = Rc::clone(&state);
        let config = Rc::clone(&config);
        let show_toast = show_toast.clone();
        window.on_export_png_action(move |scale_preset, bg_mode| {
            {
                let mut cfg = config.borrow_mut();
                cfg.export_scale_preset = scale_preset as u8;
                cfg.export_bg_mode = bg_mode as u8;
                let _ = cfg.save();
            }

            let eng = engine.borrow();
            let st = state.borrow();
            let Some(doc) = eng.current_document() else {
                return;
            };

            let scale = match scale_preset {
                2 => 2.0,
                4 => 4.0,
                8 => 8.0,
                _ => 1.0,
            };

            let bg = match bg_mode {
                1 => ExportBackground::White,
                2 => ExportBackground::Black,
                _ => ExportBackground::Transparent,
            };

            let opt = ExportOptions {
                scale,
                custom_width: None,
                custom_height: None,
                background: bg,
            };

            let default_name = doc
                .path
                .as_ref()
                .and_then(|p| p.file_stem())
                .and_then(|s| s.to_str())
                .unwrap_or("export");

            let suggested_file_name = format!("{}_{}x.png", default_name, scale as u32);

            let mut dialog = rfd::FileDialog::new()
                .add_filter("PNG Image (*.png)", &["png"])
                .set_title("Export SVG as PNG")
                .set_file_name(&suggested_file_name);

            if let Some(p) = &st.current_path {
                if let Some(parent) = p.parent() {
                    dialog = dialog.set_directory(parent);
                }
            }

            if let Some(save_path) = dialog.save_file() {
                match export_png(&doc.tree, &save_path, &opt) {
                    Ok(()) => {
                        let name = save_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "export.png".to_string());
                        show_toast(&format!("Exported PNG successfully: {}", name), false);
                    }
                    Err(e) => {
                        eprintln!("Export error: {}", e);
                        show_toast(&format!("Export failed: {}", e), true);
                    }
                }
            }
        });
    }

    // Launch in detected SVG editor
    {
        let state = Rc::clone(&state);
        let editors = Rc::clone(&editors_store);
        let show_toast = show_toast.clone();
        window.on_open_editor_clicked(move |editor_id| {
            let st = state.borrow();
            if let Some(path) = &st.current_path {
                if let Some(editor) = editors.iter().find(|e| e.id == editor_id.as_str()) {
                    if let Err(e) = open_in_editor(editor, path) {
                        show_toast(&format!("Failed to launch {}: {}", editor.name, e), true);
                    }
                }
            }
        });
    }

    // Launch in system default
    {
        let state = Rc::clone(&state);
        let show_toast = show_toast.clone();
        window.on_open_default_clicked(move || {
            let st = state.borrow();
            if let Some(path) = &st.current_path {
                if let Err(e) = open_in_system_default(path) {
                    show_toast(&format!("Failed to open system default: {}", e), true);
                }
            }
        });
    }

    // Set background mode
    {
        let window_weak = window.as_weak();
        let state = Rc::clone(&state);
        let config = Rc::clone(&config);
        let update_view = update_view.clone();
        window.on_set_bg_mode(move |mode| {
            let Some(win) = window_weak.upgrade() else {
                return;
            };
            let mut st = state.borrow_mut();
            st.bg_style = match mode {
                0 => BackgroundStyle::Checkerboard,
                2 => BackgroundStyle::Black,
                3 => BackgroundStyle::Custom,
                _ => BackgroundStyle::White,
            };
            win.set_bg_mode(mode);
            drop(st);

            {
                let mut cfg = config.borrow_mut();
                cfg.bg_mode = mode as u8;
                let _ = cfg.save();
            }

            update_view();
        });
    }

    // Cycle background mode shortcut (B)
    {
        let window_weak = window.as_weak();
        let state = Rc::clone(&state);
        let config = Rc::clone(&config);
        let update_view = update_view.clone();
        window.on_cycle_bg_mode(move || {
            let Some(win) = window_weak.upgrade() else {
                return;
            };
            let mut st = state.borrow_mut();
            let next_mode = match st.bg_style {
                BackgroundStyle::White => (BackgroundStyle::Checkerboard, 0),
                BackgroundStyle::Checkerboard => (BackgroundStyle::Black, 2),
                BackgroundStyle::Black => (BackgroundStyle::Custom, 3),
                BackgroundStyle::Custom => (BackgroundStyle::White, 1),
            };
            st.bg_style = next_mode.0;
            win.set_bg_mode(next_mode.1);
            drop(st);

            {
                let mut cfg = config.borrow_mut();
                cfg.bg_mode = next_mode.1 as u8;
                let _ = cfg.save();
            }

            update_view();
        });
    }

    // Toggle theme (Dark Graphite Grey <-> Clean Light Mode)
    {
        let window_weak = window.as_weak();
        let config = Rc::clone(&config);
        let update_view = update_view.clone();
        window.on_toggle_theme(move || {
            let Some(win) = window_weak.upgrade() else {
                return;
            };
            let current = win.get_is_light_mode();
            let new_mode = !current;
            win.set_is_light_mode(new_mode);

            {
                let mut cfg = config.borrow_mut();
                cfg.is_light_mode = new_mode;
                let _ = cfg.save();
            }

            update_view();
        });
    }

    // Zoom controls: Zoom In (+)
    {
        let window_weak = window.as_weak();
        let engine = Rc::clone(&engine);
        let state = Rc::clone(&state);
        let update_view = update_view.clone();
        window.on_zoom_in_clicked(move || {
            let Some(win) = window_weak.upgrade() else {
                return;
            };
            let cw = win.get_canvas_width();
            let ch = win.get_canvas_height();
            if cw <= 0.0 || ch <= 0.0 || !cw.is_finite() || !ch.is_finite() {
                return;
            }
            let eng = engine.borrow();
            let Some(doc) = eng.current_document() else {
                return;
            };
            let svg_w = doc.metadata.width;
            let svg_h = doc.metadata.height;
            if svg_w <= 0.0 || svg_h <= 0.0 || !svg_w.is_finite() || !svg_h.is_finite() {
                return;
            }
            let mut st = state.borrow_mut();

            let mx = cw / 2.0;
            let my = ch / 2.0;

            let old_zoom = if st.zoom.is_finite() && st.zoom > 0.0 {
                st.zoom
            } else {
                1.0
            };
            let old_pan_x = if st.pan_x.is_finite() { st.pan_x } else { 0.0 };
            let old_pan_y = if st.pan_y.is_finite() { st.pan_y } else { 0.0 };

            let new_zoom = (old_zoom * 1.25).clamp(0.01, 500.0);

            let old_top_left_x = (cw - svg_w * old_zoom) / 2.0 + old_pan_x;
            let old_top_left_y = (ch - svg_h * old_zoom) / 2.0 + old_pan_y;

            let svg_pt_x = (mx - old_top_left_x) / old_zoom;
            let svg_pt_y = (my - old_top_left_y) / old_zoom;

            let new_top_left_x = mx - svg_pt_x * new_zoom;
            let new_top_left_y = my - svg_pt_y * new_zoom;

            let new_pan_x = new_top_left_x - (cw - svg_w * new_zoom) / 2.0;
            let new_pan_y = new_top_left_y - (ch - svg_h * new_zoom) / 2.0;

            st.pan_x = if new_pan_x.is_finite() {
                new_pan_x
            } else {
                0.0
            };
            st.pan_y = if new_pan_y.is_finite() {
                new_pan_y
            } else {
                0.0
            };
            st.zoom = new_zoom;

            drop(st);
            drop(eng);
            update_view();
        });
    }

    // Zoom controls: Zoom Out (-)
    {
        let window_weak = window.as_weak();
        let engine = Rc::clone(&engine);
        let state = Rc::clone(&state);
        let update_view = update_view.clone();
        window.on_zoom_out_clicked(move || {
            let Some(win) = window_weak.upgrade() else {
                return;
            };
            let cw = win.get_canvas_width();
            let ch = win.get_canvas_height();
            if cw <= 0.0 || ch <= 0.0 || !cw.is_finite() || !ch.is_finite() {
                return;
            }
            let eng = engine.borrow();
            let Some(doc) = eng.current_document() else {
                return;
            };
            let svg_w = doc.metadata.width;
            let svg_h = doc.metadata.height;
            if svg_w <= 0.0 || svg_h <= 0.0 || !svg_w.is_finite() || !svg_h.is_finite() {
                return;
            }
            let mut st = state.borrow_mut();

            let mx = cw / 2.0;
            let my = ch / 2.0;

            let old_zoom = if st.zoom.is_finite() && st.zoom > 0.0 {
                st.zoom
            } else {
                1.0
            };
            let old_pan_x = if st.pan_x.is_finite() { st.pan_x } else { 0.0 };
            let old_pan_y = if st.pan_y.is_finite() { st.pan_y } else { 0.0 };

            let new_zoom = (old_zoom / 1.25).clamp(0.01, 500.0);

            let old_top_left_x = (cw - svg_w * old_zoom) / 2.0 + old_pan_x;
            let old_top_left_y = (ch - svg_h * old_zoom) / 2.0 + old_pan_y;

            let svg_pt_x = (mx - old_top_left_x) / old_zoom;
            let svg_pt_y = (my - old_top_left_y) / old_zoom;

            let new_top_left_x = mx - svg_pt_x * new_zoom;
            let new_top_left_y = my - svg_pt_y * new_zoom;

            let new_pan_x = new_top_left_x - (cw - svg_w * new_zoom) / 2.0;
            let new_pan_y = new_top_left_y - (ch - svg_h * new_zoom) / 2.0;

            st.pan_x = if new_pan_x.is_finite() {
                new_pan_x
            } else {
                0.0
            };
            st.pan_y = if new_pan_y.is_finite() {
                new_pan_y
            } else {
                0.0
            };
            st.zoom = new_zoom;

            drop(st);
            drop(eng);
            update_view();
        });
    }

    // Fit to Window
    {
        let window_weak = window.as_weak();
        let engine = Rc::clone(&engine);
        let state = Rc::clone(&state);
        let update_view = update_view.clone();
        window.on_zoom_fit_clicked(move || {
            let Some(win) = window_weak.upgrade() else {
                return;
            };
            let eng = engine.borrow();
            let Some(doc) = eng.current_document() else {
                return;
            };

            let canvas_w = win.get_canvas_width();
            let canvas_h = win.get_canvas_height();

            state.borrow_mut().fit_to_window(
                canvas_w,
                canvas_h,
                doc.metadata.width,
                doc.metadata.height,
            );
            update_view();
        });
    }

    // 100% Actual Size
    {
        let state = Rc::clone(&state);
        let update_view = update_view.clone();
        window.on_zoom_100_clicked(move || {
            let mut st = state.borrow_mut();
            st.set_zoom(1.0);
            st.reset_pan();
            drop(st);
            update_view();
        });
    }

    // Toggle fullscreen
    {
        let window_weak = window.as_weak();
        window.on_toggle_fullscreen_clicked(move || {
            if let Some(win) = window_weak.upgrade() {
                let is_full = win.window().is_fullscreen();
                win.window().set_fullscreen(!is_full);
                win.set_is_fullscreen(!is_full);
            }
        });
    }

    // Escape handling
    {
        let window_weak = window.as_weak();
        window.on_handle_escape(move || {
            if let Some(win) = window_weak.upgrade() {
                if win.window().is_fullscreen() {
                    win.window().set_fullscreen(false);
                    win.set_is_fullscreen(false);
                }
            }
        });
    }

    // Canvas Window Resized
    {
        let update_view = update_view.clone();
        let window_weak = window.as_weak();
        window.on_canvas_resized(move || {
            let Some(win) = window_weak.upgrade() else {
                return;
            };
            if win.get_has_document() {
                update_view();
            } else {
                let sf = win.window().scale_factor();
                let tw = (500.0 * sf).round() as u32;
                let th = ((tw as f32) * (794.0 / 3290.0)).round() as u32;
                if let Some(b_img) = svg::create_lanczos_scaled_image(BANNER_BYTES, tw, th) {
                    win.set_banner_image(b_img);
                }
            }
        });
    }

    // Canvas Mouse Wheel Zoom: Vector zooming centered precisely at cursor!
    {
        let window_weak = window.as_weak();
        let engine = Rc::clone(&engine);
        let state = Rc::clone(&state);
        let update_view = update_view.clone();

        window.on_canvas_wheel(move |delta_y, mouse_x, mouse_y| {
            if delta_y == 0.0 {
                return;
            }
            let Some(win) = window_weak.upgrade() else {
                return;
            };
            let eng = engine.borrow();
            let Some(doc) = eng.current_document() else {
                return;
            };

            let mut st = state.borrow_mut();

            let canvas_w = win.get_canvas_width();
            let canvas_h = win.get_canvas_height();
            let svg_w = doc.metadata.width;
            let svg_h = doc.metadata.height;

            if canvas_w <= 0.0 || canvas_h <= 0.0 || svg_w <= 0.0 || svg_h <= 0.0 {
                return;
            }

            let factor = if delta_y > 0.0 { 1.2 } else { 1.0 / 1.2 };
            let old_zoom = if st.zoom.is_finite() && st.zoom > 0.0 {
                st.zoom
            } else {
                1.0
            };
            let new_zoom = (old_zoom * factor).clamp(0.01, 500.0);

            if (new_zoom - old_zoom).abs() < 1e-5 {
                return;
            }

            let old_pan_x = if st.pan_x.is_finite() { st.pan_x } else { 0.0 };
            let old_pan_y = if st.pan_y.is_finite() { st.pan_y } else { 0.0 };

            let old_top_left_x = (canvas_w - svg_w * old_zoom) / 2.0 + old_pan_x;
            let old_top_left_y = (canvas_h - svg_h * old_zoom) / 2.0 + old_pan_y;

            let svg_pt_x = (mouse_x - old_top_left_x) / old_zoom;
            let svg_pt_y = (mouse_y - old_top_left_y) / old_zoom;

            let new_top_left_x = mouse_x - svg_pt_x * new_zoom;
            let new_top_left_y = mouse_y - svg_pt_y * new_zoom;

            let new_pan_x = new_top_left_x - (canvas_w - svg_w * new_zoom) / 2.0;
            let new_pan_y = new_top_left_y - (canvas_h - svg_h * new_zoom) / 2.0;

            st.pan_x = if new_pan_x.is_finite() {
                new_pan_x
            } else {
                0.0
            };
            st.pan_y = if new_pan_y.is_finite() {
                new_pan_y
            } else {
                0.0
            };
            st.zoom = new_zoom;

            drop(st);
            drop(eng);
            update_view();
        });
    }

    // Canvas Pan: 1:1 cursor locking with zero drift or acceleration!
    {
        let state = Rc::clone(&state);
        let drag = Rc::clone(&drag_start);

        window.on_pan_started(move |mx, my| {
            let st = state.borrow();
            let px = if st.pan_x.is_finite() { st.pan_x } else { 0.0 };
            let py = if st.pan_y.is_finite() { st.pan_y } else { 0.0 };
            *drag.borrow_mut() = Some((mx, my, px, py));
        });
    }

    {
        let state = Rc::clone(&state);
        let drag = Rc::clone(&drag_start);
        let update_view = update_view.clone();

        window.on_pan_moved(move |mx, my| {
            let d = *drag.borrow();
            if let Some((start_mx, start_my, start_pan_x, start_pan_y)) = d {
                let dx = mx - start_mx;
                let dy = my - start_my;

                let mut st = state.borrow_mut();
                st.pan_x = start_pan_x + dx;
                st.pan_y = start_pan_y + dy;
                drop(st);

                update_view();
            }
        });
    }

    {
        let drag = Rc::clone(&drag_start);
        window.on_pan_ended(move || {
            *drag.borrow_mut() = None;
        });
    }

    // Drag-and-drop file support
    {
        let load_file = load_file.clone();
        window.on_file_dropped(move |data| {
            if let Ok(paths) = data.file_paths() {
                for p in paths {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if ext_lower == "svg" || ext_lower == "svgz" {
                            load_file(p.to_path_buf());
                            break;
                        }
                    }
                }
            }
        });
    }

    // Open file from command line if supplied
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let p = PathBuf::from(&args[1]);
        if p.is_file() {
            let load_file = load_file.clone();
            slint::Timer::single_shot(std::time::Duration::from_millis(50), move || {
                load_file(p);
            });
        }
    }

    window.run()
}
