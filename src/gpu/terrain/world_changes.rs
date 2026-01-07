// ============================================
// World Changes - Хранение изменений мира
// ============================================
// Хранит сломанные/поставленные блоки поверх процедурной генерации

use std::collections::HashMap;
use crate::gpu::blocks::{BlockType, AIR};

/// Ключ для блока в мире
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    
    pub fn from_array(arr: [i32; 3]) -> Self {
        Self { x: arr[0], y: arr[1], z: arr[2] }
    }
    
    /// Получить ключ чанка для этого блока
    pub fn chunk_key(&self) -> (i32, i32) {
        let chunk_x = if self.x >= 0 { self.x / 16 } else { (self.x - 15) / 16 };
        let chunk_z = if self.z >= 0 { self.z / 16 } else { (self.z - 15) / 16 };
        (chunk_x, chunk_z)
    }
}

/// Хранилище изменений мира
pub struct WorldChanges {
    /// Изменённые блоки: позиция -> новый тип (Air = сломан)
    changes: HashMap<BlockPos, BlockType>,
    
    /// Чанки которые нужно перегенерировать
    dirty_chunks: Vec<(i32, i32)>,
    
    /// Версия изменений (инкрементируется при каждом изменении)
    version: u64,
}

impl WorldChanges {
    pub fn new() -> Self {
        Self {
            changes: HashMap::new(),
            dirty_chunks: Vec::new(),
            version: 0,
        }
    }
    
    /// Получить версию изменений
    pub fn version(&self) -> u64 {
        self.version
    }
    
    /// Установить блок (или удалить если Air)
    pub fn set_block(&mut self, pos: BlockPos, block_type: BlockType) {
        self.changes.insert(pos, block_type);
        self.version += 1;
        
        // Помечаем чанк как грязный
        let chunk_key = pos.chunk_key();
        if !self.dirty_chunks.contains(&chunk_key) {
            self.dirty_chunks.push(chunk_key);
        }
    }
    
    /// Сломать блок (установить Air)
    pub fn break_block(&mut self, x: i32, y: i32, z: i32) {
        self.set_block(BlockPos::new(x, y, z), AIR);
    }
    
    /// Получить изменённый блок (если есть)
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<BlockType> {
        self.changes.get(&BlockPos::new(x, y, z)).copied()
    }
    
    /// Проверить есть ли изменение для блока
    pub fn has_change(&self, x: i32, y: i32, z: i32) -> bool {
        self.changes.contains_key(&BlockPos::new(x, y, z))
    }
    
    /// Получить и очистить список грязных чанков
    pub fn take_dirty_chunks(&mut self) -> Vec<(i32, i32)> {
        std::mem::take(&mut self.dirty_chunks)
    }
    
    /// Есть ли грязные чанки
    pub fn has_dirty_chunks(&self) -> bool {
        !self.dirty_chunks.is_empty()
    }
    
    /// Количество изменений
    pub fn change_count(&self) -> usize {
        self.changes.len()
    }
    
    /// Получить копию всех изменений (для передачи в генератор)
    pub fn get_all_changes_copy(&self) -> HashMap<BlockPos, BlockType> {
        self.changes.clone()
    }
    
    /// Получить изменения только для конкретного чанка
    pub fn get_changes_for_chunk(&self, chunk_x: i32, chunk_z: i32, chunk_size: i32) -> HashMap<BlockPos, BlockType> {
        let min_x = chunk_x * chunk_size;
        let max_x = min_x + chunk_size;
        let min_z = chunk_z * chunk_size;
        let max_z = min_z + chunk_size;
        
        self.changes
            .iter()
            .filter(|(pos, _)| {
                pos.x >= min_x && pos.x < max_x && pos.z >= min_z && pos.z < max_z
            })
            .map(|(pos, block)| (*pos, *block))
            .collect()
    }
}

impl Default for WorldChanges {
    fn default() -> Self {
        Self::new()
    }
}
