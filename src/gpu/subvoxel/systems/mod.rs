// ============================================
// SubVoxel Systems - ECS системы (ОПТИМИЗИРОВАННЫЕ)
// ============================================

mod placement;
mod raycast;
mod mesh;
mod mesh_system;

pub use placement::{world_to_subvoxel_pos, placement_pos_from_hit};
pub use raycast::{SubVoxelHit, subvoxel_raycast};

// Legacy mesher (36 байт вершины, ChunkGrid декомпрессия)
pub use mesh::{ChunkMeshData, ChunkMeshContext, mesh_chunk, mesh_chunk_new, SubVoxelVertex};

// Оптимизированный mesher (8 байт вершины, mask greedy)
pub use mesh_system::{
    // Компоненты
    DirtyChunk, ChunkMesh,
    // Ресурсы
    MeshingConfig, MeshingSystemContext,
    // Системы
    mark_chunk_dirty, update_priorities, process_meshing_queue,
    get_chunk_mesh, get_all_meshes, remove_chunk_mesh, clear_all_meshes,
    // Статистика
    MeshingStats, get_meshing_stats,
};
