use std::collections::{HashMap, HashSet};
use rayon::prelude::*;

use crate::gpu::terrain::voxel::{VoxelChunk, ChunkNeighbors, CHUNK_SIZE, MeshingContext};
use crate::gpu::terrain::mesh::TerrainVertex;
use crate::gpu::terrain::cache::ChunkKey;
use crate::gpu::terrain::lod::LodLevel;
use crate::gpu::terrain::BlockPos;
use crate::gpu::blocks::BlockType;

use super::types::{GeneratedChunkData, GeneratedMesh};
use super::lod_mesh::generate_lod_chunk;

/// Генератор terrain с кэшированием и zero-allocation контекстом
pub(super) struct HybridGenerator {
    voxel_cache: HashMap<(i32, i32), VoxelChunk>,
    mesh_cache: HashMap<ChunkKey, (Vec<TerrainVertex>, Vec<u32>)>,
    cache_version: u64,
    lod_levels: [LodLevel; 4],
    /// Переиспользуемый контекст для генерации мешей (zero-allocation)
    meshing_ctx: MeshingContext,
}

impl HybridGenerator {
    pub fn new() -> Self {
        Self {
            voxel_cache: HashMap::new(),
            mesh_cache: HashMap::new(),
            cache_version: 0,
            lod_levels: LodLevel::DEFAULT_LEVELS,
            meshing_ctx: MeshingContext::new(),
        }
    }
    
    pub fn set_lod_distances(&mut self, distances: [i32; 4]) {
        self.lod_levels[0] = LodLevel { min_chunks: 0, max_chunks: distances[0], scale: 1 };
        self.lod_levels[1] = LodLevel { min_chunks: distances[0], max_chunks: distances[1], scale: 2 };
        self.lod_levels[2] = LodLevel { min_chunks: distances[1], max_chunks: distances[2], scale: 4 };
        self.lod_levels[3] = LodLevel { min_chunks: distances[2], max_chunks: distances[3], scale: 8 };
        self.mesh_cache.clear();
    }

    pub fn generate(
        &mut self,
        player_x: f32,
        player_z: f32,
        world_changes: &HashMap<BlockPos, BlockType>,
        changes_version: u64,
    ) -> GeneratedMesh {
        let center_cx = (player_x / CHUNK_SIZE as f32).floor() as i32;
        let center_cz = (player_z / CHUNK_SIZE as f32).floor() as i32;
        
        self.invalidate_changed_chunks(world_changes, changes_version);
        
        let (required_keys, chunks_to_generate) = self.collect_chunks_to_generate(center_cx, center_cz);
        
        // Воксельные чанки - последовательно (нужен кэш соседей)
        self.generate_voxel_chunks(&chunks_to_generate, world_changes);
        
        // LOD чанки - параллельно
        self.generate_lod_chunks_parallel(&chunks_to_generate);
        
        let new_chunks = self.collect_new_chunks(&chunks_to_generate);
        self.cleanup_caches(center_cx, center_cz, &required_keys);
        
        GeneratedMesh { new_chunks, required_keys }
    }


    fn invalidate_changed_chunks(&mut self, world_changes: &HashMap<BlockPos, BlockType>, changes_version: u64) {
        if changes_version == self.cache_version { return; }
        
        for pos in world_changes.keys() {
            let chunk_x = pos.x.div_euclid(CHUNK_SIZE);
            let chunk_z = pos.z.div_euclid(CHUNK_SIZE);
            for dx in -1..=1 {
                for dz in -1..=1 {
                    self.voxel_cache.remove(&(chunk_x + dx, chunk_z + dz));
                    self.mesh_cache.remove(&ChunkKey::new(chunk_x + dx, chunk_z + dz, 1));
                }
            }
        }
        self.cache_version = changes_version;
    }
    
