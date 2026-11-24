# Contributing to SvagView

Thank you for your interest in contributing to **SvagView**! We welcome bug reports, feature proposals, documentation improvements, and pull requests.

## Code of Conduct

Please be respectful, constructive, and welcoming in all interactions across issues, discussions, and pull requests.

## Development Setup

### Prerequisites
- **Rust Toolchain**: Rust 1.85+ (`rustup update stable`)
- **System Dependencies** (Linux only):
  ```bash
  sudo apt-get install -y libfontconfig1-dev libxkbcommon-dev libxcb-shape0-dev libxcb-xfixes0-dev
  ```

### Building the Project
```bash
# Clone the repository
git clone https://github.com/vaibhav-upadhyay/SvagView.git
cd SvagView

# Run debug build
cargo run

# Run with an SVG sample
cargo run -- test_assets/icon_dashboard.svg
```

### Running Tests
Ensure all unit and integration tests pass:
```bash
cargo test -- --nocapture
```

### Code Formatting and Linting
All code must pass formatting checks and clippy with zero warnings:
```bash
cargo fmt -- --check
cargo clippy --all-targets
```

## Pull Request Guidelines

1. **Create a branch**: Branch off `main` or `master` with a descriptive name (e.g., `feature/custom-zoom-slider` or `fix/viewbox-parsing`).
2. **Commit clearly**: Write concise, informative commit messages following conventional commits style.
3. **Add tests**: Add unit tests in `tests/svg_tests.rs` or relevant modules for new features and bug fixes.
4. **Verify UI**: If changing UI components, verify both Light Mode and Dark Graphite Mode for contrast and alignment.
5. **Open a PR**: Fill in the pull request template and link any relevant issues.
