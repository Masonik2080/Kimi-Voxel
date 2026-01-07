// ============================================
// GPU Chunk - Буферы чанка на GPU
// ============================================

use wgpu::util::DeviceExt;
use crate::gpu::terrain::cache::ChunkKey;
use crate::gpu::terrain::mesh::TerrainVertex;

/// GPU буферы для одного чанка
pub struct GpuChunk {
    pub key: ChunkKey,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl GpuChunk {
    pub fn new(
        device: &wgpu::Device,
        key: ChunkKey,
        vertices: &[TerrainVertex],
        indices: &[u32],
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Chunk {:?} Vertices", key)),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Chunk {:?} Indices", key)),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            key,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}
