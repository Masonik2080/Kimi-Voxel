// ============================================
// Climate Map - Климатическая карта мира
// ============================================

use super::types::ClimateData;
use crate::gpu::terrain::generation::noise::{fbm2d, noise2d};

/// Генератор климатической карты
pub struct ClimateMap {
    seed: u64,
}

impl ClimateMap {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Получить климатические данные для координат
    #[inline]
    pub fn sample(&self, x: f32, z: f32) -> ClimateData {
        let seed_offset = self.seed as f32 * 0.1;
        
        // Температура - крупномасштабный шум
        // Зависит от "широты" (z) + шум для вариации
        let temp_base = fbm2d(
            x * 0.0008 + seed_offset,
            z * 0.0008,
            3
        );
        // Добавляем градиент по Z для имитации широты
        let latitude_factor = (z * 0.0001).sin() * 0.3;
        let temperature = (temp_base + latitude_factor + 0.5).clamp(0.0, 1.0);

        // Влажность - другой масштаб шума
        let humidity = fbm2d(
            x * 0.001 + seed_offset + 500.0,
            z * 0.001 + 500.0,
            3
        ).clamp(0.0, 1.0);

        // Континентальность - ОЧЕНЬ низкая частота для больших горных массивов
        let continentalness = fbm2d(
            x * 0.0002 + seed_offset + 1000.0,
            z * 0.0002 + 1000.0,
            4
        );

        // Эрозия - тоже низкая частота
        let erosion = fbm2d(
            x * 0.0005 + seed_offset + 2000.0,
            z * 0.0005 + 2000.0,
            3
        );

        ClimateData {
            temperature,
            humidity,
            continentalness,
            erosion,
        }
    }

    /// Быстрая версия для LOD
    #[inline]
    pub fn sample_fast(&self, x: f32, z: f32) -> ClimateData {
        let seed_offset = self.seed as f32 * 0.1;
        
        let temperature = noise2d(
            x * 0.0008 + seed_offset,
            z * 0.0008
        );
        
        let humidity = noise2d(
            x * 0.001 + seed_offset + 500.0,
            z * 0.001 + 500.0
        );

        let continentalness = noise2d(
            x * 0.0003 + seed_offset + 1000.0,
            z * 0.0003 + 1000.0
        );

        ClimateData {
            temperature,
            humidity,
            continentalness,
            erosion: 0.5,
        }
    }
}

impl Default for ClimateMap {
    fn default() -> Self {
        Self::new(42)
    }
}

use std::sync::OnceLock;
static CLIMATE_MAP: OnceLock<ClimateMap> = OnceLock::new();

pub fn climate_map() -> &'static ClimateMap {
    CLIMATE_MAP.get_or_init(ClimateMap::default)
}
