// ============================================
// Chunk Grid - Единая сетка субвокселей чанка
// ============================================
//
// Вместо greedy meshing на уровне отдельных блоков,
// строим единую сетку для всего чанка и делаем greedy на ней.
// Это позволяет:
// 1. Culling граней между соседними блоками
// 2. Greedy meshing через границы блоков
// 3. Эффективность на больших полотнах

use crate::gpu::blocks::BlockType;
use crate::gpu::subvoxel::chunk::ChunkSubVoxelStorage;

/// Размер сетки чанка в субвокселях (16 блоков * 4 субвокселя = 64)
pub const CHUNK_GRID_SIZE: usize = 64;
/// Максимальная высота сетки (оптимизация - только занятый диапазон)
pub const MAX_GRID_HEIGHT: usize = 64;

/// Единая 3D сетка субвокселей для чанка
/// Хранит только занятый по Y диапазон для экономии памяти
pub struct ChunkGrid {
    /// Данные сетки [z][y_local][x] где y_local = y - min_y
    data: Vec<Option<BlockType>>,
    /// Минимальная Y координата
    pub min_y: i32,
    /// Максимальная Y координата
    pub max_y: i32,
    /// Высота сетки в субвокселях
    height: usize,
}

impl ChunkGrid {
    /// Создать сетку из хранилища чанка
    pub fn from_chunk_storage(storage: &ChunkSubVoxelStorage) -> Option<Self> {
        if storage.is_empty() {
            return None;
        }

        // Находим диапазон Y
        let mut min_block_y = i32::MAX;
        let mut max_block_y = i32::MIN;

        for (key, _) in storage.iter_blocks() {
            min_block_y = min_block_y.min(key.y as i32);
            max_block_y = max_block_y.max(key.y as i32);
        }

        if min_block_y > max_block_y {
            return None;
        }

        // Конвертируем в субвоксельные координаты (4 субвокселя на блок)
        let min_y = min_block_y * 4;
        let max_y = (max_block_y + 1) * 4 - 1;
        let height = ((max_y - min_y + 1) as usize).min(MAX_GRID_HEIGHT);

        // Аллоцируем сетку
        let total_size = CHUNK_GRID_SIZE * height * CHUNK_GRID_SIZE;
        let mut data = vec![None; total_size];

        // Заполняем сетку из октодеревьев
        for (key, octree) in storage.iter_blocks() {
            let block_base_x = (key.x as usize) * 4;
            let block_base_y = ((key.y as i32 - min_block_y) * 4) as usize;
            let block_base_z = (key.z as usize) * 4;

            // Итерируем по субвокселям в октодереве
            for (sx, sy, sz, size, block_type) in octree.iter_solid() {
                // Конвертируем нормализованные координаты [0,1) в индексы [0,4)
                let cells = (size * 4.0).round() as usize;
                let start_x = (sx * 4.0).floor() as usize;
                let start_y = (sy * 4.0).floor() as usize;
                let start_z = (sz * 4.0).floor() as usize;

                // Заполняем все ячейки субвокселя
                for dz in 0..cells {
                    for dy in 0..cells {
                        for dx in 0..cells {
                            let gx = block_base_x + start_x + dx;
                            let gy = block_base_y + start_y + dy;
                            let gz = block_base_z + start_z + dz;

                            if gx < CHUNK_GRID_SIZE && gy < height && gz < CHUNK_GRID_SIZE {
                                let idx = gz * height * CHUNK_GRID_SIZE + gy * CHUNK_GRID_SIZE + gx;
                                data[idx] = Some(block_type);
                            }
                        }
                    }
                }
            }
        }

        Some(Self {
            data,
            min_y,
            max_y,
            height,
        })
    }

    /// Получить блок по координатам сетки
    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> Option<BlockType> {
        if x >= CHUNK_GRID_SIZE || y >= self.height || z >= CHUNK_GRID_SIZE {
            return None;
        }
        let idx = z * self.height * CHUNK_GRID_SIZE + y * CHUNK_GRID_SIZE + x;
        self.data[idx]
    }

    /// Проверить, есть ли блок (для culling)
    #[inline]
    pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        if x < 0 || x >= CHUNK_GRID_SIZE as i32 || 
           y < 0 || y >= self.height as i32 || 
           z < 0 || z >= CHUNK_GRID_SIZE as i32 {
            return false;
        }
        self.get(x as usize, y as usize, z as usize).is_some()
    }

    /// Размеры сетки
    #[inline]
    pub fn width(&self) -> usize { CHUNK_GRID_SIZE }
    
    #[inline]
    pub fn height(&self) -> usize { self.height }
    
    #[inline]
    pub fn depth(&self) -> usize { CHUNK_GRID_SIZE }

    /// Размер одного субвокселя в мировых координатах
    #[inline]
    pub fn cell_size(&self) -> f32 { 0.25 } // 1/4 блока
}
