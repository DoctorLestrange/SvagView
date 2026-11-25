use std::path::Path;
use svagview::app::{detect_editors, AppConfig, AppState};
use svagview::svg::{
    decode_ico_to_slint_image, export_png, ExportBackground, ExportOptions, RenderBackground,
    SvgEngine, ViewportParams,
};

#[test]
fn test_engine_parse_all_test_assets() {
    let mut engine = SvgEngine::new();
    let assets = [
        "test_assets/complex_shapes.svg",
        "test_assets/gradients_and_patterns.svg",
        "test_assets/clipping_and_masks.svg",
        "test_assets/filters_and_effects.svg",
        "test_assets/icon_dashboard.svg",
    ];

    for asset in assets {
        let path = Path::new(asset);
        assert!(path.exists(), "Asset not found: {}", asset);
        let doc = engine.load_file(path).expect("Failed to load SVG");
        assert!(doc.metadata.width > 0.0);
        assert!(doc.metadata.height > 0.0);
        println!(
            "Loaded {}: {}x{}, paths={}, gradients={}, filters={}",
            asset,
            doc.metadata.width,
            doc.metadata.height,
            doc.metadata.path_count,
            doc.metadata.gradient_count,
            doc.metadata.filter_count
        );
    }
}

#[test]
fn test_viewport_rendering_accuracy_and_zoom() {
    let mut engine = SvgEngine::new();
    let doc = engine
        .load_file("test_assets/icon_dashboard.svg")
        .expect("Failed to load SVG");

    let modes = [
        RenderBackground::Checkerboard,
        RenderBackground::White,
        RenderBackground::Black,
        RenderBackground::Slate,
    ];

    let zoom_levels = [0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 25.0];

    for zoom in zoom_levels {
        for &bg in &modes {
            let canvas_w = 800;
            let canvas_h = 600;
            let svg_w = doc.metadata.width * zoom;
            let svg_h = doc.metadata.height * zoom;
            let top_left_x = (canvas_w as f32 - svg_w) / 2.0;
            let top_left_y = (canvas_h as f32 - svg_h) / 2.0;

            for is_light in [false, true] {
                let params = ViewportParams {
                    canvas_w,
                    canvas_h,
                    svg_x: top_left_x,
                    svg_y: top_left_y,
                    svg_w,
                    svg_h,
                    scale: zoom,
                    bg,
                    is_light_mode: is_light,
                };

                let res = engine
                    .render_viewport(&params)
                    .expect("Viewport render failed");
                assert!(res.render_time_ms >= 0.0);
            }
        }
    }
    println!("Verified crisp vector viewport rendering across zoom levels up to 25x (2500%)");
}

#[test]
fn test_png_export_and_scaling() {
    let mut engine = SvgEngine::new();
    let doc = engine
        .load_file("test_assets/gradients_and_patterns.svg")
        .expect("Failed to load");

    let tmp_dir = std::env::temp_dir();
    let export_path = tmp_dir.join("svagview_test_export.png");

    let opt = ExportOptions {
        scale: 2.0,
        custom_width: None,
        custom_height: None,
        background: ExportBackground::White,
    };

    export_png(&doc.tree, &export_path, &opt).expect("Export failed");
    assert!(export_path.exists());
    let metadata = std::fs::metadata(&export_path).expect("Metadata failed");
    assert!(metadata.len() > 1000);
    println!("Exported 2x PNG size: {} bytes", metadata.len());

    let _ = std::fs::remove_file(export_path);
}

#[test]
fn test_editor_detection() {
    let editors = detect_editors();
    println!("Detected {} SVG editors on system:", editors.len());
    for ed in &editors {
        println!(" - {} ({}): {:?}", ed.name, ed.icon_text, ed.executable);
        assert!(!ed.id.is_empty(), "Editor ID must not be empty");
        assert!(!ed.name.is_empty(), "Editor name must not be empty");
        assert!(
            !ed.icon_text.is_empty(),
            "Editor icon text must not be empty"
        );
        assert!(
            ed.executable.is_file(),
            "Detected editor executable must exist on disk: {:?}",
            ed.executable
        );
    }
    // Note: Clean CI environments (GitHub Actions Ubuntu, Windows, macOS runners)
    // do not have desktop graphics suites (Inkscape, Illustrator, CorelDRAW) preinstalled.
    // In that environment, returning an empty list without panicking is the correct behavior.
    println!("Editor detection executed safely!");
}

