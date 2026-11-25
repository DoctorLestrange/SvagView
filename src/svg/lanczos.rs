//! High-quality Lanczos3 image downscaling resampler.
//!
//! Evaluates true sinc-windowed continuous filter over the scaled kernel footprint
//! to produce pristine, artifact-free, anti-aliased downscaled images.

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

fn sinc(x: f32) -> f32 {
    let ax = x.abs();
    if ax < 1e-5 {
        1.0
    } else {
        let px = std::f32::consts::PI * x;
        px.sin() / px
    }
}

fn lanczos3(x: f32) -> f32 {
    let ax = x.abs();
    if ax < 3.0 {
        sinc(x) * sinc(x / 3.0)
    } else {
        0.0
    }
}

/// Downscale an RGBA image from (src_w, src_h) to (dst_w, dst_h) using 2-pass separable Lanczos3.
pub fn lanczos3_downscale(
    src_pixels: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> Vec<u8> {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return Vec::new();
    }
    let expected = (src_w as usize)
        .saturating_mul(src_h as usize)
        .saturating_mul(4);
    if src_pixels.len() < expected {
        return Vec::new();
    }
    if src_w == dst_w && src_h == dst_h {
        return src_pixels.to_vec();
    }

    let scale_x = dst_w as f32 / src_w as f32;
    let scale_y = dst_h as f32 / src_h as f32;

    // Pass 1: Horizontal resample (src_w, src_h) -> (dst_w, src_h)
    let h_weights = compute_filter_weights(src_w, dst_w, scale_x);
    let mut intermediate = vec![0u8; (dst_w * src_h * 4) as usize];

    for y in 0..src_h {
        let src_row_start = (y * src_w * 4) as usize;
        let dst_row_start = (y * dst_w * 4) as usize;

        for dx in 0..dst_w {
            let (ref samples, ref weights) = h_weights[dx as usize];
            let mut r = 0.0f32;
            let mut g = 0.0f32;
            let mut b = 0.0f32;
            let mut a = 0.0f32;

            for (&sx, &w) in samples.iter().zip(weights.iter()) {
                let px_idx = src_row_start + (sx * 4);
                r += src_pixels[px_idx] as f32 * w;
                g += src_pixels[px_idx + 1] as f32 * w;
                b += src_pixels[px_idx + 2] as f32 * w;
                a += src_pixels[px_idx + 3] as f32 * w;
            }

            let out_idx = dst_row_start + (dx as usize * 4);
            intermediate[out_idx] = r.clamp(0.0, 255.0).round() as u8;
            intermediate[out_idx + 1] = g.clamp(0.0, 255.0).round() as u8;
            intermediate[out_idx + 2] = b.clamp(0.0, 255.0).round() as u8;
            intermediate[out_idx + 3] = a.clamp(0.0, 255.0).round() as u8;
        }
    }

    // Pass 2: Vertical resample (dst_w, src_h) -> (dst_w, dst_h)
    let v_weights = compute_filter_weights(src_h, dst_h, scale_y);
    let mut output = vec![0u8; (dst_w * dst_h * 4) as usize];

    for dy in 0..dst_h {
        let (ref samples, ref weights) = v_weights[dy as usize];
        let dst_row_start = (dy * dst_w * 4) as usize;

        for x in 0..dst_w {
            let mut r = 0.0f32;
            let mut g = 0.0f32;
            let mut b = 0.0f32;
            let mut a = 0.0f32;

            for (&sy, &w) in samples.iter().zip(weights.iter()) {
                let px_idx = (sy * (dst_w as usize) * 4) + (x as usize * 4);
                r += intermediate[px_idx] as f32 * w;
                g += intermediate[px_idx + 1] as f32 * w;
                b += intermediate[px_idx + 2] as f32 * w;
                a += intermediate[px_idx + 3] as f32 * w;
            }

            let out_idx = dst_row_start + (x as usize * 4);
            output[out_idx] = r.clamp(0.0, 255.0).round() as u8;
            output[out_idx + 1] = g.clamp(0.0, 255.0).round() as u8;
            output[out_idx + 2] = b.clamp(0.0, 255.0).round() as u8;
            output[out_idx + 3] = a.clamp(0.0, 255.0).round() as u8;
        }
    }

    output
}

