// ============================================
// Chunk Key - Ключ чанка для субвокселей
// ============================================

/// Ключ чанка (координаты чанка)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SubVoxelChunkKey {
    pub x: i32,
    pub z: i32,
}

impl SubVoxelChunkKey {
    #[inline]
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Из мировых координат блока
    #[inline]
    pub fn from_block_pos(block_x: i32, block_z: i32) -> Self {
        Self {
            x: block_x.div_euclid(16),
            z: block_z.div_euclid(16),
        }
    }

    /// Из мировых координат (float)
    #[inline]
    pub fn from_world_pos(world_x: f32, world_z: f32) -> Self {
        Self::from_block_pos(world_x.floor() as i32, world_z.floor() as i32)
    }
}
