// ============================================
// Meshing Context - Zero-allocation буферы
// ============================================
//
// Контекст для генерации мешей с переиспользуемыми буферами.
// Принцип "Alloc Once, Reuse Forever" - память выделяется один раз,
// затем только очищается через clear() сохраняя capacity.

use crate::gpu::terrain::mesh::TerrainVertex;
use super::constants::CHUNK_SIZE;
use super::greedy::FaceInfo;

/// Максимальный размер слоя для масок (16x16)
const LAYER_SIZE: usize = (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize);

/// Максимальная высота для вертикальных масок
const MAX_HEIGHT: usize = 160; // WORLD_HEIGHT - MIN_HEIGHT

/// Размер вертикальной маски (16 * max_height)
const VERTICAL_MASK_SIZE: usize = (CHUNK_SIZE as usize) * MAX_HEIGHT;

/// Буферы для одного направления граней
#[derive(Default)]
pub struct FaceMaskBuffers {
    /// Маска положительного направления
    pub mask_pos: Vec<Option<FaceInfo>>,
    /// Маска отрицательного направления  
    pub mask_neg: Vec<Option<FaceInfo>>,
    /// Флаги посещённых ячеек для greedy meshing
    pub visited: Vec<bool>,
}

impl FaceMaskBuffers {
    /// Создаёт буферы с заданной ёмкостью
    pub fn with_capacity(size: usize) -> Self {
        Self {
            mask_pos: vec![None; size],
            mask_neg: vec![None; size],
            visited: vec![false; size],
        }
    }

    /// Очищает буферы, сохраняя capacity
    #[inline]
    pub fn clear(&mut self, size: usize) {
        // Заполняем None/false вместо clear() чтобы сохранить длину
        self.mask_pos[..size].fill(None);
        self.mask_neg[..size].fill(None);
        self.visited[..size].fill(false);
    }

    /// Очищает только visited буфер
    #[inline]
    pub fn clear_visited(&mut self, size: usize) {
        self.visited[..size].fill(false);
    }
}

/// Контекст генерации меша - содержит все переиспользуемые буферы
pub struct MeshingContext {
    /// Буферы для Y граней (горизонтальные слои 16x16)
    pub y_buffers: FaceMaskBuffers,
    /// Буферы для X граней (вертикальные слои 16xH)
    pub x_buffers: FaceMaskBuffers,
    /// Буферы для Z граней (вертикальные слои 16xH)
    pub z_buffers: FaceMaskBuffers,
    
    /// Выходной буфер вершин
    pub vertices: Vec<TerrainVertex>,
    /// Выходной буфер индексов
    pub indices: Vec<u32>,
    
    /// Временный буфер для результатов greedy meshing
    pub greedy_results: Vec<(usize, usize, usize, usize, FaceInfo)>,
}

impl MeshingContext {
    /// Создаёт новый контекст с преаллоцированными буферами
    pub fn new() -> Self {
        Self {
            y_buffers: FaceMaskBuffers::with_capacity(LAYER_SIZE),
            x_buffers: FaceMaskBuffers::with_capacity(VERTICAL_MASK_SIZE),
            z_buffers: FaceMaskBuffers::with_capacity(VERTICAL_MASK_SIZE),
            vertices: Vec::with_capacity(8000),
            indices: Vec::with_capacity(12000),
            greedy_results: Vec::with_capacity(256),
        }
    }

    /// Очищает выходные буферы перед генерацией нового меша
    #[inline]
    pub fn clear_output(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    /// Очищает буферы Y масок для нового слоя
    #[inline]
    pub fn clear_y_masks(&mut self) {
        self.y_buffers.clear(LAYER_SIZE);
    }

    /// Очищает буферы X масок для нового слоя
    #[inline]
    pub fn clear_x_masks(&mut self, height: usize) {
        let size = (CHUNK_SIZE as usize) * height;
        self.x_buffers.clear(size);
    }

    /// Очищает буферы Z масок для нового слоя
    #[inline]
    pub fn clear_z_masks(&mut self, height: usize) {
        let size = (CHUNK_SIZE as usize) * height;
        self.z_buffers.clear(size);
    }

    /// Возвращает результаты и очищает внутренние буферы
    #[inline]
    pub fn take_results(&mut self) -> (Vec<TerrainVertex>, Vec<u32>) {
        let vertices = std::mem::take(&mut self.vertices);
        let indices = std::mem::take(&mut self.indices);
        
        // Восстанавливаем capacity для следующего использования
        self.vertices = Vec::with_capacity(8000);
        self.indices = Vec::with_capacity(12000);
        
        (vertices, indices)
    }
}

impl Default for MeshingContext {
    fn default() -> Self {
        Self::new()
    }
}
