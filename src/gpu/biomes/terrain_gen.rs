// ============================================
// Biome Terrain Generation - Генерация terrain по биому
// ============================================

use super::types::*;
use super::selector::biome_selector;
use super::registry::biome_registry;
use crate::gpu::terrain::generation::noise::{fbm2d, noise3d};

/// Генератор terrain с учётом биомов
pub struct BiomeTerrainGen;

impl BiomeTerrainGen {
    /// Получить высоту terrain с ПЛАВНЫМ переходом между биомами
    /// Использует континентальность напрямую для плавных склонов
    #[inline]
    pub fn get_height(x: f32, z: f32) -> f32 {
        let (biome_id, climate) = biome_selector().get_biome_with_climate(x, z);
        let biome = biome_registry().get(biome_id);
        
        // Континентальность определяет "горность" - это уже плавное значение из шума!
        let c = climate.continentalness;
        
        // Базовая высота равнины
        let plains_height = 20.0 + fbm2d(x * 0.005, z * 0.005, 3) * 8.0;
        
        // Если это не горы - просто возвращаем высоту биома
        if biome.terrain_type != TerrainType::Mountains3D {
            // Но даже для равнин добавляем небольшой подъём при высокой континентальности
            let lift = (c - 0.4).max(0.0) * 30.0;
            return Self::height_for_biome(x, z, biome, &climate) + lift;
        }
        
        // Для гор: плавный переход от равнины к горам на основе континентальности
        // c = 0.55 это граница гор, делаем плавный подъём от 0.3 до 0.8
        let mountain_factor = ((c - 0.3) / 0.5).clamp(0.0, 1.0);
        // Smoothstep для ещё более плавного перехода
        let mountain_factor = mountain_factor * mountain_factor * (3.0 - 2.0 * mountain_factor);
        
        // Высота горы
        let mountain_height = Self::raw_mountain_height(x, z, biome);
        
        // Интерполяция: равнина -> предгорья -> горы
        plains_height + (mountain_height - plains_height) * mountain_factor
    }
    
    /// Сырая высота горы с острыми пиками (Ridged Noise)
    fn raw_mountain_height(x: f32, z: f32, biome: &BiomeDefinition) -> f32 {
        // Ridged Noise: 1.0 - abs(noise) создает острые пики ("гребни")
        let ridge1 = 1.0 - fbm2d(x * 0.003, z * 0.003, 1).abs();
        let ridge1 = ridge1.powf(4.0); // Делаем пики очень острыми
        
        let ridge2 = 1.0 - fbm2d(x * 0.01, z * 0.01, 1).abs();
        let ridge2 = ridge2.powf(2.0);
        
        // Основная форма (низкая частота)
        let main_shape = fbm2d(x * 0.0005, z * 0.0005, 3);
        
        // Смешиваем: основная форма поднимает землю, а ridged noise добавляет скалы
        let combined = main_shape * 0.5 + ridge1 * 1.2 + ridge2 * 0.3;
        
        biome.base_height + combined * biome.height_variation * 2.5
    }

    /// Генерация высоты для конкретного биома
    fn height_for_biome(x: f32, z: f32, biome: &BiomeDefinition, climate: &ClimateData) -> f32 {
        match biome.terrain_type {
            TerrainType::Flat => Self::flat_terrain(x, z, biome),
            TerrainType::Rolling => Self::rolling_terrain(x, z, biome, climate),
            TerrainType::Mountains3D => Self::mountain_terrain(x, z, biome),
            TerrainType::Valley => Self::valley_terrain(x, z, biome),
            TerrainType::Ocean => Self::ocean_terrain(x, z, biome),
        }
    }

    /// Плоский terrain (болота, тундра)
    fn flat_terrain(x: f32, z: f32, biome: &BiomeDefinition) -> f32 {
        let noise = fbm2d(x * 0.01, z * 0.01, 2);
        biome.base_height + noise * biome.height_variation
    }

    /// Холмистый terrain (равнины, леса)
    fn rolling_terrain(x: f32, z: f32, biome: &BiomeDefinition, climate: &ClimateData) -> f32 {
        let base = fbm2d(x * 0.005, z * 0.005, 4);
        let detail = fbm2d(x * 0.02, z * 0.02, 2) * 0.3;
        
        // Эрозия сглаживает terrain
        let erosion_factor = 1.0 - climate.erosion * 0.5;
        
        biome.base_height + (base + detail) * biome.height_variation * erosion_factor
    }

