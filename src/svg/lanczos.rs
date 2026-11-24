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

/// Helper: Load and Lanczos3-downscale an image into a slint::Image
pub fn create_lanczos_scaled_image(
    png_bytes: &[u8],
    target_w: u32,
    target_h: u32,
) -> Option<Image> {
    let pixmap = resvg::tiny_skia::Pixmap::decode_png(png_bytes).ok()?;
    let src_w = pixmap.width();
    let src_h = pixmap.height();

    // Convert premultiplied RGBA from tiny_skia to straight RGBA
    let mut straight_rgba = Vec::with_capacity((src_w * src_h * 4) as usize);
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

    let downscaled = lanczos3_downscale(&straight_rgba, src_w, src_h, target_w, target_h);

    let pixel_buffer =
        SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&downscaled, target_w, target_h);
    Some(Image::from_rgba8(pixel_buffer))
}
