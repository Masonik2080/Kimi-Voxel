// ============================================
// Core Module - Основные компоненты и ресурсы
// ============================================

pub mod app;
mod resources;
mod config;

pub use app::App;
pub use resources::GameResources;
pub use config::{SAVE_FILE, DEFAULT_SEED};
