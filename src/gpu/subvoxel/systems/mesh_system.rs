// ============================================
// Mesh System - ECS система генерации мешей (ОПТИМИЗИРОВАННАЯ)
// ============================================
//
// Использует:
// - SparseChunkStorage (O(N) память)
// - MaskGreedyContext (битовые маски, без сортировки)
// - PackedVertex (8 байт вместо 36)

use std::collections::{HashMap, HashSet};
use crate::gpu::blocks::BlockType;
use crate::gpu::subvoxel::chunk::{SubVoxelChunkKey, SparseChunkStorage, PackedBlockKey};
use crate::gpu::subvoxel::meshing::{
    PackedVertex, MaskGreedyContext, VoxelAccess, greedy_mesh_masked,
};

// ============================================
// Компоненты
// ============================================

/// Маркер грязного чанка
#[derive(Clone, Copy, Debug)]
pub struct DirtyChunk {
    pub key: SubVoxelChunkKey,
    pub priority: u8,
}

/// Готовый меш чанка (оптимизированный)
#[derive(Default)]
pub struct ChunkMesh {
    pub vertices: Vec<PackedVertex>,
    pub indices: Vec<u32>,
    pub version: u64,
}

impl ChunkMesh {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    #[inline]
    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    /// Память в байтах
    #[inline]
    pub fn memory_usage(&self) -> usize {
        self.vertices.len() * std::mem::size_of::<PackedVertex>() +
        self.indices.len() * std::mem::size_of::<u32>()
    }
}

// ============================================
// Адаптер для VoxelAccess
// ============================================

/// Адаптер SparseChunkStorage -> VoxelAccess для mask greedy
struct SparseChunkVoxelAccess<'a> {
    storage: &'a SparseChunkStorage,
    min_y: i32,
    max_y: i32,
}

impl<'a> SparseChunkVoxelAccess<'a> {
    fn new(storage: &'a SparseChunkStorage) -> Self {
        let (min_y, max_y) = storage.y_range();
        Self {
            storage,
            min_y: min_y as i32 * 4, // В субвоксельных координатах
            max_y: (max_y as i32 + 1) * 4 - 1,
        }
    }
}

impl<'a> VoxelAccess for SparseChunkVoxelAccess<'a> {
    fn get(&self, x: i32, y: i32, z: i32) -> Option<BlockType> {
        if x < 0 || x >= 64 || z < 0 || z >= 64 || y < self.min_y || y > self.max_y {
            return None;
        }

        // Конвертируем субвоксельные координаты в блок + sub
        let block_x = (x / 4) as u8;
        let block_z = (z / 4) as u8;
        let block_y = (y / 4) as u8;
        let sub_x = (x % 4) as u8;
        let sub_y = (y % 4) as u8;
        let sub_z = (z % 4) as u8;

        self.storage.get(block_x, block_y, block_z, sub_x, sub_y, sub_z, 2)
    }

    fn bounds(&self) -> (i32, i32, i32, i32, i32, i32) {
        (0, self.min_y, 0, 63, self.max_y, 63)
    }
}

// ============================================
// Ресурсы
// ============================================

/// Конфигурация мешинга
#[derive(Clone)]
pub struct MeshingConfig {
    pub max_chunks_per_frame: usize,
    pub priority_radius: i32,
}

impl Default for MeshingConfig {
    fn default() -> Self {
        Self {
            max_chunks_per_frame: 4,
            priority_radius: 2,
        }
    }
}

/// Контекст системы мешинга
pub struct MeshingSystemContext {
    /// Контекст mask greedy (переиспользуемый)
    greedy_ctx: MaskGreedyContext,
    /// Очередь грязных чанков
    dirty_queue: Vec<DirtyChunk>,
    /// Готовые меши
    meshes: HashMap<SubVoxelChunkKey, ChunkMesh>,
    /// Конфигурация
    config: MeshingConfig,
}

impl MeshingSystemContext {
    pub fn new() -> Self {
        Self {
            greedy_ctx: MaskGreedyContext::new(),
            dirty_queue: Vec::with_capacity(64),
            meshes: HashMap::with_capacity(256),
            config: MeshingConfig::default(),
        }
    }

    pub fn with_config(config: MeshingConfig) -> Self {
        Self {
            greedy_ctx: MaskGreedyContext::new(),
            dirty_queue: Vec::with_capacity(64),
            meshes: HashMap::with_capacity(256),
            config,
        }
    }
}

