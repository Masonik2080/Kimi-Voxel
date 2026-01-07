// ============================================
// SubVoxel Renderer - GPU рендеринг (ОПТИМИЗИРОВАННЫЙ)
// ============================================
//
// Использует:
// - PackedVertex (8 байт вместо 36)
// - MaskGreedyContext (без сортировки)
// - SparseChunkStorage (O(N) память)

use std::collections::HashMap;
use crate::gpu::subvoxel::meshing::{PackedVertex, MaskGreedyContext, VoxelAccess, greedy_mesh_masked};
use crate::gpu::subvoxel::chunk::{SubVoxelChunkKey, SparseChunkStorage};
use crate::gpu::subvoxel::components::SubVoxelWorld;
use crate::gpu::blocks::BlockType;

/// GPU данные для одного чанка
struct ChunkGpuData {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    version: u64,
}

/// Рендерер субвокселей (оптимизированный)
pub struct OptimizedSubVoxelRenderer {
    /// GPU буферы по чанкам
    chunk_buffers: HashMap<SubVoxelChunkKey, ChunkGpuData>,
    /// Контекст для meshing (переиспользуется)
    mesh_ctx: MaskGreedyContext,
}

impl OptimizedSubVoxelRenderer {
    pub fn new(_device: &wgpu::Device) -> Self {
        Self {
            chunk_buffers: HashMap::new(),
            mesh_ctx: MaskGreedyContext::new(),
        }
    }

    /// Обновить только грязные чанки
    pub fn update_dirty_chunks(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world: &mut SubVoxelWorld,
    ) {
        let dirty_chunks = world.take_dirty_chunks();

        if dirty_chunks.is_empty() {
            return;
        }

        for chunk_key in dirty_chunks {
            if let Some(chunk) = world.get_chunk(&chunk_key) {
                if chunk.is_empty() {
                    self.chunk_buffers.remove(&chunk_key);
                    continue;
                }

                // Генерируем меш через mask greedy
                let chunk_offset = [
                    (chunk_key.x * 16) as f32,
                    0.0,
                    (chunk_key.z * 16) as f32,
                ];

                let voxel_access = SparseChunkVoxelAccess::new(chunk);
                greedy_mesh_masked(&voxel_access, &mut self.mesh_ctx, chunk_offset);

                if self.mesh_ctx.vertices.is_empty() {
                    self.chunk_buffers.remove(&chunk_key);
                } else {
                    let vertices = std::mem::take(&mut self.mesh_ctx.vertices);
                    let indices = std::mem::take(&mut self.mesh_ctx.indices);
                    self.update_chunk_buffers(
                        device, queue, chunk_key,
                        vertices, indices, chunk.version()
                    );
                }
            } else {
                self.chunk_buffers.remove(&chunk_key);
            }
        }
    }

    fn update_chunk_buffers(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chunk_key: SubVoxelChunkKey,
        vertices: Vec<PackedVertex>,
        indices: Vec<u32>,
        version: u64,
    ) {
        let vertex_size = vertices.len() * std::mem::size_of::<PackedVertex>();
        let index_size = indices.len() * std::mem::size_of::<u32>();

        let needs_recreate = self.chunk_buffers.get(&chunk_key)
            .map(|data| {
                data.vertex_buffer.size() < vertex_size as u64 ||
                data.index_buffer.size() < index_size as u64
            })
            .unwrap_or(true);

        if needs_recreate {
            let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("SubVoxel Chunk {:?} Vertex", chunk_key)),
                size: (vertex_size * 2).max(256) as u64, // Меньше минимум т.к. вершины компактнее
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("SubVoxel Chunk {:?} Index", chunk_key)),
                size: (index_size * 2).max(256) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            self.chunk_buffers.insert(chunk_key, ChunkGpuData {
                vertex_buffer,
                index_buffer,
                num_indices: 0,
                version: 0,
            });
        }

        if let Some(gpu_data) = self.chunk_buffers.get_mut(&chunk_key) {
            queue.write_buffer(&gpu_data.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
            queue.write_buffer(&gpu_data.index_buffer, 0, bytemuck::cast_slice(&indices));
            gpu_data.num_indices = indices.len() as u32;
            gpu_data.version = version;
        }
    }

    pub fn iter_chunk_buffers(&self) -> impl Iterator<Item = (&wgpu::Buffer, &wgpu::Buffer, u32)> {
        self.chunk_buffers.values()
            .filter(|d| d.num_indices > 0)
            .map(|d| (&d.vertex_buffer, &d.index_buffer, d.num_indices))
    }

    pub fn total_indices(&self) -> u32 {
        self.chunk_buffers.values().map(|d| d.num_indices).sum()
    }

    pub fn has_content(&self) -> bool {
        self.chunk_buffers.values().any(|d| d.num_indices > 0)
    }

    pub fn chunk_count(&self) -> usize {
        self.chunk_buffers.len()
    }

    /// Общее использование GPU памяти (байт)
    pub fn gpu_memory_usage(&self) -> usize {
        self.chunk_buffers.values()
            .map(|d| d.vertex_buffer.size() as usize + d.index_buffer.size() as usize)
            .sum()
    }
}

// ============================================
// Адаптер VoxelAccess для SparseChunkStorage
// ============================================

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
            min_y: min_y as i32 * 4,
            max_y: (max_y as i32 + 1) * 4 - 1,
        }
    }
}

impl<'a> VoxelAccess for SparseChunkVoxelAccess<'a> {
    fn get(&self, x: i32, y: i32, z: i32) -> Option<BlockType> {
        if x < 0 || x >= 64 || z < 0 || z >= 64 || y < self.min_y || y > self.max_y {
            return None;
        }

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
