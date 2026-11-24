use resvg::usvg::{self, Node, Tree};
use std::path::Path;

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct SvgMetadata {
    pub file_name: String,
    pub file_path: String,
    pub file_size_bytes: u64,
    pub file_size_formatted: String,
    pub width: f32,
    pub height: f32,
    pub aspect_ratio: f32,
    pub view_box: String,
    pub path_count: usize,
    pub group_count: usize,
    pub text_count: usize,
    pub image_count: usize,
    pub gradient_count: usize,
    pub clip_path_count: usize,
    pub mask_count: usize,
    pub filter_count: usize,
}

impl SvgMetadata {
    pub fn from_tree_and_file(tree: &Tree, raw_svg: &[u8], path: Option<&Path>) -> Self {
        let size = tree.size();
        let width = size.width();
        let height = size.height();
        let aspect_ratio = if height > 0.0 && height.is_finite() && width.is_finite() {
            width / height
        } else {
            1.0
        };

        let file_name = path
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled.svg")
            .to_string();

        let file_path = path.and_then(|p| p.to_str()).unwrap_or("").to_string();

        let file_size_bytes = path
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .unwrap_or(raw_svg.len() as u64);

        let file_size_formatted = format_bytes(file_size_bytes);

        let view_box =
            extract_view_box(raw_svg).unwrap_or_else(|| format!("0 0 {width:.0} {height:.0}"));

        let mut path_count = 0;
        let mut group_count = 0;
        let mut text_count = 0;
        let mut image_count = 0;

        count_nodes(
            tree.root(),
            &mut path_count,
            &mut group_count,
            &mut text_count,
            &mut image_count,
        );

        let gradient_count = tree.linear_gradients().len() + tree.radial_gradients().len();
        let clip_path_count = tree.clip_paths().len();
        let mask_count = tree.masks().len();
        let filter_count = tree.filters().len();

        SvgMetadata {
            file_name,
            file_path,
            file_size_bytes,
            file_size_formatted,
            width,
            height,
            aspect_ratio,
            view_box,
            path_count,
            group_count,
            text_count,
            image_count,
            gradient_count,
            clip_path_count,
            mask_count,
            filter_count,
        }
    }
}

fn count_nodes(
    group: &usvg::Group,
    paths: &mut usize,
    groups: &mut usize,
    texts: &mut usize,
    images: &mut usize,
) {
    for child in group.children() {
        match child {
            Node::Group(g) => {
                *groups += 1;
                count_nodes(g, paths, groups, texts, images);
            }
            Node::Path(_) => *paths += 1,
            Node::Image(_) => *images += 1,
            Node::Text(_) => *texts += 1,
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn extract_view_box(raw_svg: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(raw_svg).ok()?;
    let s_lower = s.to_lowercase();
    let svg_start = s_lower.find("<svg")?;
    let svg_end = s[svg_start..].find('>')? + svg_start;
    let header = &s[svg_start..svg_end];

    // Find viewBox=" or viewBox=' (case-insensitive)
    let vb_idx = header.to_lowercase().find("viewbox")?;
    let rem = &header[vb_idx + 7..];
    let eq_idx = rem.find('=')?;
    let val_rem = rem[eq_idx + 1..].trim_start();
    let quote = val_rem.chars().next()?;
    if quote == '"' || quote == '\'' {
        let val_body = &val_rem[1..];
        let end_quote = val_body.find(quote)?;
        return Some(val_body[..end_quote].trim().to_string());
    }
    None
}
