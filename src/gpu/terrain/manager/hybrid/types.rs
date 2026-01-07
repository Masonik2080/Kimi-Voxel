use std::collections::{HashMap, HashSet};

use crate::gpu::terrain::mesh::TerrainVertex;
use crate::gpu::terrain::cache::ChunkKey;
use crate::gpu::terrain::BlockPos;
use crate::gpu::blocks::BlockType;

/// Запрос на генерацию terrain
pub(super) struct GenerateRequest {
    pub player_x: f32,
    pub player_z: f32,
    pub world_changes: HashMap<BlockPos, BlockType>,
    pub changes_version: u64,
    pub lod_distances: Option<[i32; 4]>,
}

/// Данные сгенерированного чанка
pub struct GeneratedChunkData {
    pub key: ChunkKey,
    pub vertices: Vec<TerrainVertex>,
    pub indices: Vec<u32>,
}

/// Результат генерации мешей
pub struct GeneratedMesh {
    pub new_chunks: Vec<GeneratedChunkData>,
    pub required_keys: HashSet<ChunkKey>,
}
