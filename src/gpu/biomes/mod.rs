// ============================================
// Biomes Module - Система биомов
// ============================================
// 
// Климатическая карта определяет биом:
// - Temperature (температура): холодно -> жарко
// - Humidity (влажность): сухо -> влажно
// - Continentalness: океан -> суша -> горы
// - Erosion: плоско -> холмисто
//
// Каждый биом имеет свой тип генерации terrain:
// - Flat: болота, тундра
// - Rolling: равнины, леса
// - Mountains3D: горы с 3D шумом (карнизы, нависания)
// - Valley: долины с крутыми стенами
// - Ocean: океанское дно

mod types;
mod climate;
mod registry;
mod selector;
mod terrain_gen;
pub mod features;
pub mod foliage;

pub use types::*;
pub use climate::*;
pub use registry::*;
pub use selector::*;
pub use terrain_gen::*;
pub use foliage::{FoliageCache, is_leaf_block};
