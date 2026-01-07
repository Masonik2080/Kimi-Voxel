// ============================================
// Chunk SubVoxel Storage - Плоский массив для O(1) доступа
// ============================================
//
// Вместо HashMap используем Vec<Option<LinearOctree>> с индексацией:
// index = y * 256 + z * 16 + x
// Это дает O(1) доступ вместо хеширования.

use crate::gpu::blocks::{BlockType, AIR};
use super::super::octree::LinearOctree;

/// Размер чанка
pub const CHUNK_SIZE: usize = 16;
/// Высота мира (для Y координаты)
pub const CHUNK_HEIGHT: usize = 256;
/// Общий размер массива
pub const STORAGE_SIZE: usize = CHUNK_HEIGHT * CHUNK_SIZE * CHUNK_SIZE;

/// Ключ блока внутри чанка (локальные координаты)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LocalBlockKey {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl LocalBlockKey {
    #[inline]
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        Self { x, y, z }
    }

    /// Из мировых координат блока и координат чанка
    #[inline]
    pub fn from_world(block_x: i32, block_y: i32, block_z: i32, chunk_x: i32, chunk_z: i32) -> Self {
        let local_x = (block_x - chunk_x * 16).rem_euclid(16) as u8;
        let local_z = (block_z - chunk_z * 16).rem_euclid(16) as u8;
        Self {
            x: local_x,
            y: block_y as u8,
            z: local_z,
        }
    }

    /// Индекс в плоском массиве
    #[inline]
    pub fn to_index(&self) -> usize {
        (self.y as usize) * 256 + (self.z as usize) * 16 + (self.x as usize)
    }

    /// Из индекса в плоском массиве
    #[inline]
    pub fn from_index(index: usize) -> Self {
        let y = (index / 256) as u8;
        let z = ((index % 256) / 16) as u8;
        let x = (index % 16) as u8;
        Self { x, y, z }
    }
}

/// Хранилище субвокселей для одного чанка
/// Использует плоский массив для O(1) доступа
pub struct ChunkSubVoxelStorage {
    /// Плоский массив октодеревьев (None = нет субвокселей в этом блоке)
    /// Индексация: y * 256 + z * 16 + x
    blocks: Vec<Option<LinearOctree>>,
    /// Количество непустых блоков (для быстрой проверки is_empty)
    block_count: usize,
    /// Список индексов непустых блоков (для быстрой итерации)
    occupied_indices: Vec<usize>,
    /// Флаг "грязности"
    dirty: bool,
    /// Версия для отслеживания изменений
    version: u64,
}

