use resvg::tiny_skia::{
    Color, Paint, PathBuilder, Pixmap, PremultipliedColorU8, Rect, Stroke, Transform,
};
use resvg::usvg::{self, fontdb, Tree};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use super::metadata::SvgMetadata;

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum RenderBackground {
    Transparent,
    Checkerboard,
    White,
    Black,
    Slate,
    Custom(u8, u8, u8, u8),
}

pub struct ViewportParams {
    pub canvas_w: u32,
    pub canvas_h: u32,
    pub svg_x: f32, // artboard top-left in physical pixels
    pub svg_y: f32, // artboard top-left in physical pixels
    pub svg_w: f32, // artboard width in physical pixels
    pub svg_h: f32, // artboard height in physical pixels
    pub scale: f32, // scale factor from SVG units to physical pixels
    pub bg: RenderBackground,
    pub is_light_mode: bool,
}

pub struct ViewportRenderResult {
    pub image: Image,
    pub render_time_ms: f32,
}

pub struct SvgDocument {
    pub path: Option<PathBuf>,
    pub raw_data: Vec<u8>,
    pub tree: Arc<Tree>,
    pub metadata: SvgMetadata,
    pub parse_time_ms: f32,
}

pub struct SvgEngine {
    fontdb: Arc<fontdb::Database>,
    current_doc: Option<Arc<SvgDocument>>,
}

impl SvgEngine {
    pub fn new() -> Self {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let fontdb = Arc::new(db);

        Self {
            fontdb,
            current_doc: None,
        }
    }

    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Arc<SvgDocument>, String> {
        let p = path.as_ref();
        let data = std::fs::read(p)
            .map_err(|e| format!("Failed to read file '{}': {}", p.display(), e))?;

        self.load_data(data, Some(p.to_path_buf()))
    }

    pub fn load_data(
        &mut self,
        raw_data: Vec<u8>,
        path: Option<PathBuf>,
    ) -> Result<Arc<SvgDocument>, String> {
        let start = Instant::now();

        // Support transparent decompression for SVGZ (gzip-compressed SVG)
        let mut final_data = raw_data;
        if final_data.starts_with(&[0x1f, 0x8b]) {
            use flate2::read::GzDecoder;
            use std::io::Read;
            let mut decoder = GzDecoder::new(&final_data[..]);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() && !decompressed.is_empty() {
                final_data = decompressed;
            }
        }

        let opt = usvg::Options {
            fontdb: Arc::clone(&self.fontdb),
            font_family: "sans-serif".to_string(),
            ..Default::default()
        };

        let tree = Tree::from_data(&final_data, &opt)
            .map_err(|e| format!("Failed to parse SVG: {:?}", e))?;

        let parse_time_ms = start.elapsed().as_secs_f32() * 1000.0;
        let metadata = SvgMetadata::from_tree_and_file(&tree, &final_data, path.as_deref());

        let doc = Arc::new(SvgDocument {
            path,
            raw_data: final_data,
            tree: Arc::new(tree),
            metadata,
            parse_time_ms,
        });

        self.current_doc = Some(Arc::clone(&doc));
        Ok(doc)
    }

    pub fn current_document(&self) -> Option<Arc<SvgDocument>> {
        self.current_doc.clone()
    }

