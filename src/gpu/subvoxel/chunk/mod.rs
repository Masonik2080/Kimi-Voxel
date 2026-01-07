// ============================================
// Chunk Module - Чанковое хранение субвокселей
// ============================================
//
// Две реализации:
// - ChunkSubVoxelStorage: Оригинальная (плоский массив, ~3.5 МБ на чанк)
// - SparseChunkStorage: Оптимизированная (HashMap, O(N) память)

mod key;
mod storage;
mod sparse_storage;

pub use key::SubVoxelChunkKey;
pub use storage::{
    ChunkSubVoxelStorage, LocalBlockKey, RaycastHit,
    CHUNK_SIZE, CHUNK_HEIGHT, STORAGE_SIZE,
};
pub use sparse_storage::{SparseChunkStorage, PackedBlockKey};