impl ChunkSubVoxelStorage {
    pub fn new() -> Self {
        Self {
            blocks: vec![None; STORAGE_SIZE],
            block_count: 0,
            occupied_indices: Vec::with_capacity(64),
            dirty: false,
            version: 0,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.block_count == 0
    }

    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    #[inline]
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    #[inline]
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    #[inline]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Индекс в плоском массиве
    #[inline]
    fn index(x: u8, y: u8, z: u8) -> usize {
        (y as usize) * 256 + (z as usize) * 16 + (x as usize)
    }

    /// Установить субвоксель
    pub fn set(
        &mut self,
        local_x: u8, local_y: u8, local_z: u8,
        sub_x: u8, sub_y: u8, sub_z: u8,
        divisions: u8,
        block_type: BlockType,
    ) {
        let depth = match divisions {
            1 => 0,
            2 => 1,
            4 => 2,
            _ => return,
        };

        let idx = Self::index(local_x, local_y, local_z);

        if block_type == AIR {
            // Удаление
            if let Some(ref mut octree) = self.blocks[idx] {
                octree.remove_discrete(sub_x, sub_y, sub_z, depth);
                if octree.is_empty() {
                    self.blocks[idx] = None;
                    self.block_count -= 1;
                    self.occupied_indices.retain(|&i| i != idx);
                }
            }
        } else {
            // Добавление
            let was_empty = self.blocks[idx].is_none();
            let octree = self.blocks[idx].get_or_insert_with(LinearOctree::new);
            octree.set_discrete(sub_x, sub_y, sub_z, depth, block_type);
            
            if was_empty {
                self.block_count += 1;
                self.occupied_indices.push(idx);
            }
        }

        self.dirty = true;
        self.version += 1;
    }

    /// Получить субвоксель - O(1)
    #[inline]
    pub fn get(
        &self,
        local_x: u8, local_y: u8, local_z: u8,
        sub_x: u8, sub_y: u8, sub_z: u8,
        divisions: u8,
    ) -> Option<BlockType> {
        let depth = match divisions {
            1 => 0,
            2 => 1,
            4 => 2,
            _ => return None,
        };

        let idx = Self::index(local_x, local_y, local_z);
        self.blocks[idx].as_ref()?.get_discrete(sub_x, sub_y, sub_z, depth)
    }

    /// Получить октодерево для блока - O(1)
    #[inline]
    pub fn get_block_octree(&self, local_x: u8, local_y: u8, local_z: u8) -> Option<&LinearOctree> {
        let idx = Self::index(local_x, local_y, local_z);
        self.blocks[idx].as_ref()
    }

    /// Получить мутабельное октодерево - O(1)
    #[inline]
    pub fn get_block_octree_mut(&mut self, local_x: u8, local_y: u8, local_z: u8) -> Option<&mut LinearOctree> {
        let idx = Self::index(local_x, local_y, local_z);
        self.blocks[idx].as_mut()
    }

    /// Удалить все субвоксели в блоке
    pub fn clear_block(&mut self, local_x: u8, local_y: u8, local_z: u8) {
        let idx = Self::index(local_x, local_y, local_z);
        if self.blocks[idx].take().is_some() {
            self.block_count -= 1;
            self.occupied_indices.retain(|&i| i != idx);
            self.dirty = true;
            self.version += 1;
        }
    }

    /// Количество блоков с субвокселями
    #[inline]
    pub fn block_count(&self) -> usize {
        self.block_count
    }

    /// Общее количество субвокселей
    pub fn subvoxel_count(&self) -> usize {
        self.occupied_indices.iter()
            .filter_map(|&idx| self.blocks[idx].as_ref())
            .map(|o| o.count_solid())
            .sum()
    }

    /// Быстрый итератор по непустым блокам (использует occupied_indices)
    pub fn iter_blocks(&self) -> impl Iterator<Item = (LocalBlockKey, &LinearOctree)> {
        self.occupied_indices.iter()
            .filter_map(move |&idx| {
                self.blocks[idx].as_ref().map(|octree| {
                    (LocalBlockKey::from_index(idx), octree)
                })
            })
    }

    /// Проверка коллизии AABB с субвокселями чанка
    pub fn check_aabb_collision(
        &self,
        chunk_x: i32, chunk_z: i32,
        min_x: f32, min_y: f32, min_z: f32,
        max_x: f32, max_y: f32, max_z: f32,
    ) -> bool {
        let base_x = chunk_x * 16;
        let base_z = chunk_z * 16;

        // Определяем диапазон блоков для проверки
        let local_min_x = ((min_x - base_x as f32).floor().max(0.0) as u8).min(15);
        let local_max_x = ((max_x - base_x as f32).ceil().max(0.0) as u8).min(15);
        let local_min_y = (min_y.floor().max(0.0) as u8);
        let local_max_y = (max_y.ceil().max(0.0) as u8).min(255);
        let local_min_z = ((min_z - base_z as f32).floor().max(0.0) as u8).min(15);
        let local_max_z = ((max_z - base_z as f32).ceil().max(0.0) as u8).min(15);

        // Проверяем только блоки в диапазоне
        for y in local_min_y..=local_max_y {
            for z in local_min_z..=local_max_z {
                for x in local_min_x..=local_max_x {
                    let idx = Self::index(x, y, z);
                    let Some(octree) = &self.blocks[idx] else { continue };

                    let block_world_x = (base_x + x as i32) as f32;
                    let block_world_y = y as f32;
                    let block_world_z = (base_z + z as i32) as f32;

                    // Детальная проверка субвокселей
                    for (sx, sy, sz, size, _block_type) in octree.iter_solid() {
                        let sv_min_x = block_world_x + sx;
                        let sv_min_y = block_world_y + sy;
                        let sv_min_z = block_world_z + sz;
                        let sv_max_x = sv_min_x + size;
                        let sv_max_y = sv_min_y + size;
                        let sv_max_z = sv_min_z + size;

                        if max_x > sv_min_x && min_x < sv_max_x &&
                           max_y > sv_min_y && min_y < sv_max_y &&
                           max_z > sv_min_z && min_z < sv_max_z {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Raycast через блоки чанка - O(log N) для каждого блока
    pub fn raycast_blocks(
        &self,
        chunk_x: i32, chunk_z: i32,
        origin: [f32; 3],
        direction: [f32; 3],
        max_distance: f32,
    ) -> Option<RaycastHit> {
        let base_x = chunk_x * 16;
        let base_z = chunk_z * 16;

        let mut closest: Option<RaycastHit> = None;

        // Используем occupied_indices для быстрой итерации
        for &idx in &self.occupied_indices {
            let Some(octree) = &self.blocks[idx] else { continue };
            
            let key = LocalBlockKey::from_index(idx);
            let block_world_x = (base_x + key.x as i32) as f32;
            let block_world_y = key.y as f32;
            let block_world_z = (base_z + key.z as i32) as f32;

            // Быстрая проверка - луч пересекает блок?
            let current_max = closest.as_ref().map(|c| c.distance).unwrap_or(max_distance);
            if !ray_intersects_block(origin, direction, block_world_x, block_world_y, block_world_z, current_max) {
                continue;
            }

            // Трансформируем луч в локальные координаты блока [0, 1)
            let local_origin = [
                origin[0] - block_world_x,
                origin[1] - block_world_y,
                origin[2] - block_world_z,
            ];

            // O(log N) raycast через октодерево
            if let Some(hit) = octree.raycast(local_origin, direction, current_max) {
                let world_hit_point = [
                    block_world_x + local_origin[0] + direction[0] * hit.t,
                    block_world_y + local_origin[1] + direction[1] * hit.t,
                    block_world_z + local_origin[2] + direction[2] * hit.t,
                ];

                if closest.is_none() || hit.t < closest.as_ref().unwrap().distance {
                    closest = Some(RaycastHit {
                        block_key: key,
                        sub_pos: [hit.x, hit.y, hit.z],
                        size: hit.size,
                        block_type: hit.block_type,
                        hit_point: world_hit_point,
                        hit_normal: hit.normal,
                        distance: hit.t,
                    });
                }
            }
        }

        closest
    }
}

impl Default for ChunkSubVoxelStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Результат raycast
#[derive(Clone, Copy, Debug)]
pub struct RaycastHit {
    pub block_key: LocalBlockKey,
    pub sub_pos: [f32; 3],
    pub size: f32,
    pub block_type: BlockType,
    pub hit_point: [f32; 3],
    pub hit_normal: [f32; 3],
    pub distance: f32,
}

/// Быстрая проверка пересечения луча с блоком 1x1x1
#[inline]
fn ray_intersects_block(
    origin: [f32; 3],
    direction: [f32; 3],
    bx: f32, by: f32, bz: f32,
    max_dist: f32,
) -> bool {
    ray_aabb_intersection(
        origin, direction,
        [bx, by, bz],
        [bx + 1.0, by + 1.0, bz + 1.0],
    ).map(|(t, _)| t >= 0.0 && t <= max_dist).unwrap_or(false)
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