    fn collect_chunks_to_generate(&self, center_cx: i32, center_cz: i32) -> (HashSet<ChunkKey>, Vec<(ChunkKey, bool)>) {
        let mut required_keys = HashSet::new();
        let mut chunks_to_generate = Vec::new();
        
        for lod in &self.lod_levels {
            for dz in -lod.max_chunks..=lod.max_chunks {
                for dx in -lod.max_chunks..=lod.max_chunks {
                    let dist = dx.abs().max(dz.abs());
                    if dist < lod.min_chunks || dist >= lod.max_chunks { continue; }
                    
                    let world_cx = center_cx + dx;
                    let world_cz = center_cz + dz;
                    
                    let (final_cx, final_cz) = if lod.scale > 1 {
                        (world_cx.div_euclid(lod.scale) * lod.scale, world_cz.div_euclid(lod.scale) * lod.scale)
                    } else {
                        (world_cx, world_cz)
                    };
                    
                    let key = ChunkKey::new(final_cx, final_cz, lod.scale);
                    if required_keys.contains(&key) { continue; }
                    required_keys.insert(key);
                    
                    if !self.mesh_cache.contains_key(&key) {
                        chunks_to_generate.push((key, lod.scale == 1));
                    }
                }
            }
        }
        
        (required_keys, chunks_to_generate)
    }
    
    fn generate_voxel_chunks(&mut self, chunks: &[(ChunkKey, bool)], world_changes: &HashMap<BlockPos, BlockType>) {
        let voxel_keys: Vec<_> = chunks.iter()
            .filter(|(_, is_voxel)| *is_voxel)
            .map(|(key, _)| *key)
            .collect();
        
        for key in voxel_keys {
            let (vertices, indices) = self.generate_voxel_chunk(key.x, key.z, world_changes);
            self.mesh_cache.insert(key, (vertices, indices));
        }
    }
    
    fn generate_lod_chunks_parallel(&mut self, chunks: &[(ChunkKey, bool)]) {
        let lod_keys: Vec<_> = chunks.iter()
            .filter(|(_, is_voxel)| !*is_voxel)
            .map(|(key, _)| *key)
            .collect();
        
        let results: Vec<_> = lod_keys.par_iter()
            .map(|key| (*key, generate_lod_chunk(key.x, key.z, key.scale)))
            .collect();
        
        for (key, (vertices, indices)) in results {
            self.mesh_cache.insert(key, (vertices, indices));
        }
    }
    
    fn generate_voxel_chunk(&mut self, cx: i32, cz: i32, world_changes: &HashMap<BlockPos, BlockType>) -> (Vec<TerrainVertex>, Vec<u32>) {
        // Ensure chunk and neighbors exist
        if !self.voxel_cache.contains_key(&(cx, cz)) {
            self.voxel_cache.insert((cx, cz), VoxelChunk::new(cx, cz, world_changes));
        }
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            if !self.voxel_cache.contains_key(&(cx + dx, cz + dz)) {
                self.voxel_cache.insert((cx + dx, cz + dz), VoxelChunk::new(cx + dx, cz + dz, world_changes));
            }
        }
        
        let neighbors = ChunkNeighbors {
            pos_x: self.voxel_cache.get(&(cx + 1, cz)),
            neg_x: self.voxel_cache.get(&(cx - 1, cz)),
            pos_z: self.voxel_cache.get(&(cx, cz + 1)),
            neg_z: self.voxel_cache.get(&(cx, cz - 1)),
        };
        
        // Используем zero-allocation контекст
        self.voxel_cache.get(&(cx, cz))
            .map(|c| c.generate_mesh_with_context(&neighbors, &mut self.meshing_ctx))
            .unwrap_or_default()
    }
    
    fn collect_new_chunks(&self, chunks: &[(ChunkKey, bool)]) -> Vec<GeneratedChunkData> {
        chunks.iter()
            .filter_map(|(key, _)| {
                self.mesh_cache.get(key).and_then(|(vertices, indices)| {
                    if !vertices.is_empty() {
                        Some(GeneratedChunkData {
                            key: *key,
                            vertices: vertices.clone(),
                            indices: indices.clone(),
                        })
                    } else {
                        None
                    }
                })
            })
            .collect()
    }
    
    fn cleanup_caches(&mut self, center_cx: i32, center_cz: i32, required_keys: &HashSet<ChunkKey>) {
        let max_dist = self.lod_levels[0].max_chunks + 2;
        self.voxel_cache.retain(|(cx, cz), _| {
            (cx - center_cx).abs().max((cz - center_cz).abs()) < max_dist
        });
        self.mesh_cache.retain(|key, _| required_keys.contains(key));
    }
}
