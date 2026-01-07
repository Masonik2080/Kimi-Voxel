// ============================================
// Библиотека блоков для GPU движка
// ============================================
// Data-Driven Architecture: блоки загружаются из JSON

mod types;
mod definition;
mod registry;
mod block_breaker;
mod worldgen;
pub mod texture_atlas;

pub use types::*;
pub use definition::*;
pub use registry::*;
pub use block_breaker::*;
pub use worldgen::*;
