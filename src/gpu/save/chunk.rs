// ============================================
// Compressed Chunk - Сжатый чанк для сохранения
// ============================================
// Использует палитру для минимизации размера

use serde::{Serialize, Deserialize};
use crate::gpu::blocks::{BlockType, AIR};
use super::palette::BlockPalette;

/// Размер секции чанка (16x16x16)
pub const SECTION_SIZE: usize = 16;
pub const SECTION_VOLUME: usize = SECTION_SIZE * SECTION_SIZE * SECTION_SIZE;

/// Сжатый чанк с палитрой
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedChunk {
    /// Координаты чанка (X, Z)
    pub chunk_x: i32,
    pub chunk_z: i32,
    /// Секции чанка по высоте (каждая 16x16x16)
    pub sections: Vec<CompressedSection>,
}

/// Сжатая секция 16x16x16
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedSection {
    /// Y-координата секции (в блоках, кратно 16)
    pub section_y: i32,
    /// Палитра блоков для этой секции
    pub palette: BlockPalette,
    /// Индексы блоков (ссылки на палитру)
    pub indices: Vec<u16>,
}

impl CompressedChunk {
    pub fn new(chunk_x: i32, chunk_z: i32) -> Self {
        Self {
            chunk_x,
            chunk_z,
            sections: Vec::new(),
        }
    }

    /// Добавить секцию
    pub fn add_section(&mut self, section: CompressedSection) {
        self.sections.push(section);
    }

    /// Получить секцию по Y
    pub fn get_section(&self, section_y: i32) -> Option<&CompressedSection> {
        self.sections.iter().find(|s| s.section_y == section_y)
    }

    /// Проверить пустой ли чанк
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }
}

impl CompressedSection {
    /// Создать секцию из массива блоков 16x16x16
    pub fn from_blocks(section_y: i32, blocks: &[BlockType; SECTION_VOLUME]) -> Self {
        let mut palette = BlockPalette::new();
        let mut indices = Vec::with_capacity(SECTION_VOLUME);

        // Первый проход: строим палитру и индексы
        for &block in blocks {
            let idx = palette.get_or_insert(block);
            indices.push(idx);
        }

        Self {
            section_y,
            palette,
            indices,
        }
    }

    /// Распаковать секцию в массив блоков
    pub fn decompress(&self) -> [BlockType; SECTION_VOLUME] {
        let mut blocks = [AIR; SECTION_VOLUME];
        
        for (i, &idx) in self.indices.iter().enumerate() {
            if let Some(block) = self.palette.get(idx) {
                blocks[i] = block;
            }
        }
        
        blocks
    }

    /// Проверить содержит ли секция только воздух
    pub fn is_air_only(&self) -> bool {
        self.palette.len() == 1 && self.palette.get(0) == Some(AIR)
    }

    /// Восстановить палитру после десериализации
    pub fn rebuild_palette(&mut self) {
        self.palette.rebuild_reverse_map();
    }
}

/// Индекс блока внутри секции
#[inline]
pub fn section_index(x: usize, y: usize, z: usize) -> usize {
    y * SECTION_SIZE * SECTION_SIZE + z * SECTION_SIZE + x
}

/// Координаты из индекса секции
#[inline]
pub fn index_to_coords(index: usize) -> (usize, usize, usize) {
    let y = index / (SECTION_SIZE * SECTION_SIZE);
    let rem = index % (SECTION_SIZE * SECTION_SIZE);
    let z = rem / SECTION_SIZE;
    let x = rem % SECTION_SIZE;
    (x, y, z)
}