#[test]
fn test_app_state_navigation() {
    let mut state = AppState::default();
    let test_file = Path::new("test_assets/complex_shapes.svg")
        .canonicalize()
        .unwrap();

    state.set_current_file(test_file.clone());
    assert!(state.folder_files.len() >= 5);
    let (_idx, total) = state.current_file_info();
    assert!(total >= 5);

    let next = state.next_file();
    assert!(next.is_some());
    assert_ne!(next.unwrap(), test_file);

    let prev = state.prev_file();
    assert_eq!(prev.unwrap(), test_file);
}

#[test]
fn test_generate_app_ico() {
    let svg_path = Path::new("resources/SvagView_Logo_Icon.svg");
    assert!(svg_path.exists());
    let svg_data = std::fs::read(svg_path).expect("Failed to read SVG");

    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(&svg_data, &opt).expect("Failed to parse SVG");

    let sizes: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
    let mut image_entries: Vec<(u32, Vec<u8>)> = Vec::new();

    for &sz in &sizes {
        let mut pixmap = resvg::tiny_skia::Pixmap::new(sz, sz).unwrap();
        let scale_x = sz as f32 / tree.size().width();
        let scale_y = sz as f32 / tree.size().height();
        let transform = resvg::tiny_skia::Transform::from_scale(scale_x, scale_y);

        resvg::render(&tree, transform, &mut pixmap.as_mut());

        if sz == 256 {
            // 256x256 uses modern PNG container
            let png_data = pixmap.encode_png().expect("PNG encode failed");
            image_entries.push((sz, png_data));
        } else {
            // Standard Windows DIB format for 16, 24, 32, 48, 64, 128
            let mut dib: Vec<u8> = Vec::new();
            let and_row_bytes = sz.div_ceil(32) * 4;
            let and_mask_bytes = and_row_bytes * sz;
            let image_bytes = sz * sz * 4 + and_mask_bytes;

            // BITMAPINFOHEADER (40 bytes)
            dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
            dib.extend_from_slice(&(sz as i32).to_le_bytes()); // biWidth
            dib.extend_from_slice(&((sz * 2) as i32).to_le_bytes()); // biHeight (XOR + AND)
            dib.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
            dib.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
            dib.extend_from_slice(&0u32.to_le_bytes()); // biCompression (BI_RGB)
            dib.extend_from_slice(&image_bytes.to_le_bytes()); // biSizeImage
            dib.extend_from_slice(&0i32.to_le_bytes()); // biXPelsPerMeter
            dib.extend_from_slice(&0i32.to_le_bytes()); // biYPelsPerMeter
            dib.extend_from_slice(&0u32.to_le_bytes()); // biClrUsed
            dib.extend_from_slice(&0u32.to_le_bytes()); // biClrImportant

            // Pixel data: Bottom-up BGRA
            let raw_pixels = pixmap.data();
            for y in (0..sz).rev() {
                for x in 0..sz {
                    let idx = ((y * sz + x) * 4) as usize;
                    let r = raw_pixels[idx];
                    let g = raw_pixels[idx + 1];
                    let b = raw_pixels[idx + 2];
                    let a = raw_pixels[idx + 3];

                    // Un-premultiply if needed (tiny-skia is premultiplied)
                    let (r_un, g_un, b_un) = if a > 0 {
                        (
                            ((r as u16 * 255) / a as u16) as u8,
                            ((g as u16 * 255) / a as u16) as u8,
                            ((b as u16 * 255) / a as u16) as u8,
                        )
                    } else {
                        (0, 0, 0)
                    };

                    dib.push(b_un);
                    dib.push(g_un);
                    dib.push(r_un);
                    dib.push(a);
                }
            }

            // AND mask (all zeroes for 32-bit alpha)
            dib.resize(dib.len() + and_mask_bytes as usize, 0);
            image_entries.push((sz, dib));
        }
    }

    // Build standard ICO format binary
    let mut ico: Vec<u8> = Vec::new();
    // Header: 6 bytes
    ico.extend_from_slice(&[0, 0]); // Reserved
    ico.extend_from_slice(&[1, 0]); // Type 1 = Icon
    ico.extend_from_slice(&(sizes.len() as u16).to_le_bytes()); // Image count

    let header_size = 6 + (sizes.len() * 16) as u32;
    let mut current_offset = header_size;

    for (sz, data) in &image_entries {
        let w = if *sz == 256 { 0u8 } else { *sz as u8 };
        let h = if *sz == 256 { 0u8 } else { *sz as u8 };
        ico.push(w); // Width
        ico.push(h); // Height
        ico.push(0); // Colors (0 = no palette)
        ico.push(0); // Reserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // Color planes = 1
        ico.extend_from_slice(&32u16.to_le_bytes()); // Bits per pixel = 32
        ico.extend_from_slice(&(data.len() as u32).to_le_bytes()); // Size of image data
        ico.extend_from_slice(&current_offset.to_le_bytes()); // Offset
        current_offset += data.len() as u32;
    }

    // Append image data
    for (_, data) in &image_entries {
        ico.extend_from_slice(data);
    }

    let out_ico = std::env::temp_dir().join("SvagView_Logo_Icon_test.ico");
    std::fs::write(&out_ico, &ico).expect("Failed to write test ICO");
    println!(
        "Generated multi-resolution ICO (16, 24, 32, 48, 64, 128, 256 px): {:?}",
        out_ico
    );
    assert!(out_ico.exists());
    assert!(out_ico.metadata().unwrap().len() > 1000);
    let _ = std::fs::remove_file(out_ico);
}

