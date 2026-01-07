// ============================================
// Terrain Module - Гибридная генерация мира
// ============================================

pub mod generation;
pub mod mesh;
pub mod voxel;
pub mod cache;
pub mod gpu;
pub mod lod;
pub mod manager;
pub mod world_changes;

// Re-exports
pub use mesh::TerrainVertex;
pub use cache::ChunkKey;
pub use gpu::GpuChunkManager;
pub use voxel::{VoxelChunk, ChunkNeighbors, CHUNK_SIZE, MIN_HEIGHT};
pub use manager::{HybridTerrainManager, GeneratedMesh, GeneratedChunkData, SectionTerrainManager};
pub use generation::{get_height, get_lod_height, CaveParams, is_cave};
pub use world_changes::{WorldChanges, BlockPos};
