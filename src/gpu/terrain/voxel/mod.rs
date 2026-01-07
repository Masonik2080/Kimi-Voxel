// ============================================
// Voxel Module - Воксельная система
// ============================================

pub mod constants;
pub mod context;
pub mod thread_local;

mod greedy;
mod chunk;

pub use constants::{CHUNK_SIZE, MIN_HEIGHT};
pub use context::MeshingContext;
pub use chunk::{VoxelChunk, ChunkNeighbors, ChunkGenerationResult};

// Re-export для внутреннего использования
pub(crate) use greedy::{FaceDir, FaceInfo, greedy_mesh_layer_into, add_greedy_face};
