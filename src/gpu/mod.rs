// ============================================
// GPU Module - Бесконечный terrain на шейдерах
// ============================================
// Система Player + Camera с режимами 1-го и 3-го лица
// Рефакторинг: разделено на модули по ECS-принципам

pub mod terrain;
pub mod blocks;
pub mod lighting;
pub mod render;
pub mod gui;
pub mod save;
pub mod audio;
pub mod player;
pub mod subvoxel;
pub mod biomes;

// Новые модули после рефакторинга
pub mod core;
pub mod systems;

// Реэкспорт для обратной совместимости
pub use core::app::run;