fn compute_filter_weights(src_len: u32, dst_len: u32, scale: f32) -> Vec<(Vec<usize>, Vec<f32>)> {
    let mut result = Vec::with_capacity(dst_len as usize);
    let (filter_scale, radius) = if scale < 1.0 {
        (scale, 3.0 / scale)
    } else {
        (1.0, 3.0)
    };

    for d in 0..dst_len {
        let center = (d as f32 + 0.5) / scale - 0.5;
        let min_idx = ((center - radius).floor().max(0.0)) as usize;
        let max_idx = ((center + radius).ceil().min(src_len as f32 - 1.0)) as usize;

        let mut samples = Vec::new();
        let mut weights = Vec::new();
        let mut sum = 0.0f32;

        for s in min_idx..=max_idx {
            let dist = (s as f32 - center) * filter_scale;
            let w = lanczos3(dist) * filter_scale;
            if w.abs() > 1e-6 {
                samples.push(s);
                weights.push(w);
                sum += w;
            }
        }

        if samples.is_empty() {
            let fallback_idx = (center.round() as usize).clamp(0, src_len as usize - 1);
            samples.push(fallback_idx);
            weights.push(1.0);
        } else if sum.abs() > 1e-6 {
            let inv_sum = 1.0 / sum;
            for w in &mut weights {
                *w *= inv_sum;
            }
        } else {
            let even = 1.0 / samples.len() as f32;
            weights.fill(even);
        }

        result.push((samples, weights));
    }

    result
}

/// Convert a premultiplied tiny-skia Pixmap into a straight-alpha RGBA byte buffer.
fn pixmap_to_straight_rgba(pixmap: &resvg::tiny_skia::Pixmap) -> Vec<u8> {
    let mut straight_rgba = Vec::with_capacity((pixmap.width() * pixmap.height() * 4) as usize);
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
    straight_rgba
}

/// Helper: Load and Lanczos3-downscale an image into a slint::Image
pub fn create_lanczos_scaled_image(
    png_bytes: &[u8],
    target_w: u32,
    target_h: u32,
) -> Option<Image> {
    let pixmap = resvg::tiny_skia::Pixmap::decode_png(png_bytes).ok()?;
    let src_w = pixmap.width();
    let src_h = pixmap.height();
    let straight_rgba = pixmap_to_straight_rgba(&pixmap);

    let downscaled = lanczos3_downscale(&straight_rgba, src_w, src_h, target_w, target_h);

    let pixel_buffer =
        SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&downscaled, target_w, target_h);
    Some(Image::from_rgba8(pixel_buffer))
}

