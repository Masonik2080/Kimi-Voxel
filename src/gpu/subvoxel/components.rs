// ============================================
// SubVoxel Components - ECS компоненты (ОПТИМИЗИРОВАННЫЕ)
// ============================================
//
// Использует:
// - SparseChunkStorage вместо ChunkSubVoxelStorage (O(N) память)
// - CompactOctree вместо LinearOctree (4 байта на узел)

use std::collections::HashMap;
use crate::gpu::blocks::{BlockType, AIR};
use super::chunk::{SubVoxelChunkKey, SparseChunkStorage};

/// Уровень детализации субвокселя
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SubVoxelLevel {
    /// Полный блок 1x1x1
    Full = 0,
    /// Половинный блок 1/2 (8 в одном полном)
    Half = 1,
    /// Четвертинный блок 1/4 (64 в одном полном)
    Quarter = 2,
}

impl SubVoxelLevel {
    #[inline]
    pub fn size(&self) -> f32 {
        match self {
            SubVoxelLevel::Full => 1.0,
            SubVoxelLevel::Half => 0.5,
            SubVoxelLevel::Quarter => 0.25,
        }
    }

    #[inline]
    pub fn divisions(&self) -> u8 {
        match self {
            SubVoxelLevel::Full => 1,
            SubVoxelLevel::Half => 2,
            SubVoxelLevel::Quarter => 4,
        }
    }

    #[inline]
    pub fn depth(&self) -> u8 {
        match self {
            SubVoxelLevel::Full => 0,
            SubVoxelLevel::Half => 1,
            SubVoxelLevel::Quarter => 2,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SubVoxelLevel::Full => SubVoxelLevel::Half,
            SubVoxelLevel::Half => SubVoxelLevel::Quarter,
            SubVoxelLevel::Quarter => SubVoxelLevel::Full,
        }
    }
}

impl Default for SubVoxelLevel {
    fn default() -> Self {
        SubVoxelLevel::Quarter
    }
}

/// Позиция субвокселя в мире
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SubVoxelPos {
    pub block_x: i32,
    pub block_y: i32,
    pub block_z: i32,
    pub sub_x: u8,
    pub sub_y: u8,
    pub sub_z: u8,
    pub level: SubVoxelLevel,
}

impl SubVoxelPos {
    #[inline]
    pub fn new(
        block_x: i32, block_y: i32, block_z: i32,
        sub_x: u8, sub_y: u8, sub_z: u8,
        level: SubVoxelLevel,
    ) -> Self {
        Self { block_x, block_y, block_z, sub_x, sub_y, sub_z, level }
    }

    #[inline]
    pub fn world_min(&self) -> [f32; 3] {
        let size = self.level.size();
        [
            self.block_x as f32 + self.sub_x as f32 * size,
            self.block_y as f32 + self.sub_y as f32 * size,
            self.block_z as f32 + self.sub_z as f32 * size,
        ]
    }

    #[inline]
    pub fn chunk_key(&self) -> SubVoxelChunkKey {
        SubVoxelChunkKey::from_block_pos(self.block_x, self.block_z)
    }

    #[inline]
    pub fn local_block(&self) -> (u8, u8, u8) {
        let local_x = self.block_x.rem_euclid(16) as u8;
        let local_z = self.block_z.rem_euclid(16) as u8;
        (local_x, self.block_y as u8, local_z)
    }
}

// ============================================
// SubVoxelWorld - ОПТИМИЗИРОВАННАЯ ВЕРСИЯ
// ============================================

/// Глобальное хранилище субвокселей (ECS Resource)
/// Использует SparseChunkStorage для O(N) памяти
pub struct SubVoxelWorld {
    /// Разреженные хранилища по чанкам
    chunks: HashMap<SubVoxelChunkKey, SparseChunkStorage>,
    /// Грязные чанки
    dirty_chunks: Vec<SubVoxelChunkKey>,
    /// Глобальная версия
    version: u64,
}

