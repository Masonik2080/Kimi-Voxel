// ============================================
// Sparse Chunk Storage - Разреженное хранение субвокселей
// ============================================
//
// Вместо Vec<Option<LinearOctree>> размером 65536 элементов (~3.5 МБ)
// используем HashMap с интовыми ключами. Память O(N) где N = занятые блоки.

use std::collections::HashMap;
use crate::gpu::blocks::{BlockType, AIR};
use super::super::octree::CompactOctree;

/// Упакованный ключ блока внутри чанка (20 бит: 4+8+4 для x,y,z)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackedBlockKey(u32);

impl PackedBlockKey {
    #[inline]
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        debug_assert!(x < 16 && z < 16);
        Self(((y as u32) << 8) | ((z as u32) << 4) | (x as u32))
    }

    #[inline]
    pub fn unpack(self) -> (u8, u8, u8) {
        let x = (self.0 & 0xF) as u8;
        let z = ((self.0 >> 4) & 0xF) as u8;
        let y = ((self.0 >> 8) & 0xFF) as u8;
        (x, y, z)
    }

    #[inline]
    pub fn x(self) -> u8 { (self.0 & 0xF) as u8 }
    
    #[inline]
    pub fn y(self) -> u8 { ((self.0 >> 8) & 0xFF) as u8 }
    
    #[inline]
    pub fn z(self) -> u8 { ((self.0 >> 4) & 0xF) as u8 }
}

/// Разреженное хранилище субвокселей для чанка
/// Память: O(N) где N = количество блоков с субвокселями
/// Типичный чанк: 10-100 блоков = 1-10 КБ вместо 3.5 МБ
pub struct SparseChunkStorage {
    /// HashMap вместо плоского массива
    blocks: HashMap<PackedBlockKey, CompactOctree>,
    /// Версия для отслеживания изменений
    version: u64,
    /// Флаг грязности
    dirty: bool,
    /// Кэш min/max Y для быстрого доступа
    min_y: u8,
    max_y: u8,
}

impl SparseChunkStorage {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::with_capacity(16), // Начинаем с малого
            version: 0,
            dirty: false,
            min_y: 255,
            max_y: 0,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    #[inline]
    pub fn block_count(&self) -> usize {
        self.blocks.len()
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
    pub fn version(&self) -> u64 {
        self.version
    }

    #[inline]
    pub fn y_range(&self) -> (u8, u8) {
        (self.min_y, self.max_y)
    }

    /// Установить субвоксель
    pub fn set(
        &mut self,
        block_x: u8, block_y: u8, block_z: u8,
        sub_x: u8, sub_y: u8, sub_z: u8,
        depth: u8,
        block_type: BlockType,
    ) {
        let key = PackedBlockKey::new(block_x, block_y, block_z);

        if block_type == AIR {
            // Удаление
            if let Some(octree) = self.blocks.get_mut(&key) {
                octree.remove(sub_x, sub_y, sub_z, depth);
                if octree.is_empty() {
                    self.blocks.remove(&key);
                    self.update_y_bounds();
                }
            }
        } else {
            // Добавление
            let octree = self.blocks.entry(key).or_insert_with(CompactOctree::new);
            octree.set(sub_x, sub_y, sub_z, depth, block_type);
            
            // Обновляем Y bounds
            self.min_y = self.min_y.min(block_y);
            self.max_y = self.max_y.max(block_y);
        }

        self.dirty = true;
        self.version += 1;
    }

    /// Получить субвоксель
    #[inline]
    pub fn get(
        &self,
        block_x: u8, block_y: u8, block_z: u8,
        sub_x: u8, sub_y: u8, sub_z: u8,
        depth: u8,
    ) -> Option<BlockType> {
        let key = PackedBlockKey::new(block_x, block_y, block_z);
        self.blocks.get(&key)?.get(sub_x, sub_y, sub_z, depth)
    }

    /// Получить октодерево блока
    #[inline]
    pub fn get_block(&self, block_x: u8, block_y: u8, block_z: u8) -> Option<&CompactOctree> {
        let key = PackedBlockKey::new(block_x, block_y, block_z);
        self.blocks.get(&key)
    }

    /// Итератор по занятым блокам
    #[inline]
    pub fn iter_blocks(&self) -> impl Iterator<Item = (PackedBlockKey, &CompactOctree)> {
        self.blocks.iter().map(|(&k, v)| (k, v))
    }

    /// Проверка solid в точке (для culling)
    #[inline]
    pub fn is_solid_at(&self, block_x: u8, block_y: u8, block_z: u8, sub_x: u8, sub_y: u8, sub_z: u8, depth: u8) -> bool {
        self.get(block_x, block_y, block_z, sub_x, sub_y, sub_z, depth).is_some()
    }

    /// Обновить Y bounds после удаления
    fn update_y_bounds(&mut self) {
        if self.blocks.is_empty() {
            self.min_y = 255;
            self.max_y = 0;
            return;
        }

        self.min_y = 255;
        self.max_y = 0;
        for key in self.blocks.keys() {
            let y = key.y();
            self.min_y = self.min_y.min(y);
            self.max_y = self.max_y.max(y);
        }
    }

    /// Память в байтах (приблизительно)
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + 
        self.blocks.len() * (std::mem::size_of::<PackedBlockKey>() + std::mem::size_of::<CompactOctree>())
    }
}

impl Default for SparseChunkStorage {
    fn default() -> Self {
        Self::new()
    }
}
