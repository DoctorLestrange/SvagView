use arboard::{Clipboard, ImageData};
use resvg::tiny_skia::Pixmap;
use std::borrow::Cow;

pub fn copy_image_to_clipboard(pixmap: &Pixmap) -> Result<(), String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;

    // Convert tiny-skia's premultiplied RGBA into standard straight RGBA
    // to prevent dark halos and fringing when pasted into external editors/browsers
    let mut straight_rgba = Vec::with_capacity(width * height * 4);
    for p in pixmap.pixels() {
        let a = p.alpha();
        if a > 0 {
            let r = ((p.red() as u16 * 255) / a as u16).min(255) as u8;
            let g = ((p.green() as u16 * 255) / a as u16).min(255) as u8;
            let b = ((p.blue() as u16 * 255) / a as u16).min(255) as u8;
            straight_rgba.extend_from_slice(&[r, g, b, a]);
        } else {
            straight_rgba.extend_from_slice(&[0, 0, 0, 0]);
        }
    }

    let img_data = ImageData {
        width,
        height,
        bytes: Cow::Owned(straight_rgba),
    };
    clipboard
        .set_image(img_data)
        .map_err(|e| format!("Failed to copy image to clipboard: {}", e))?;
    Ok(())
}

pub fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to copy text to clipboard: {}", e))?;
    Ok(())
}
