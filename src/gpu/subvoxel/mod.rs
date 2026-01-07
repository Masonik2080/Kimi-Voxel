// ============================================
// SubVoxel Module - Система субвокселей
// ============================================
//
// Две реализации:
// 1. Legacy (subvoxel.rs, subvoxel_render.rs) - используется сейчас
// 2. Optimized (chunk/, octree/, meshing/) - новая архитектура
//
// Оптимизации в новой версии:
// - SparseChunkStorage: O(N) память вместо ~3.5 МБ на чанк
// - CompactOctree: 4 байта на узел вместо 16+
// - PackedVertex: 8 байт вместо 36
// - MaskGreedy: битовые маски без сортировки

pub mod octree;
pub mod chunk;
pub mod meshing;
pub mod components;
pub mod systems;
pub mod render;

// Legacy API (используется в текущем коде)
mod subvoxel;
pub mod subvoxel_render;

pub use subvoxel::{
    SubVoxelLevel, SubVoxelPos, SubVoxelStorage, SubVoxel, SubVoxelHit,
    world_to_subvoxel, subvoxel_intersects_player, placement_pos_from_hit,
};
pub use subvoxel_render::SubVoxelRenderer;

// Оптимизированный API (для миграции)
pub use components::{
    SubVoxelLevel as OptSubVoxelLevel,
    SubVoxelPos as OptSubVoxelPos,
    SubVoxelWorld,
};
pub use chunk::{SubVoxelChunkKey, SparseChunkStorage, PackedBlockKey};
pub use octree::{CompactOctree, CompactNode};
pub use meshing::{PackedVertex, MaskGreedyContext, greedy_mesh_masked};
pub use render::OptimizedSubVoxelRenderer;
pub use systems::{
    MeshingSystemContext, MeshingConfig, ChunkMesh,
    mark_chunk_dirty, process_meshing_queue, get_meshing_stats,
};
