pub mod clipboard;
pub mod config;
pub mod editors;
pub mod state;

pub use clipboard::{copy_image_to_clipboard, copy_text_to_clipboard};
pub use config::AppConfig;
pub use editors::{detect_editors, open_in_editor, open_in_system_default};
pub use state::{AppState, BackgroundStyle};
