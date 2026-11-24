use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug)]
pub struct SvgEditor {
    pub id: String,
    pub name: String,
    pub icon_text: String,
    pub executable: PathBuf,
}

pub fn detect_editors() -> Vec<SvgEditor> {
    let mut editors = Vec::new();

    // 1. Inkscape
    if let Some(path) = find_inkscape() {
        editors.push(SvgEditor {
            id: "inkscape".to_string(),
            name: "Inkscape".to_string(),
            icon_text: "🖌️".to_string(),
            executable: path,
        });
    }

    // 2. Adobe Illustrator
    if let Some(path) = find_illustrator() {
        editors.push(SvgEditor {
            id: "illustrator".to_string(),
            name: "Illustrator".to_string(),
            icon_text: "🎨".to_string(),
            executable: path,
        });
    }

    // 3. CorelDRAW
    if let Some(path) = find_coreldraw() {
        editors.push(SvgEditor {
            id: "coreldraw".to_string(),
            name: "CorelDRAW".to_string(),
            icon_text: "📐".to_string(),
            executable: path,
        });
    }

    // 4. Affinity Designer
    if let Some(path) = find_affinity() {
        editors.push(SvgEditor {
            id: "affinity".to_string(),
            name: "Affinity".to_string(),
            icon_text: "✨".to_string(),
            executable: path,
        });
    }

    // 5. VS Code (for code view / editing)
    if let Some(path) = find_vscode() {
        editors.push(SvgEditor {
            id: "vscode".to_string(),
            name: "VS Code".to_string(),
            icon_text: "💻".to_string(),
            executable: path,
        });
    }

    editors
}

fn normalize_path<P: AsRef<Path>>(p: P) -> PathBuf {
    let path = p.as_ref();
    if path.is_absolute() {
        path.to_path_buf()
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.join(path)
    } else {
        path.to_path_buf()
    }
}

pub fn open_in_editor<P: AsRef<Path>>(editor: &SvgEditor, file_path: P) -> Result<(), String> {
    let path = normalize_path(file_path);
    Command::new(&editor.executable)
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to launch {}: {}", editor.name, e))?;
    Ok(())
}

pub fn open_in_system_default<P: AsRef<Path>>(file_path: P) -> Result<(), String> {
    let path = normalize_path(file_path);
    open::that(&path).map_err(|e| format!("Failed to open with system default: {}", e))
}

fn check_path(p: PathBuf) -> Option<PathBuf> {
    if p.is_file() {
        Some(p)
    } else {
        None
    }
}

fn find_in_path(binary_name: &str) -> Option<PathBuf> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(binary_name);
            if candidate.is_file() {
                return Some(candidate);
            }
            #[cfg(windows)]
            {
                let candidate_exe = dir.join(format!("{}.exe", binary_name));
                if candidate_exe.is_file() {
                    return Some(candidate_exe);
                }
                let candidate_cmd = dir.join(format!("{}.cmd", binary_name));
                if candidate_cmd.is_file() {
                    return Some(candidate_cmd);
                }
            }
        }
    }
    None
}

fn find_inkscape() -> Option<PathBuf> {
    // Linux / Mac / PATH check
    if let Some(p) = find_in_path("inkscape") {
        return Some(p);
    }

    #[cfg(windows)]
    {
        let candidates = [
            r"C:\Program Files\Inkscape\bin\inkscape.exe",
            r"C:\Program Files\Inkscape\inkscape.exe",
            r"C:\Program Files (x86)\Inkscape\bin\inkscape.exe",
            r"C:\Program Files (x86)\Inkscape\inkscape.exe",
        ];
        for c in candidates {
            if let Some(p) = check_path(PathBuf::from(c)) {
                return Some(p);
            }
        }

        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(&local_app_data)
                .join("Programs")
                .join("Inkscape")
                .join("bin")
                .join("inkscape.exe");
            if let Some(found) = check_path(p) {
                return Some(found);
            }
        }
    }

    #[cfg(unix)]
    {
        let unix_candidates = [
            "/usr/bin/inkscape",
            "/usr/local/bin/inkscape",
            "/app/bin/inkscape",
            "/snap/bin/inkscape",
        ];
        for c in unix_candidates {
            if let Some(p) = check_path(PathBuf::from(c)) {
                return Some(p);
            }
        }
    }

    None
}

fn find_illustrator() -> Option<PathBuf> {
    if let Some(p) = find_in_path("illustrator") {
        return Some(p);
    }

    #[cfg(windows)]
    {
        let adobe_root = Path::new(r"C:\Program Files\Adobe");
        if adobe_root.is_dir() {
            if let Ok(entries) = std::fs::read_dir(adobe_root) {
                for entry in entries.flatten() {
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    if dir_name.starts_with("Adobe Illustrator") {
                        let candidate = entry
                            .path()
                            .join("Support Files")
                            .join("Contents")
                            .join("Windows")
                            .join("Illustrator.exe");
                        if let Some(p) = check_path(candidate) {
                            return Some(p);
                        }
                    }
                }
            }
        }
    }

    None
}

fn find_coreldraw() -> Option<PathBuf> {
    if let Some(p) = find_in_path("coreldrw") {
        return Some(p);
    }

    #[cfg(windows)]
    {
        let corel_root = Path::new(r"C:\Program Files\Corel");
        if corel_root.is_dir() {
            if let Ok(entries) = std::fs::read_dir(corel_root) {
                for entry in entries.flatten() {
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    if dir_name.starts_with("CorelDRAW Graphics Suite") {
                        let p1 = entry.path().join("Programs64").join("CorelDRW.exe");
                        if let Some(p) = check_path(p1) {
                            return Some(p);
                        }
                        // Check subdirectories like "27/Programs64/CorelDRW.exe" or "2024/Programs64/CorelDRW.exe"
                        if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                            for sub in sub_entries.flatten() {
                                let p2 = sub.path().join("Programs64").join("CorelDRW.exe");
                                if let Some(p) = check_path(p2) {
                                    return Some(p);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn find_affinity() -> Option<PathBuf> {
    if let Some(p) = find_in_path("Designer") {
        return Some(p);
    }

    #[cfg(windows)]
    {
        let candidates = [
            r"C:\Program Files\Affinity\Designer 2\Designer.exe",
            r"C:\Program Files\Affinity\Designer\Designer.exe",
        ];
        for c in candidates {
            if let Some(p) = check_path(PathBuf::from(c)) {
                return Some(p);
            }
        }
    }

    None
}

fn find_vscode() -> Option<PathBuf> {
    if let Some(p) = find_in_path("code") {
        return Some(p);
    }

    #[cfg(windows)]
    {
        let candidates = [
            r"C:\Program Files\Microsoft VS Code\bin\code.cmd",
            r"C:\Program Files\Microsoft VS Code\Code.exe",
        ];
        for c in candidates {
            if let Some(p) = check_path(PathBuf::from(c)) {
                return Some(p);
            }
        }

        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(&local_app_data)
                .join("Programs")
                .join("Microsoft VS Code")
                .join("Code.exe");
            if let Some(found) = check_path(p) {
                return Some(found);
            }
        }
    }

    None
}
