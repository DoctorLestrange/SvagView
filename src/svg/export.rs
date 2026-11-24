use resvg::tiny_skia::{Color, Pixmap, Transform};
use resvg::usvg::Tree;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum ExportBackground {
    Transparent,
    White,
    Black,
    Custom(u8, u8, u8, u8),
}

pub struct ExportOptions {
    pub scale: f32,
    pub custom_width: Option<u32>,
    pub custom_height: Option<u32>,
    pub background: ExportBackground,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            custom_width: None,
            custom_height: None,
            background: ExportBackground::Transparent,
        }
    }
}

pub fn render_for_export(tree: &Tree, options: &ExportOptions) -> Result<Pixmap, String> {
    let svg_size = tree.size();
    let sw = svg_size.width();
    let sh = svg_size.height();

    if sw <= 0.0 || sh <= 0.0 || !sw.is_finite() || !sh.is_finite() {
        return Err("SVG document has zero or invalid dimensions".to_string());
    }

    let scale = if options.scale.is_finite() && options.scale > 0.0 {
        options.scale
    } else {
        1.0
    };

    let (target_w, target_h) = match (options.custom_width, options.custom_height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            let h = ((w as f32 / sw) * sh).round() as u32;
            (w, h.max(1))
        }
        (None, Some(h)) => {
            let w = ((h as f32 / sh) * sw).round() as u32;
            (w.max(1), h)
        }
        (None, None) => {
            let w = (sw * scale).round() as u32;
            let h = (sh * scale).round() as u32;
            (w.max(1), h.max(1))
        }
    };

    if target_w == 0 || target_h == 0 {
        return Err("Export dimensions must be greater than 0".to_string());
    }

    if target_w > 16384 || target_h > 16384 {
        return Err(
            "Export dimensions exceed maximum allowable resolution (16384x16384)".to_string(),
        );
    }

    let mut pixmap = Pixmap::new(target_w, target_h).ok_or_else(|| {
        format!(
            "Failed to allocate export buffer ({}x{})",
            target_w, target_h
        )
    })?;

    match options.background {
        ExportBackground::Transparent => {}
        ExportBackground::White => {
            pixmap.fill(Color::WHITE);
        }
        ExportBackground::Black => {
            pixmap.fill(Color::BLACK);
        }
        ExportBackground::Custom(r, g, b, a) => {
            let col = Color::from_rgba8(r, g, b, a);
            pixmap.fill(col);
        }
    }

    let sx = target_w as f32 / svg_size.width();
    let sy = target_h as f32 / svg_size.height();
    let transform = Transform::from_scale(sx, sy);

    resvg::render(tree, transform, &mut pixmap.as_mut());

    Ok(pixmap)
}

pub fn export_png<P: AsRef<Path>>(
    tree: &Tree,
    dest_path: P,
    options: &ExportOptions,
) -> Result<(), String> {
    let pixmap = render_for_export(tree, options)?;
    let png_bytes = pixmap
        .encode_png()
        .map_err(|e| format!("Failed to encode PNG: {:?}", e))?;

    let path = dest_path.as_ref();
    let mut file = File::create(path)
        .map_err(|e| format!("Failed to create file '{}': {}", path.display(), e))?;

    file.write_all(&png_bytes)
        .map_err(|e| format!("Failed to write PNG data: {}", e))?;

    Ok(())
}
