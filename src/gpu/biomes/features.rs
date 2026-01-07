// ============================================
// Biome Features - Генерация структур (деревья)
// ============================================

use std::collections::HashMap;
use crate::gpu::blocks::{BlockType, AIR, OAK_LOG, OAK_LEAVES, BIRCH_LOG, BIRCH_LEAVES, SPRUCE_LOG, SPRUCE_LEAVES};
use crate::gpu::terrain::voxel::constants::{CHUNK_SIZE, MIN_HEIGHT, WORLD_HEIGHT};
use crate::gpu::terrain::BlockPos;

/// Тип дерева
#[derive(Clone, Copy)]
pub enum TreeType {
    Oak,
    Birch,
    Spruce,
}

/// Данные для размещения субвокселя листвы (для экспорта)
#[derive(Clone, Copy)]
pub struct LeafSubVoxel {
    pub world_x: i32,
    pub world_y: i32,
    pub world_z: i32,
    pub block_type: BlockType,
}

/// Хелпер для безопасной записи в массив блоков чанка
pub struct ChunkWriter<'a> {
    blocks: &'a mut Vec<BlockType>,
    world_changes: Option<&'a HashMap<BlockPos, BlockType>>,
    base_x: i32,
    base_z: i32,
    /// Позиции блоков листвы для последующей конвертации в субвоксели
    pub leaf_positions: Vec<LeafSubVoxel>,
}

impl<'a> ChunkWriter<'a> {
    pub fn new(
        blocks: &'a mut Vec<BlockType>,
        world_changes: Option<&'a HashMap<BlockPos, BlockType>>,
        base_x: i32,
        base_z: i32,
    ) -> Self {
        Self { 
            blocks, 
            world_changes, 
            base_x, 
            base_z,
            leaf_positions: Vec::new(),
        }
    }

    /// Безопасная установка блока
    pub fn set_block(&mut self, lx: i32, y: i32, lz: i32, block: BlockType) {
        if lx < 0 || lx >= CHUNK_SIZE || lz < 0 || lz >= CHUNK_SIZE || y < MIN_HEIGHT || y >= WORLD_HEIGHT {
            return;
        }
        
        if let Some(changes) = self.world_changes {
            let pos = BlockPos::new(self.base_x + lx, y, self.base_z + lz);
            if changes.contains_key(&pos) {
                return;
            }
        }
        
        let idx = Self::index(lx, y, lz);
        if self.blocks[idx] == AIR {
            self.blocks[idx] = block;
        }
    }
    
    /// Установка листвы - записывает позицию для субвокселей
    pub fn set_leaf(&mut self, lx: i32, y: i32, lz: i32, leaf_type: BlockType) {
        if lx < 0 || lx >= CHUNK_SIZE || lz < 0 || lz >= CHUNK_SIZE || y < MIN_HEIGHT || y >= WORLD_HEIGHT {
            return;
        }
        
        if let Some(changes) = self.world_changes {
            let pos = BlockPos::new(self.base_x + lx, y, self.base_z + lz);
            if changes.contains_key(&pos) {
                return;
            }
        }
        
        let idx = Self::index(lx, y, lz);
        if self.blocks[idx] != AIR {
            return;
        }
        
        // НЕ ставим блок - листва будет только субвокселями!
        // Записываем позицию для генерации субвокселей
        self.leaf_positions.push(LeafSubVoxel {
            world_x: self.base_x + lx,
            world_y: y,
            world_z: self.base_z + lz,
            block_type: leaf_type,
        });
    }
    
    /// Принудительная установка (для ствола)
    pub fn set_solid(&mut self, lx: i32, y: i32, lz: i32, block: BlockType) {
        if lx < 0 || lx >= CHUNK_SIZE || lz < 0 || lz >= CHUNK_SIZE || y < MIN_HEIGHT || y >= WORLD_HEIGHT {
            return;
        }
        
        if let Some(changes) = self.world_changes {
            let pos = BlockPos::new(self.base_x + lx, y, self.base_z + lz);
            if changes.contains_key(&pos) {
                return;
            }
        }
        
        let idx = Self::index(lx, y, lz);
        self.blocks[idx] = block;
    }

    #[inline]
    fn index(lx: i32, y: i32, lz: i32) -> usize {
        let ly = y - MIN_HEIGHT;
        (ly as usize) * (CHUNK_SIZE as usize * CHUNK_SIZE as usize)
            + (lz as usize) * (CHUNK_SIZE as usize)
            + (lx as usize)
    }
    
    /// Получить позиции листвы
    pub fn take_leaf_subvoxels(&mut self) -> Vec<LeafSubVoxel> {
        std::mem::take(&mut self.leaf_positions)
    }
}

/// Генерация стандартного дерева (Дуб/Береза)
pub fn place_basic_tree(writer: &mut ChunkWriter, lx: i32, base_y: i32, lz: i32, tree_type: TreeType, height: i32) {
    let (log, leaves) = match tree_type {
        TreeType::Birch => (BIRCH_LOG, BIRCH_LEAVES),
        TreeType::Spruce => (SPRUCE_LOG, SPRUCE_LEAVES),
        TreeType::Oak => (OAK_LOG, OAK_LEAVES),
    };

    // Листва (сначала, чтобы ствол мог перезаписать центр)
    for y in (base_y + height - 3)..=(base_y + height + 1) {
        let y_offset = y - (base_y + height);
        let radius: i32 = if y_offset >= 0 { 1 } else { 2 };

        for x in -radius..=radius {
            for z in -radius..=radius {
                // Скругляем углы
                if x.abs() == radius && z.abs() == radius && y_offset < 0 {
                    continue;
                }
                writer.set_leaf(lx + x, y, lz + z, leaves);
            }
        }
    }

    // Ствол
    for y in 0..height {
        writer.set_solid(lx, base_y + y, lz, log);
    }
}

/// Генерация ели (конусообразная крона)
pub fn place_spruce_tree(writer: &mut ChunkWriter, lx: i32, base_y: i32, lz: i32, height: i32) {
    let log = SPRUCE_LOG;
    let leaves = SPRUCE_LEAVES;

    // Ствол
    for y in 0..height {
        writer.set_solid(lx, base_y + y, lz, log);
    }

    let top_y = base_y + height;
    
    // Верхушка
    writer.set_leaf(lx, top_y, lz, leaves);
    writer.set_leaf(lx, top_y + 1, lz, leaves);

    // Слои вниз (конусом)
    for y in (base_y + 2..top_y).rev() {
        let layer = (top_y - y) % 4;
        let radius = if layer == 1 || layer == 3 { 2 } else { 1 };

        if radius == 1 {
            writer.set_leaf(lx + 1, y, lz, leaves);
            writer.set_leaf(lx - 1, y, lz, leaves);
            writer.set_leaf(lx, y, lz + 1, leaves);
            writer.set_leaf(lx, y, lz - 1, leaves);
        } else {
            for dx in -1i32..=1 {
                for dz in -1i32..=1 {
                    if dx.abs() + dz.abs() <= 2 {
                        writer.set_leaf(lx + dx, y, lz + dz, leaves);
                    }
                }
            }
        }
    }
}
