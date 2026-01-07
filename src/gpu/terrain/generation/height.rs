// ============================================
// Height Map - Генерация карты высот с биомами
// ============================================

use crate::gpu::biomes::{BiomeTerrainGen, get_biome_height};

/// Базовая высота террейна (теперь с учётом биомов)
#[inline]
pub fn get_height(x: f32, z: f32) -> f32 {
    get_biome_height(x, z)
}

/// Высота для LOD (центрированная)
#[inline]
pub fn get_lod_height(x: f32, z: f32, scale: i32) -> f32 {
    if scale == 1 {
        return get_height(x, z);
    }
    let half = scale as f32 * 0.5;
    get_height(x + half, z + half)
}

/// 3D density для гор с карнизами
#[inline]
pub fn get_3d_density(x: f32, y: f32, z: f32) -> f32 {
    BiomeTerrainGen::get_3d_density(x, y, z)
}

/// Проверка твёрдости блока в 3D (для гор)
#[inline]
pub fn is_solid_3d(x: f32, y: f32, z: f32) -> bool {
    BiomeTerrainGen::is_solid(x, y, z)
}