    /// Renders the SVG directly into the screen's canvas viewport with crisp sub-pixel vector accuracy
    pub fn render_viewport(&self, params: &ViewportParams) -> Result<ViewportRenderResult, String> {
        let doc = self
            .current_doc
            .as_ref()
            .ok_or_else(|| "No document loaded to render".to_string())?;

        if !params.scale.is_finite() || params.scale <= 0.0 {
            return Err("Invalid render scale".to_string());
        }
        if !params.svg_x.is_finite()
            || !params.svg_y.is_finite()
            || !params.svg_w.is_finite()
            || !params.svg_h.is_finite()
        {
            return Err("Invalid artboard coordinates".to_string());
        }

        let width = params.canvas_w.clamp(1, 8192);
        let height = params.canvas_h.clamp(1, 8192);

        let start = Instant::now();

        let mut pixmap = Pixmap::new(width, height)
            .ok_or_else(|| format!("Failed to allocate viewport buffer ({}x{})", width, height))?;

        // 1. Fill surrounding workspace canvas (Graphite dark or Clean light)
        if params.is_light_mode {
            pixmap.fill(Color::from_rgba8(234, 234, 238, 255));
        } else {
            // Dark graphite grey workspace
            pixmap.fill(Color::from_rgba8(22, 22, 25, 255));
        }

        // 2. Draw SVG artboard page background
        if params.svg_w > 0.0 && params.svg_h > 0.0 {
            let art_rect_opt =
                Rect::from_xywh(params.svg_x, params.svg_y, params.svg_w, params.svg_h);

            match params.bg {
                RenderBackground::Checkerboard => {
                    fill_artboard_checkerboard(
                        &mut pixmap,
                        params.svg_x,
                        params.svg_y,
                        params.svg_w,
                        params.svg_h,
                        16,
                        params.is_light_mode,
                    );
                }
                RenderBackground::White => {
                    if let Some(rect) = art_rect_opt {
                        let mut paint = Paint::default();
                        paint.set_color(Color::WHITE);
                        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                    }
                }
                RenderBackground::Black => {
                    if let Some(rect) = art_rect_opt {
                        let mut paint = Paint::default();
                        paint.set_color(Color::BLACK);
                        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                    }
                }
                RenderBackground::Slate => {
                    if let Some(rect) = art_rect_opt {
                        let mut paint = Paint::default();
                        if params.is_light_mode {
                            paint.set_color(Color::from_rgba8(220, 220, 226, 255));
                        } else {
                            paint.set_color(Color::from_rgba8(36, 36, 42, 255));
                        }
                        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                    }
                }
                RenderBackground::Custom(r, g, b, a) => {
                    if let Some(rect) = art_rect_opt {
                        let mut paint = Paint::default();
                        paint.set_color(Color::from_rgba8(r, g, b, a));
                        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                    }
                }
                RenderBackground::Transparent => {}
            }

            // Draw crisp 1px artboard border so document boundary is visible on canvas
            if let Some(rect) = art_rect_opt {
                let border_path = PathBuilder::from_rect(rect);
                let mut border_paint = Paint::default();
                if params.is_light_mode {
                    border_paint.set_color(Color::from_rgba8(200, 200, 208, 255));
                } else {
                    border_paint.set_color(Color::from_rgba8(55, 55, 64, 255));
                }
                let stroke = Stroke {
                    width: 1.0,
                    ..Default::default()
                };
                pixmap.stroke_path(
                    &border_path,
                    &border_paint,
                    &stroke,
                    Transform::identity(),
                    None,
                );
            }
        }

        // 3. Render vector shapes directly at screen resolution!
        let transform = Transform::from_scale(params.scale, params.scale)
            .post_translate(params.svg_x, params.svg_y);

        resvg::render(&doc.tree, transform, &mut pixmap.as_mut());

        let render_time_ms = start.elapsed().as_secs_f32() * 1000.0;

        let pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            pixmap.data(),
            pixmap.width(),
            pixmap.height(),
        );
        let image = Image::from_rgba8(pixel_buffer);

        Ok(ViewportRenderResult {
            image,
            render_time_ms,
        })
    }
}

impl Default for SvgEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn fill_artboard_checkerboard(
    pixmap: &mut Pixmap,
    art_x: f32,
    art_y: f32,
    art_w: f32,
    art_h: f32,
    square_size: u32,
    is_light_mode: bool,
) {
    if !art_x.is_finite() || !art_y.is_finite() || !art_w.is_finite() || !art_h.is_finite() {
        return;
    }

    let pw = pixmap.width() as i32;
    let ph = pixmap.height() as i32;

    let min_x = (art_x.round() as i32).max(0);
    let min_y = (art_y.round() as i32).max(0);
    let max_x = ((art_x + art_w).round() as i32).min(pw);
    let max_y = ((art_y + art_h).round() as i32).min(ph);

    if min_x >= max_x || min_y >= max_y {
        return;
    }

    let pixels = pixmap.pixels_mut();
    let (c1, c2) = if is_light_mode {
        (
            PremultipliedColorU8::from_rgba(255, 255, 255, 255).unwrap(),
            PremultipliedColorU8::from_rgba(230, 230, 236, 255).unwrap(),
        )
    } else {
        (
            PremultipliedColorU8::from_rgba(42, 42, 48, 255).unwrap(),
            PremultipliedColorU8::from_rgba(26, 26, 30, 255).unwrap(),
        )
    };

    let sq = square_size.max(4) as i32;
    let ox = art_x.round() as i32;
    let oy = art_y.round() as i32;

    for py in min_y..max_y {
        let row_idx = (((py - oy).abs()) / sq) % 2;
        let row_start = (py * pw) as usize;
        for px in min_x..max_x {
            let col_idx = (((px - ox).abs()) / sq) % 2;
            pixels[row_start + px as usize] = if row_idx == col_idx { c1 } else { c2 };
        }
    }
}
