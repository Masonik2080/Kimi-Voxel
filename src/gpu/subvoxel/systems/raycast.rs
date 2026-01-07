// ============================================
// Raycast System - Рейкаст по субвокселям (ОПТИМИЗИРОВАННЫЙ)
// ============================================

use crate::gpu::blocks::BlockType;
use super::super::components::{SubVoxelPos, SubVoxelLevel, SubVoxelWorld};
use super::super::chunk::{SubVoxelChunkKey, SparseChunkStorage};

/// Результат raycast по субвокселям
#[derive(Clone, Copy, Debug)]
pub struct SubVoxelHit {
    pub pos: SubVoxelPos,
    pub block_type: BlockType,
    pub hit_point: [f32; 3],
    pub hit_normal: [f32; 3],
    pub distance: f32,
}

/// Raycast через субвоксели мира
pub fn subvoxel_raycast(
    world: &SubVoxelWorld,
    origin: [f32; 3],
    direction: [f32; 3],
    max_distance: f32,
    level: SubVoxelLevel,
) -> Option<SubVoxelHit> {
    let mut closest_hit: Option<SubVoxelHit> = None;

    let end = [
        origin[0] + direction[0] * max_distance,
        origin[1] + direction[1] * max_distance,
        origin[2] + direction[2] * max_distance,
    ];

    let min_chunk_x = (origin[0].min(end[0]).floor() as i32).div_euclid(16);
    let max_chunk_x = (origin[0].max(end[0]).ceil() as i32).div_euclid(16);
    let min_chunk_z = (origin[2].min(end[2]).floor() as i32).div_euclid(16);
    let max_chunk_z = (origin[2].max(end[2]).ceil() as i32).div_euclid(16);

    for cx in min_chunk_x..=max_chunk_x {
        for cz in min_chunk_z..=max_chunk_z {
            let key = SubVoxelChunkKey::new(cx, cz);
            if let Some(chunk) = world.get_chunk(&key) {
                if let Some(hit) = raycast_chunk(chunk, cx, cz, origin, direction, max_distance, level) {
                    if closest_hit.is_none() || hit.distance < closest_hit.as_ref().unwrap().distance {
                        closest_hit = Some(hit);
                    }
                }
            }
        }
    }

    closest_hit
}

/// Raycast через один чанк (использует SparseChunkStorage)
fn raycast_chunk(
    chunk: &SparseChunkStorage,
    chunk_x: i32, chunk_z: i32,
    origin: [f32; 3],
    direction: [f32; 3],
    max_distance: f32,
    level: SubVoxelLevel,
) -> Option<SubVoxelHit> {
    let mut closest_hit: Option<SubVoxelHit> = None;
    let base_x = chunk_x * 16;
    let base_z = chunk_z * 16;
    let divisions = level.divisions();

    for (block_key, octree) in chunk.iter_blocks() {
        let (bx, by, bz) = block_key.unpack();
        let block_world_x = base_x + bx as i32;
        let block_world_y = by as i32;
        let block_world_z = base_z + bz as i32;

        // Быстрая проверка AABB блока
        let block_min = [block_world_x as f32, block_world_y as f32, block_world_z as f32];
        let block_max = [block_min[0] + 1.0, block_min[1] + 1.0, block_min[2] + 1.0];
        
        if ray_aabb_intersection(origin, direction, block_min, block_max).is_none() {
            continue;
        }

        // Детальная проверка субвокселей
        for (sx, sy, sz, size, block_type) in octree.iter_solid() {
            let min_x = block_world_x as f32 + sx;
            let min_y = block_world_y as f32 + sy;
            let min_z = block_world_z as f32 + sz;
            let max_x = min_x + size;
            let max_y = min_y + size;
            let max_z = min_z + size;

            if let Some((t, normal)) = ray_aabb_intersection(
                origin, direction,
                [min_x, min_y, min_z],
                [max_x, max_y, max_z],
            ) {
                if t > 0.0 && t < max_distance {
                    if closest_hit.is_none() || t < closest_hit.as_ref().unwrap().distance {
                        let sub_x = ((sx / size) as u8).min(divisions - 1);
                        let sub_y = ((sy / size) as u8).min(divisions - 1);
                        let sub_z = ((sz / size) as u8).min(divisions - 1);

                        closest_hit = Some(SubVoxelHit {
                            pos: SubVoxelPos::new(
                                block_world_x, block_world_y, block_world_z,
                                sub_x, sub_y, sub_z,
                                level,
                            ),
                            block_type,
                            hit_point: [
                                origin[0] + direction[0] * t,
                                origin[1] + direction[1] * t,
                                origin[2] + direction[2] * t,
                            ],
                            hit_normal: normal,
                            distance: t,
                        });
                    }
                }
            }
        }
    }

    closest_hit
}

/// Ray-AABB intersection
fn ray_aabb_intersection(
    origin: [f32; 3],
    direction: [f32; 3],
    aabb_min: [f32; 3],
    aabb_max: [f32; 3],
) -> Option<(f32, [f32; 3])> {
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;
    let mut normal = [0.0f32; 3];

    for i in 0..3 {
        if direction[i].abs() < 1e-8 {
            if origin[i] < aabb_min[i] || origin[i] > aabb_max[i] {
                return None;
            }
        } else {
            let inv_d = 1.0 / direction[i];
            let mut t1 = (aabb_min[i] - origin[i]) * inv_d;
            let mut t2 = (aabb_max[i] - origin[i]) * inv_d;

            let mut n = [0.0f32; 3];
            n[i] = -1.0;

            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
                n[i] = 1.0;
            }

            if t1 > t_min {
                t_min = t1;
                normal = n;
            }
            t_max = t_max.min(t2);

            if t_min > t_max {
                return None;
            }
        }
    }

    Some((t_min, normal))
}