impl Default for MeshingSystemContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================
// Системы
// ============================================

/// Помечает чанк как грязный
pub fn mark_chunk_dirty(ctx: &mut MeshingSystemContext, key: SubVoxelChunkKey, priority: u8) {
    if ctx.dirty_queue.iter().any(|d| d.key == key) {
        return;
    }
    ctx.dirty_queue.push(DirtyChunk { key, priority });
}

/// Обновляет приоритеты чанков
pub fn update_priorities(ctx: &mut MeshingSystemContext, player_chunk_x: i32, player_chunk_z: i32) {
    let radius = ctx.config.priority_radius;
    for dirty in &mut ctx.dirty_queue {
        let dx = (dirty.key.x - player_chunk_x).abs();
        let dz = (dirty.key.z - player_chunk_z).abs();
        if dx <= radius && dz <= radius {
            dirty.priority = 255 - (dx + dz).min(255) as u8;
        }
    }
}

/// Обрабатывает очередь мешинга
/// Возвращает количество обработанных чанков
pub fn process_meshing_queue(
    ctx: &mut MeshingSystemContext,
    storages: &HashMap<SubVoxelChunkKey, SparseChunkStorage>,
) -> usize {
    if ctx.dirty_queue.is_empty() {
        return 0;
    }

    // Сортируем по приоритету
    ctx.dirty_queue.sort_by(|a, b| b.priority.cmp(&a.priority));

    let max_chunks = ctx.config.max_chunks_per_frame;
    let mut processed = 0;

    while processed < max_chunks && !ctx.dirty_queue.is_empty() {
        let dirty = ctx.dirty_queue.remove(0);
        
        let Some(storage) = storages.get(&dirty.key) else {
            // Чанк удалён - удаляем меш
            ctx.meshes.remove(&dirty.key);
            continue;
        };

        if storage.is_empty() {
            ctx.meshes.remove(&dirty.key);
            processed += 1;
            continue;
        }

        // Генерируем меш через mask greedy
        let chunk_offset = [
            (dirty.key.x * 16) as f32,
            0.0,
            (dirty.key.z * 16) as f32,
        ];

        let voxel_access = SparseChunkVoxelAccess::new(storage);
        greedy_mesh_masked(&voxel_access, &mut ctx.greedy_ctx, chunk_offset);

        // Сохраняем результат
        ctx.meshes.insert(dirty.key, ChunkMesh {
            vertices: std::mem::take(&mut ctx.greedy_ctx.vertices),
            indices: std::mem::take(&mut ctx.greedy_ctx.indices),
            version: storage.version(),
        });

        processed += 1;
    }

    processed
}

/// Получает меш чанка
#[inline]
pub fn get_chunk_mesh(ctx: &MeshingSystemContext, key: SubVoxelChunkKey) -> Option<&ChunkMesh> {
    ctx.meshes.get(&key)
}

/// Получает все меши
#[inline]
pub fn get_all_meshes(ctx: &MeshingSystemContext) -> &HashMap<SubVoxelChunkKey, ChunkMesh> {
    &ctx.meshes
}

/// Удаляет меш чанка
pub fn remove_chunk_mesh(ctx: &mut MeshingSystemContext, key: SubVoxelChunkKey) {
    ctx.meshes.remove(&key);
    ctx.dirty_queue.retain(|d| d.key != key);
}

/// Очищает все меши
pub fn clear_all_meshes(ctx: &mut MeshingSystemContext) {
    ctx.meshes.clear();
    ctx.dirty_queue.clear();
}

// ============================================
// Статистика
// ============================================

#[derive(Clone, Copy, Debug, Default)]
pub struct MeshingStats {
    pub total_meshes: usize,
    pub dirty_queue_size: usize,
    pub total_vertices: usize,
    pub total_indices: usize,
    pub total_memory_bytes: usize,
}

pub fn get_meshing_stats(ctx: &MeshingSystemContext) -> MeshingStats {
    let (total_vertices, total_indices, total_memory) = ctx.meshes.values()
        .fold((0, 0, 0), |(v, i, m), mesh| {
            (v + mesh.vertex_count(), i + mesh.index_count(), m + mesh.memory_usage())
        });

    MeshingStats {
        total_meshes: ctx.meshes.len(),
        dirty_queue_size: ctx.dirty_queue.len(),
        total_vertices,
        total_indices,
        total_memory_bytes: total_memory,
    }
}
