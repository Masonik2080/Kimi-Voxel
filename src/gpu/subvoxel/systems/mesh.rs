// ============================================
// Mesh System - ECS система генерации мешей
// ============================================

// Legacy mesher (с декомпрессией в ChunkGrid)
pub use crate::gpu::subvoxel::meshing::{
    ChunkMeshData, ChunkMeshContext, mesh_chunk, mesh_chunk_new, SubVoxelVertex,
};

// Новый octree mesher (без декомпрессии, O(log N))
pub use crate::gpu::subvoxel::meshing::{
    OctreeMeshData, OctreeMeshContext, mesh_chunk_octree, mesh_chunk_octree_new,
};