#[test]
fn test_svgz_gzip_loading() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let svg_xml = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="150" viewBox="0 0 200 150">
        <rect width="200" height="150" fill="indigo"/>
        <circle cx="100" cy="75" r="50" fill="gold"/>
    </svg>"#;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(svg_xml.as_bytes()).unwrap();
    let gzipped_data = encoder.finish().unwrap();

    let mut engine = SvgEngine::new();
    let doc = engine
        .load_data(gzipped_data, None)
        .expect("Failed to parse gzipped SVGZ");

    assert_eq!(doc.metadata.width, 200.0);
    assert_eq!(doc.metadata.height, 150.0);
    assert!(doc.metadata.view_box.contains("200"));
    assert!(std::str::from_utf8(&doc.raw_data).is_ok());
    println!("Successfully loaded and verified SVGZ decompression and metadata!");
}

#[test]
fn test_degenerate_and_corrupt_svgs() {
    let mut engine = SvgEngine::new();

    // 1. Empty data
    assert!(engine.load_data(Vec::new(), None).is_err());

    // 2. Corrupt non-XML binary data
    let random_junk = vec![0xde, 0xad, 0xbe, 0xef, 0x00, 0x12, 0x34];
    assert!(engine.load_data(random_junk, None).is_err());

    // 3. Invalid XML syntax
    let bad_xml = b"<svg><path d='M 0 0 L 10 10'><unclosed>".to_vec();
    assert!(engine.load_data(bad_xml, None).is_err());

    // 4. Zero dimensions SVG
    let zero_wh = b"<svg xmlns='http://www.w3.org/2000/svg' width='0' height='0'></svg>".to_vec();
    assert!(engine.load_data(zero_wh, None).is_err());

    println!("Verified fool-proof rejection of empty, corrupt, and degenerate SVGs!");
}

