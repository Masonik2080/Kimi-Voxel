// ============================================
// Biome Selector - Выбор биома по климату
// ============================================

use super::types::*;
use super::climate::{climate_map, ClimateMap};
use super::registry::biome_registry;

/// Селектор биомов на основе климатической карты
pub struct BiomeSelector {
    climate: &'static ClimateMap,
}

impl BiomeSelector {
    pub fn new() -> Self {
        Self {
            climate: climate_map(),
        }
    }

    /// Получить биом для координат
    #[inline]
    pub fn get_biome(&self, x: i32, z: i32) -> BiomeId {
        let climate = self.climate.sample(x as f32, z as f32);
        self.select_from_climate(&climate)
    }

    /// Получить биом и климат для координат
    #[inline]
    pub fn get_biome_with_climate(&self, x: f32, z: f32) -> (BiomeId, ClimateData) {
        let climate = self.climate.sample(x, z);
        let biome = self.select_from_climate(&climate);
        (biome, climate)
    }

    /// Получить определение биома
    #[inline]
    pub fn get_biome_def(&self, x: i32, z: i32) -> &'static BiomeDefinition {
        let id = self.get_biome(x, z);
        biome_registry().get(id)
    }

    /// Выбор биома по климатическим данным
    fn select_from_climate(&self, climate: &ClimateData) -> BiomeId {
        let t = climate.temperature;
        let h = climate.humidity;
        let c = climate.continentalness;

        // Океан - низкая континентальность
        if c < 0.25 {
            return BIOME_OCEAN;
        }

        // Горы - появляются при высокой континентальности
        // Упрощённое условие - просто высокий континентальность
        if c > 0.55 {
            return BIOME_MOUNTAINS;
        }

        // Температурно-влажностная сетка
        match (t, h) {
            // Холодно (t < 0.25)
            (t, h) if t < 0.25 && h < 0.4 => BIOME_TUNDRA,
            (t, _) if t < 0.25 => BIOME_TAIGA,

            // Жарко (t > 0.75)
            (t, h) if t > 0.75 && h < 0.25 => BIOME_DESERT,
            (t, h) if t > 0.75 && h > 0.7 => BIOME_JUNGLE,
            (t, _) if t > 0.75 => BIOME_SAVANNA,

            // Умеренно (0.25 <= t <= 0.75)
            (_, h) if h > 0.8 => BIOME_SWAMP,
            (_, h) if h > 0.5 => BIOME_FOREST,
            _ => BIOME_PLAINS,
        }
    }
}

impl Default for BiomeSelector {
    fn default() -> Self {
        Self::new()
    }
}

use std::sync::OnceLock;
static BIOME_SELECTOR: OnceLock<BiomeSelector> = OnceLock::new();

pub fn biome_selector() -> &'static BiomeSelector {
    BIOME_SELECTOR.get_or_init(BiomeSelector::default)
}
