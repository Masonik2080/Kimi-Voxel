// ============================================
// GPU Chunk Manager - Управление GPU буферами
// ============================================

use std::collections::HashMap;
use std::sync::Arc;

use crate::gpu::terrain::cache::ChunkKey;
use crate::gpu::terrain::mesh::TerrainVertex;
use super::chunk::GpuChunk;

/// Менеджер GPU буферов чанков
pub struct GpuChunkManager {
    chunks: HashMap<ChunkKey, GpuChunk>,
    device: Arc<wgpu::Device>,
}

impl GpuChunkManager {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        Self {
            chunks: HashMap::with_capacity(1024),
            device,
        }
    }

    /// Загружает чанк на GPU
    pub fn upload(&mut self, key: ChunkKey, vertices: &[TerrainVertex], indices: &[u32]) {
        if vertices.is_empty() || indices.is_empty() {
            return;
        }
        
        let gpu_chunk = GpuChunk::new(&self.device, key, vertices, indices);
        self.chunks.insert(key, gpu_chunk);
    }

    /// Удаляет чанки которых нет в списке нужных
    pub fn retain_only(&mut self, valid_keys: &std::collections::HashSet<ChunkKey>) {
        self.chunks.retain(|key, _| valid_keys.contains(key));
    }

    /// Итератор по всем GPU чанкам для рендеринга
    pub fn iter(&self) -> impl Iterator<Item = &GpuChunk> {
        self.chunks.values()
    }
}
