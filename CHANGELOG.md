# Changelog

All notable changes to **SvagView** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-11-25

### Fixed
- Fixed Windows minimum horizontal window size calculation so Slint derives exact track bounds dynamically from toolbar contents and installed external editors, preventing right-side toolbar buttons from being cropped on horizontal resize.
- Enabled graceful filename and `viewBox` text elision inside minimum bounds when the window is narrowed.

### Changed
- Updated `resources/SvagView_Logo_Icon.ico` and added runtime ICO decoding for crisp multi-resolution window and UI icons.
- Updated installer metadata, UI version footer, and release workflows to `v0.1.1`.

## [0.1.0] - 2026-11-24

### Added
- **True Vector Rendering Engine**:
  - Full SVG 1.1 / SVG 2 rendering powered by `usvg`, `resvg`, and `tiny-skia`.
  - Infinite vector zoom up to 500x with zero pixelation or raster artifacts.
  - 1:1 cursor-locked panning without acceleration drift or jumping.
  - Transparent `.svg` and gzip-compressed `.svgz` support.
  - Automatic system font resolution via `fontdb`.
- **User Interface**:
  - Native desktop UI built with Slint.
  - Default Light Mode theme and sleek Dark Graphite Mode (`T` or toggle button).
  - Four canvas background styles: White (default), Checkerboard transparency grid, Deep Black, and Dark Slate (`B` or cycle button).
  - Dynamic responsive layout with minimum window dimensions and symmetrical padding.
  - Symmetrical top and bottom toolbars with centered vector icons.
  - Interactive file drop zone and welcome screen with Lanczos3 high-quality downscaled logo banner.
- **Workflow & Productivity**:
  - Drag-and-drop file support.
  - Native file picker dialog (`Ctrl+O`).
  - Next/Previous folder image navigation (`Left` / `Right` arrows, `PageUp` / `PageDown`).
  - File reload with live disk sync (`F5` / `Ctrl+R`).
  - Native Windows executable icon embedding (`SvagView_Logo_Icon.ico`).
  - Automatic external vector graphics editor detection (Inkscape, Adobe Illustrator, CorelDRAW, Affinity Designer, VS Code).
  - Direct PNG export with customizable resolution scaling (1x, 2x HiDPI, 4x UHD, 8x Print) and background presets (`Ctrl+E` / `Ctrl+S`).
  - Clipboard integration: Copy rendered PNG bitmap (`Ctrl+C`) and copy SVG XML markup (`Ctrl+Shift+C`).
  - Fullscreen mode with toggle icon (`F11`).
  - Comprehensive SVG document inspection panel (`I`).
  - Persistent user preferences saved across sessions (`%APPDATA%/SvagView/settings.json` on Windows, `$XDG_CONFIG_HOME/svagview/settings.json` on Linux).
- **Quality & CI/CD**:
  - Comprehensive automated test suite with 11 tests verifying parser, rendering, zoom accuracy, navigation, icon generation, and persistence.
  - GitHub Actions CI workflow for Windows, Linux, and macOS.
  - Automated multi-platform GitHub Release workflow.
