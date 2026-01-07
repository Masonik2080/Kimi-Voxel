// ============================================
// Terrain Colors - Цвета по биому
// ============================================

use crate::gpu::blocks::get_face_colors;
use crate::gpu::biomes::biome_selector;

/// Получить цвет террейна по координатам (использует биом)
#[inline]
pub fn get_color(x: f32, z: f32, is_top: bool) -> [f32; 3] {
    let biome = biome_selector().get_biome_def(x as i32, z as i32);
    let block = biome.surface_block;
    
    let (top_color, side_color) = get_face_colors(block);
    if is_top { top_color } else { side_color }
}

/// Старая версия для совместимости (deprecated)
#[inline]
pub fn get_color_by_height(height: f32, is_top: bool) -> [f32; 3] {
    use crate::gpu::blocks::worldgen_blocks;
    let blocks = worldgen_blocks();
    let block = blocks.surface_block(height);
    
    let (top_color, side_color) = get_face_colors(block);
    if is_top { top_color } else { side_color }
}
