// ============================================
// Biome Types - Типы биомов
// ============================================

use crate::gpu::blocks::BlockType;

/// ID биома
pub type BiomeId = u8;

// Константы биомов
pub const BIOME_OCEAN: BiomeId = 0;
pub const BIOME_PLAINS: BiomeId = 1;
pub const BIOME_DESERT: BiomeId = 2;
pub const BIOME_FOREST: BiomeId = 3;
pub const BIOME_TAIGA: BiomeId = 4;
pub const BIOME_TUNDRA: BiomeId = 5;
pub const BIOME_SWAMP: BiomeId = 6;
pub const BIOME_MOUNTAINS: BiomeId = 7;
pub const BIOME_SAVANNA: BiomeId = 8;
pub const BIOME_JUNGLE: BiomeId = 9;

/// Тип генерации terrain для биома
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TerrainType {
    /// Плоский terrain (болота, равнины)
    Flat,
    /// Стандартные холмы
    Rolling,
    /// Горы с 3D шумом (карнизы, пещеры)
    Mountains3D,
    /// Долины с крутыми стенами
    Valley,
    /// Океан (ниже уровня воды)
    Ocean,
}

/// Определение биома
#[derive(Clone, Debug)]
pub struct BiomeDefinition {
    pub id: BiomeId,
    pub name: &'static str,
    /// Поверхностный блок
    pub surface_block: BlockType,
    /// Подповерхностный блок
    pub subsurface_block: BlockType,
    /// Глубинный блок
    pub deep_block: BlockType,
    /// Базовая высота terrain
    pub base_height: f32,
    /// Амплитуда высоты
    pub height_variation: f32,
    /// Тип генерации terrain
    pub terrain_type: TerrainType,
    /// Температура (0.0 - холодно, 1.0 - жарко)
    pub temperature: f32,
    /// Влажность (0.0 - сухо, 1.0 - влажно)
    pub humidity: f32,
    /// Сила 3D шума для гор (0.0 - нет, 1.0 - максимум)
    pub noise_3d_strength: f32,
    /// Плотность деревьев (0.0 - нет, 0.015 - лес, 0.001 - редкие)
    pub tree_density: f32,
}

impl BiomeDefinition {
    pub const fn new(
        id: BiomeId,
        name: &'static str,
        surface_block: BlockType,
        subsurface_block: BlockType,
        deep_block: BlockType,
    ) -> Self {
        Self {
            id,
            name,
            surface_block,
            subsurface_block,
            deep_block,
            base_height: 20.0,
            height_variation: 10.0,
            terrain_type: TerrainType::Rolling,
            temperature: 0.5,
            humidity: 0.5,
            noise_3d_strength: 0.0,
            tree_density: 0.0,
        }
    }

    pub const fn with_terrain(mut self, base_height: f32, height_variation: f32, terrain_type: TerrainType) -> Self {
        self.base_height = base_height;
        self.height_variation = height_variation;
        self.terrain_type = terrain_type;
        self
    }

    pub const fn with_climate(mut self, temperature: f32, humidity: f32) -> Self {
        self.temperature = temperature;
        self.humidity = humidity;
        self
    }

    pub const fn with_3d_noise(mut self, strength: f32) -> Self {
        self.noise_3d_strength = strength;
        self
    }

    pub const fn with_trees(mut self, density: f32) -> Self {
        self.tree_density = density;
        self
    }
}

/// Климатические данные для точки
#[derive(Clone, Copy, Debug)]
pub struct ClimateData {
    pub temperature: f32,
    pub humidity: f32,
    pub continentalness: f32,
    pub erosion: f32,
}
