// ============================================
// Placement System - Размещение субвокселей
// ============================================

use super::super::components::{SubVoxelPos, SubVoxelLevel};
use super::raycast::SubVoxelHit;

/// Вычислить позицию субвокселя из мировых координат
pub fn world_to_subvoxel_pos(
    world_x: f32, world_y: f32, world_z: f32,
    level: SubVoxelLevel,
) -> SubVoxelPos {
    let size = level.size();
    
    // Базовый блок
    let block_x = world_x.floor() as i32;
    let block_y = world_y.floor() as i32;
    let block_z = world_z.floor() as i32;
    
    // Позиция внутри блока
    let local_x = world_x - block_x as f32;
    let local_y = world_y - block_y as f32;
    let local_z = world_z - block_z as f32;
    
    // Индекс субвокселя
    let divisions = level.divisions();
    let sub_x = ((local_x / size).floor() as u8).min(divisions - 1);
    let sub_y = ((local_y / size).floor() as u8).min(divisions - 1);
    let sub_z = ((local_z / size).floor() as u8).min(divisions - 1);
    
    SubVoxelPos::new(block_x, block_y, block_z, sub_x, sub_y, sub_z, level)
}

/// Вычислить позицию для размещения субвокселя рядом с hit
pub fn placement_pos_from_hit(hit: &SubVoxelHit, level: SubVoxelLevel) -> SubVoxelPos {
    let size = level.size();
    // Смещаем точку попадания немного в направлении нормали
    let place_x = hit.hit_point[0] + hit.hit_normal[0] * (size * 0.5);
    let place_y = hit.hit_point[1] + hit.hit_normal[1] * (size * 0.5);
    let place_z = hit.hit_point[2] + hit.hit_normal[2] * (size * 0.5);
    
    world_to_subvoxel_pos(place_x, place_y, place_z, level)
}