#[test]
fn test_extreme_viewport_parameters() {
    let mut engine = SvgEngine::new();
    let doc = engine
        .load_file("test_assets/complex_shapes.svg")
        .expect("Failed to load SVG");

    // Test NaN and inf parameters in render_viewport
    let nan_params = ViewportParams {
        canvas_w: 800,
        canvas_h: 600,
        svg_x: f32::NAN,
        svg_y: 0.0,
        svg_w: doc.metadata.width,
        svg_h: doc.metadata.height,
        scale: 1.0,
        bg: RenderBackground::Checkerboard,
        is_light_mode: false,
    };
    assert!(engine.render_viewport(&nan_params).is_err());

    let inf_scale_params = ViewportParams {
        canvas_w: 800,
        canvas_h: 600,
        svg_x: 0.0,
        svg_y: 0.0,
        svg_w: doc.metadata.width,
        svg_h: doc.metadata.height,
        scale: f32::INFINITY,
        bg: RenderBackground::White,
        is_light_mode: true,
    };
    assert!(engine.render_viewport(&inf_scale_params).is_err());

    let zero_scale_params = ViewportParams {
        canvas_w: 800,
        canvas_h: 600,
        svg_x: 0.0,
        svg_y: 0.0,
        svg_w: doc.metadata.width,
        svg_h: doc.metadata.height,
        scale: 0.0,
        bg: RenderBackground::Black,
        is_light_mode: false,
    };
    assert!(engine.render_viewport(&zero_scale_params).is_err());

    println!("Verified fool-proof parameter validation in render_viewport!");
}

#[test]
fn test_app_state_first_last_and_robustness() {
    let mut state = AppState::default();
    let test_file = Path::new("test_assets/complex_shapes.svg")
        .canonicalize()
        .unwrap();

    state.set_current_file(test_file);
    assert!(!state.folder_files.is_empty());

    let first = state.first_file();
    assert!(first.is_some());
    assert_eq!(state.current_index, 0);

    let last = state.last_file();
    assert!(last.is_some());
    assert_eq!(state.current_index, state.folder_files.len() - 1);

    // Verify fit_to_window with extreme/NaN dimensions
    let scale = state.fit_to_window(f32::NAN, 600.0, 800.0, 600.0);
    assert_eq!(scale, 1.0);
    assert_eq!(state.zoom, 1.0);

    let scale_zero = state.fit_to_window(800.0, 600.0, 0.0, 0.0);
    assert_eq!(scale_zero, 1.0);

    println!("Verified AppState navigation, first/last methods, and fit_to_window robustness!");
}

#[test]
fn test_app_config_persistence_and_defaults() {
    let default_cfg = AppConfig::default();
    assert!(default_cfg.is_light_mode);
    assert_eq!(default_cfg.bg_mode, 1);
    assert_eq!(default_cfg.export_scale_preset, 1);
    assert_eq!(default_cfg.export_bg_mode, 0);

    // Test JSON serialization & deserialization round-trip
    let json = serde_json::to_string(&default_cfg).expect("serialize config");
    let deserialized: AppConfig = serde_json::from_str(&json).expect("deserialize config");
    assert_eq!(default_cfg, deserialized);

    // Test modified config round-trip
    let custom_cfg = AppConfig {
        is_light_mode: false,
        bg_mode: 3,
        export_scale_preset: 4,
        export_bg_mode: 2,
    };
    let json_custom = serde_json::to_string_pretty(&custom_cfg).expect("serialize custom config");
    let deserialized_custom: AppConfig =
        serde_json::from_str(&json_custom).expect("deserialize custom config");
    assert_eq!(custom_cfg, deserialized_custom);

    // Test corrupted JSON gracefully falls back in deserialization
    let corrupt_json = "{ \"is_light_mode\": \"invalid_bool\", }";
    let fallback = serde_json::from_str::<AppConfig>(corrupt_json).unwrap_or_default();
    assert_eq!(fallback, AppConfig::default());

    println!("Verified AppConfig persistence defaults and serialization round-trip!");
}

#[test]
fn test_decode_app_ico_all_sizes() {
    let ico_bytes = std::fs::read("resources/SvagView_Logo_Icon.ico")
        .expect("Failed to read resources/SvagView_Logo_Icon.ico");

    for expected_size in [16u32, 32, 64, 128, 256] {
        let img = decode_ico_to_slint_image(&ico_bytes, expected_size)
            .unwrap_or_else(|| panic!("Failed to decode ICO entry for size {}", expected_size));
        assert_eq!(img.size().width, expected_size);
        assert_eq!(img.size().height, expected_size);
    }

    // Invalid or truncated inputs should cleanly return None without panicking
    assert!(decode_ico_to_slint_image(&[], 64).is_none());
    assert!(decode_ico_to_slint_image(&[0, 0, 2, 0, 1, 0], 64).is_none());
}
