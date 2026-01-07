// ============================================
// Chunk Key - Идентификатор чанка
// ============================================

/// Ключ чанка: (chunk_x, chunk_z, lod_scale)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChunkKey {
    pub x: i32,
    pub z: i32,
    pub scale: i32,
}

impl ChunkKey {
    pub fn new(x: i32, z: i32, scale: i32) -> Self {
        Self { x, z, scale }
    }
    
    /// Создать ключ для секции (section_y = 0..16)
    pub fn new_section(chunk_x: i32, chunk_z: i32, section_y: i32) -> Self {
        Self { x: chunk_x, z: chunk_z, scale: 1000 + section_y }
    }
}
