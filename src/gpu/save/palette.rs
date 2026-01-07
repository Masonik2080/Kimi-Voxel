// ============================================
// Block Palette - Палитра блоков для сжатия
// ============================================
// Превращает BlockType в компактные индексы 0..N

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::gpu::blocks::BlockType;

/// Палитра блоков для чанка
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPalette {
    /// Список уникальных типов блоков (индекс = ID в палитре)
    blocks: Vec<u8>,
    /// Обратный маппинг: BlockType -> индекс палитры
    #[serde(skip)]
    reverse_map: HashMap<u8, u16>,
}

impl BlockPalette {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            reverse_map: HashMap::new(),
        }
    }

    /// Создать палитру из массива блоков
    pub fn from_blocks(blocks: &[BlockType]) -> Self {
        let mut palette = Self::new();
        for &block in blocks {
            palette.get_or_insert(block);
        }
        palette
    }

    /// Получить индекс блока или добавить новый
    pub fn get_or_insert(&mut self, block: BlockType) -> u16 {
        let block_id = block as u8;
        
        if let Some(&idx) = self.reverse_map.get(&block_id) {
            return idx;
        }
        
        let idx = self.blocks.len() as u16;
        self.blocks.push(block_id);
        self.reverse_map.insert(block_id, idx);
        idx
    }

    /// Получить BlockType по индексу палитры
    pub fn get(&self, index: u16) -> Option<BlockType> {
        self.blocks.get(index as usize).map(|&id| unsafe {
            std::mem::transmute::<u8, BlockType>(id)
        })
    }

    /// Количество уникальных блоков
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Пустая ли палитра
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Восстановить reverse_map после десериализации
    pub fn rebuild_reverse_map(&mut self) {
        self.reverse_map.clear();
        for (idx, &block_id) in self.blocks.iter().enumerate() {
            self.reverse_map.insert(block_id, idx as u16);
        }
    }

    /// Количество бит на индекс (для оптимального хранения)
    pub fn bits_per_index(&self) -> u8 {
        let len = self.blocks.len();
        if len <= 2 { 1 }
        else if len <= 4 { 2 }
        else if len <= 16 { 4 }
        else if len <= 256 { 8 }
        else { 16 }
    }
}

impl Default for BlockPalette {
    fn default() -> Self {
        Self::new()
    }
}