    /// Горный terrain - острые скалистые горы с warp
    fn mountain_terrain(x: f32, z: f32, biome: &BiomeDefinition) -> f32 {
        // Warp (смещение координат) для "скалистости"
        let warp_x = fbm2d(x * 0.002, z * 0.002, 2);
        let warp_z = fbm2d(x * 0.002 + 100.0, z * 0.002 + 100.0, 2);
        
        let qx = x + warp_x * 50.0;
        let qz = z + warp_z * 50.0;
        
        // Ridged noise для острых пиков
        let base = 1.0 - fbm2d(qx * 0.001, qz * 0.001, 4).abs();
        let sharp_peaks = base.powf(3.0); // Очень острые пики
        
        biome.base_height + sharp_peaks * biome.height_variation * 3.0
    }

    /// Долины с крутыми стенами
    fn valley_terrain(x: f32, z: f32, biome: &BiomeDefinition) -> f32 {
        let base = fbm2d(x * 0.004, z * 0.004, 3);
        
        // Создаём V-образный профиль
        let valley_shape = (fbm2d(x * 0.002, z * 0.002, 2) * 2.0 - 1.0).abs();
        
        biome.base_height + base * biome.height_variation * valley_shape
    }

    /// Океанское дно
    fn ocean_terrain(x: f32, z: f32, biome: &BiomeDefinition) -> f32 {
        let base = fbm2d(x * 0.003, z * 0.003, 3);
        biome.base_height + base * biome.height_variation
    }

    /// 3D density для гор (карнизы, пещеры в горах)
    /// Возвращает density: > 0 = твёрдый блок, < 0 = воздух
    #[inline]
    pub fn get_3d_density(x: f32, y: f32, z: f32) -> f32 {
        let biome = biome_selector().get_biome_def(x as i32, z as i32);
        
        if biome.noise_3d_strength < 0.01 {
            // Нет 3D шума - используем простую карту высот
            let height = Self::get_height(x, z);
            return height - y;
        }

        // Базовая высота
        let base_height = Self::get_height(x, z);
        let height_density = base_height - y;

        // 3D шум для карнизов и нависаний
        let noise_3d = Self::sample_3d_noise(x, y, z);
        
        // Смешиваем: чем выше noise_3d_strength, тем больше влияние 3D шума
        let blend = biome.noise_3d_strength;
        height_density * (1.0 - blend * 0.5) + noise_3d * blend * 30.0
    }

    /// 3D шум для создания карнизов и пещер в горах
    fn sample_3d_noise(x: f32, y: f32, z: f32) -> f32 {
        // "Сплющиваем" Y координату для шума, чтобы получить слоистую структуру
        // Это делает скалы похожими на осадочные породы и создает карнизы
        let stretch_y = y * 1.5;
        
        let n1 = noise3d(x * 0.02, stretch_y * 0.02, z * 0.02);
        let n2 = noise3d(x * 0.05, stretch_y * 0.05, z * 0.05) * 0.5;
        
        // Добавляем "сырную" дырчатость для арок
        let caves = noise3d(x * 0.03 + 100.0, y * 0.03, z * 0.03 + 100.0);
        
        let density = n1 + n2;
        
        // Если caves < порога, вырезаем дыру (отрицательная плотность)
        if caves < 0.2 {
            return -1.0;
        }
        
        density
    }

    /// Проверка: есть ли блок в точке (для 3D terrain)
    #[inline]
    pub fn is_solid(x: f32, y: f32, z: f32) -> bool {
        Self::get_3d_density(x, y, z) > 0.0
    }
}

// Глобальные функции для удобства
pub fn get_biome_height(x: f32, z: f32) -> f32 {
    BiomeTerrainGen::get_height(x, z)
}

pub fn get_3d_density(x: f32, y: f32, z: f32) -> f32 {
    BiomeTerrainGen::get_3d_density(x, y, z)
}

pub fn is_solid_3d(x: f32, y: f32, z: f32) -> bool {
    BiomeTerrainGen::is_solid(x, y, z)
}