/// Decode an embedded `.ico` byte slice into a `slint::Image`, picking the
/// entry closest to `target_size` (preferring the smallest entry >= `target_size`,
/// otherwise the largest available entry). Supports both PNG and 32bpp DIB entries.
pub fn decode_ico_to_slint_image(ico_bytes: &[u8], target_size: u32) -> Option<Image> {
    if ico_bytes.len() < 6 {
        return None;
    }
    let reserved = u16::from_le_bytes([ico_bytes[0], ico_bytes[1]]);
    let ico_type = u16::from_le_bytes([ico_bytes[2], ico_bytes[3]]);
    let count = u16::from_le_bytes([ico_bytes[4], ico_bytes[5]]) as usize;
    if reserved != 0 || ico_type != 1 || count == 0 {
        return None;
    }
    let dir_end = 6usize.checked_add(count.checked_mul(16)?)?;
    if ico_bytes.len() < dir_end {
        return None;
    }

    #[derive(Clone, Copy)]
    struct IcoEntry {
        w: u32,
        h: u32,
        bpp: u16,
        offset: usize,
        size: usize,
    }

    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let base = 6 + i * 16;
        let w = if ico_bytes[base] == 0 {
            256
        } else {
            ico_bytes[base] as u32
        };
        let h = if ico_bytes[base + 1] == 0 {
            256
        } else {
            ico_bytes[base + 1] as u32
        };
        let bpp = u16::from_le_bytes([ico_bytes[base + 6], ico_bytes[base + 7]]);
        let size = u32::from_le_bytes([
            ico_bytes[base + 8],
            ico_bytes[base + 9],
            ico_bytes[base + 10],
            ico_bytes[base + 11],
        ]) as usize;
        let offset = u32::from_le_bytes([
            ico_bytes[base + 12],
            ico_bytes[base + 13],
            ico_bytes[base + 14],
            ico_bytes[base + 15],
        ]) as usize;

        if size > 0
            && offset
                .checked_add(size)
                .is_some_and(|end| end <= ico_bytes.len())
        {
            entries.push(IcoEntry {
                w,
                h,
                bpp,
                offset,
                size,
            });
        }
    }

    entries.sort_by(|a, b| {
        let a_ge = a.w >= target_size;
        let b_ge = b.w >= target_size;
        match (a_ge, b_ge) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (true, true) => a.w.cmp(&b.w).then_with(|| b.bpp.cmp(&a.bpp)),
            (false, false) => b.w.cmp(&a.w).then_with(|| b.bpp.cmp(&a.bpp)),
        }
    });

    for entry in entries {
        let payload = &ico_bytes[entry.offset..entry.offset + entry.size];
        if payload.starts_with(b"\x89PNG\r\n\x1a\n") {
            if let Ok(pixmap) = resvg::tiny_skia::Pixmap::decode_png(payload) {
                let rgba = pixmap_to_straight_rgba(&pixmap);
                let pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
                    &rgba,
                    pixmap.width(),
                    pixmap.height(),
                );
                return Some(Image::from_rgba8(pixel_buffer));
            }
            continue;
        }

        if payload.len() < 40 {
            continue;
        }
        let header_size =
            u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
        if header_size < 40 || header_size > payload.len() {
            continue;
        }
        let bmp_w = i32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
        let bmp_h = i32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]);
        let bit_count = u16::from_le_bytes([payload[14], payload[15]]);
        let compression = u32::from_le_bytes([payload[16], payload[17], payload[18], payload[19]]);

        if bit_count != 32 || (compression != 0 && compression != 3) {
            continue;
        }

        let pixel_offset = if compression == 3 && header_size == 40 {
            52
        } else {
            header_size
        };
        let width = if bmp_w != 0 {
            bmp_w.unsigned_abs()
        } else {
            entry.w
        };
        let height = entry.h;
        let pixel_bytes_needed = (width as usize)
            .checked_mul(height as usize)
            .and_then(|px| px.checked_mul(4));
        let Some(byte_len) = pixel_bytes_needed else {
            continue;
        };
        if pixel_offset
            .checked_add(byte_len)
            .is_none_or(|end| end > payload.len())
        {
            continue;
        }

        let pixels = &payload[pixel_offset..pixel_offset + byte_len];
        let mut rgba = Vec::with_capacity(byte_len);
        let bottom_up = bmp_h > 0;
        let row_bytes = (width as usize) * 4;
        for dst_y in 0..(height as usize) {
            let src_y = if bottom_up {
                (height as usize) - 1 - dst_y
            } else {
                dst_y
            };
            let row = &pixels[src_y * row_bytes..(src_y + 1) * row_bytes];
            for bgra in row.chunks_exact(4) {
                rgba.extend_from_slice(&[bgra[2], bgra[1], bgra[0], bgra[3]]);
            }
        }

        let pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&rgba, width, height);
        return Some(Image::from_rgba8(pixel_buffer));
    }

    None
}
