use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackgroundStyle {
    Checkerboard = 0,
    White = 1,
    Black = 2,
    Custom = 3,
}

pub struct AppState {
    pub current_path: Option<PathBuf>,
    pub folder_files: Vec<PathBuf>,
    pub current_index: usize,
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub bg_style: BackgroundStyle,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_path: None,
            folder_files: Vec::new(),
            current_index: 0,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            bg_style: BackgroundStyle::White,
        }
    }
}

impl AppState {
    pub fn set_current_file(&mut self, path: PathBuf) {
        if let Some(parent) = path.parent() {
            self.refresh_folder(parent, Some(&path));
        } else {
            self.folder_files = vec![path.clone()];
            self.current_index = 0;
        }
        self.current_path = Some(path);
        self.reset_pan();
    }

    pub fn refresh_folder(&mut self, dir: &Path, target: Option<&Path>) {
        let mut svg_files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if ext_lower == "svg" || ext_lower == "svgz" {
                            svg_files.push(p);
                        }
                    }
                }
            }
        }

        // Sort alphabetically case-insensitively
        svg_files.sort_by(|a, b| {
            a.to_string_lossy()
                .to_lowercase()
                .cmp(&b.to_string_lossy().to_lowercase())
        });

        if let Some(t) = target {
            self.current_index = svg_files.iter().position(|p| p == t).unwrap_or(0);
        } else if self.current_index >= svg_files.len() {
            self.current_index = 0;
        }

        self.folder_files = svg_files;
    }

    pub fn next_file(&mut self) -> Option<PathBuf> {
        if let Some(cur) = self.current_path.clone() {
            if let Some(parent) = cur.parent() {
                self.refresh_folder(parent, Some(&cur));
            }
        }
        if self.folder_files.is_empty() {
            return None;
        }

        let total = self.folder_files.len();
        for _ in 0..total {
            self.current_index = (self.current_index + 1) % total;
            let candidate = &self.folder_files[self.current_index];
            if candidate.is_file() {
                let path = candidate.clone();
                self.current_path = Some(path.clone());
                self.reset_pan();
                return Some(path);
            }
        }
        None
    }

    pub fn prev_file(&mut self) -> Option<PathBuf> {
        if let Some(cur) = self.current_path.clone() {
            if let Some(parent) = cur.parent() {
                self.refresh_folder(parent, Some(&cur));
            }
        }
        if self.folder_files.is_empty() {
            return None;
        }

        let total = self.folder_files.len();
        for _ in 0..total {
            if self.current_index == 0 {
                self.current_index = total - 1;
            } else {
                self.current_index -= 1;
            }
            let candidate = &self.folder_files[self.current_index];
            if candidate.is_file() {
                let path = candidate.clone();
                self.current_path = Some(path.clone());
                self.reset_pan();
                return Some(path);
            }
        }
        None
    }

    pub fn first_file(&mut self) -> Option<PathBuf> {
        if let Some(cur) = self.current_path.clone() {
            if let Some(parent) = cur.parent() {
                self.refresh_folder(parent, Some(&cur));
            }
        }
        if self.folder_files.is_empty() {
            return None;
        }

        for (idx, candidate) in self.folder_files.iter().enumerate() {
            if candidate.is_file() {
                self.current_index = idx;
                let path = candidate.clone();
                self.current_path = Some(path.clone());
                self.reset_pan();
                return Some(path);
            }
        }
        None
    }

    pub fn last_file(&mut self) -> Option<PathBuf> {
        if let Some(cur) = self.current_path.clone() {
            if let Some(parent) = cur.parent() {
                self.refresh_folder(parent, Some(&cur));
            }
        }
        if self.folder_files.is_empty() {
            return None;
        }

        for (idx, candidate) in self.folder_files.iter().enumerate().rev() {
            if candidate.is_file() {
                self.current_index = idx;
                let path = candidate.clone();
                self.current_path = Some(path.clone());
                self.reset_pan();
                return Some(path);
            }
        }
        None
    }

    pub fn reset_pan(&mut self) {
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }

    #[allow(dead_code)]
    pub fn zoom_in(&mut self) {
        let cur = if self.zoom.is_finite() && self.zoom > 0.0 {
            self.zoom
        } else {
            1.0
        };
        self.zoom = (cur * 1.25).clamp(0.01, 500.0);
    }

    #[allow(dead_code)]
    pub fn zoom_out(&mut self) {
        let cur = if self.zoom.is_finite() && self.zoom > 0.0 {
            self.zoom
        } else {
            1.0
        };
        self.zoom = (cur / 1.25).clamp(0.01, 500.0);
    }

    pub fn set_zoom(&mut self, val: f32) {
        let v = if val.is_finite() && val > 0.0 {
            val
        } else {
            1.0
        };
        self.zoom = v.clamp(0.01, 500.0);
    }

    pub fn fit_to_window(&mut self, view_w: f32, view_h: f32, svg_w: f32, svg_h: f32) -> f32 {
        if view_w <= 0.0
            || view_h <= 0.0
            || svg_w <= 0.0
            || svg_h <= 0.0
            || !view_w.is_finite()
            || !view_h.is_finite()
            || !svg_w.is_finite()
            || !svg_h.is_finite()
        {
            self.zoom = 1.0;
            self.reset_pan();
            return 1.0;
        }

        // Add 32px padding around image for breathing room
        let pad_w = (view_w - 32.0).max(10.0);
        let pad_h = (view_h - 32.0).max(10.0);

        let scale_x = pad_w / svg_w;
        let scale_y = pad_h / svg_h;
        let scale = scale_x.min(scale_y);

        let clamped = if scale.is_finite() && scale > 0.0 {
            scale.clamp(0.01, 500.0)
        } else {
            1.0
        };

        self.zoom = clamped;
        self.reset_pan();
        clamped
    }

    pub fn current_file_info(&self) -> (usize, usize) {
        if self.folder_files.is_empty() {
            (0, 0)
        } else {
            (self.current_index + 1, self.folder_files.len())
        }
    }
}
