// ============================================
// Meshing Module - Greedy Meshing для субвокселей
// ============================================
//
// Три реализации:
// - chunk_mesher: Оригинальная с ChunkGrid (legacy)
// - octree_mesher: Прямой обход октодерева
// - mask_greedy: Битовые маски без сортировки (рекомендуется)
//
// Два формата вершин:
// - SubVoxelVertex: 36 байт (legacy)
// - PackedVertex: 8 байт (рекомендуется)

mod greedy;
mod chunk_grid;
mod chunk_mesher;
mod octree_mesher;
mod mask_greedy;
mod vertex;
mod packed_vertex;

// Legacy
pub use greedy::{FaceDir, FaceInfo, GreedyQuad, greedy_mesh_layer, greedy_mesh_layer_into};
pub use chunk_grid::{ChunkGrid, CHUNK_GRID_SIZE};
pub use chunk_mesher::{ChunkMeshData, ChunkMeshContext, mesh_chunk, mesh_chunk_new};
pub use vertex::SubVoxelVertex;

// Octree mesher
pub use octree_mesher::{OctreeMeshData, OctreeMeshContext, mesh_chunk_octree, mesh_chunk_octree_new};

// Optimized (рекомендуется)
pub use packed_vertex::{PackedVertex, NormalIndex, MicroVertex, ColorPalette, pack_color, unpack_color};
pub use mask_greedy::{MaskGreedyContext, VoxelAccess, greedy_mesh_masked, MASK_SIZE};