impl SubVoxelWorld {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            dirty_chunks: Vec::new(),
            version: 0,
        }
    }

    /// Установить субвоксель
    pub fn set(&mut self, pos: SubVoxelPos, block_type: BlockType) {
        let chunk_key = pos.chunk_key();
        let (local_x, local_y, local_z) = pos.local_block();
        
        let chunk = self.chunks.entry(chunk_key).or_insert_with(SparseChunkStorage::new);
        chunk.set(
            local_x, local_y, local_z,
            pos.sub_x, pos.sub_y, pos.sub_z,
            pos.level.depth(),
            block_type,
        );

        if !self.dirty_chunks.contains(&chunk_key) {
            self.dirty_chunks.push(chunk_key);
        }

        if chunk.is_empty() {
            self.chunks.remove(&chunk_key);
        }

        self.version += 1;
    }

    /// Получить субвоксель
    #[inline]
    pub fn get(&self, pos: &SubVoxelPos) -> Option<BlockType> {
        let chunk_key = pos.chunk_key();
        let (local_x, local_y, local_z) = pos.local_block();
        
        self.chunks.get(&chunk_key)?.get(
            local_x, local_y, local_z,
            pos.sub_x, pos.sub_y, pos.sub_z,
            pos.level.depth(),
        )
    }

    /// Удалить субвоксель
    #[inline]
    pub fn remove(&mut self, pos: &SubVoxelPos) {
        self.set(*pos, AIR);
    }

    /// Получить и очистить грязные чанки
    #[inline]
    pub fn take_dirty_chunks(&mut self) -> Vec<SubVoxelChunkKey> {
        std::mem::take(&mut self.dirty_chunks)
    }

    /// Получить хранилище чанка
    #[inline]
    pub fn get_chunk(&self, key: &SubVoxelChunkKey) -> Option<&SparseChunkStorage> {
        self.chunks.get(key)
    }

    /// Итератор по чанкам
    #[inline]
    pub fn iter_chunks(&self) -> impl Iterator<Item = (&SubVoxelChunkKey, &SparseChunkStorage)> {
        self.chunks.iter()
    }

    #[inline]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    #[inline]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Общее использование памяти (байт)
    pub fn memory_usage(&self) -> usize {
        self.chunks.values().map(|c| c.memory_usage()).sum()
    }

    /// Проверка коллизии AABB
    pub fn check_aabb_collision(
        &self,
        min_x: f32, min_y: f32, min_z: f32,
        max_x: f32, max_y: f32, max_z: f32,
    ) -> bool {
        let min_chunk_x = (min_x.floor() as i32).div_euclid(16);
        let max_chunk_x = (max_x.ceil() as i32).div_euclid(16);
        let min_chunk_z = (min_z.floor() as i32).div_euclid(16);
        let max_chunk_z = (max_z.ceil() as i32).div_euclid(16);

        for cx in min_chunk_x..=max_chunk_x {
            for cz in min_chunk_z..=max_chunk_z {
                let key = SubVoxelChunkKey::new(cx, cz);
                if let Some(chunk) = self.chunks.get(&key) {
                    if check_chunk_aabb_collision(chunk, cx, cz, min_x, min_y, min_z, max_x, max_y, max_z) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl Default for SubVoxelWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// Проверка коллизии AABB с чанком
fn check_chunk_aabb_collision(
    chunk: &SparseChunkStorage,
    chunk_x: i32, chunk_z: i32,
    min_x: f32, min_y: f32, min_z: f32,
    max_x: f32, max_y: f32, max_z: f32,
) -> bool {
    let base_x = chunk_x * 16;
    let base_z = chunk_z * 16;

    for (key, octree) in chunk.iter_blocks() {
        let (bx, by, bz) = key.unpack();
        let block_x = (base_x + bx as i32) as f32;
        let block_y = by as f32;
        let block_z = (base_z + bz as i32) as f32;

        // Быстрая проверка AABB блока
        if max_x <= block_x || min_x >= block_x + 1.0 ||
           max_y <= block_y || min_y >= block_y + 1.0 ||
           max_z <= block_z || min_z >= block_z + 1.0 {
            continue;
        }

        // Детальная проверка через октодерево
        for (sx, sy, sz, size, _bt) in octree.iter_solid() {
            let sv_min_x = block_x + sx;
            let sv_min_y = block_y + sy;
            let sv_min_z = block_z + sz;
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
    false
}
