// ============================================
// Lighting Module - Cascaded Shadow Maps (CSM)
// ============================================
// Оптимизированная система теней для больших миров
// Поддержка теней от гор, построек и других объектов

mod csm;
mod shadow_map;
mod light;
mod cascade;
mod celestial;
mod celestial_render;

pub use csm::CascadedShadowMaps;
pub use shadow_map::ShadowMap;
pub use light::{DirectionalLight, SunLight};
pub use cascade::{Cascade, CascadeConfig};
pub use celestial::{DayNightCycle, TimeOfDay, Sun, Moon, CelestialBody};
pub use celestial_render::CelestialRenderer;
