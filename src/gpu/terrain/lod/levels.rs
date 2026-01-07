// ============================================
// LOD Levels - Уровни детализации
// ============================================

#[derive(Clone, Copy)]
pub struct LodLevel {
    pub min_chunks: i32,
    pub max_chunks: i32,
    pub scale: i32,
}

impl LodLevel {
    pub const DEFAULT_LEVELS: [LodLevel; 4] = [
        LodLevel { min_chunks: 0, max_chunks: 8, scale: 1 },
        LodLevel { min_chunks: 8, max_chunks: 16, scale: 2 },
        LodLevel { min_chunks: 16, max_chunks: 32, scale: 4 },
        LodLevel { min_chunks: 32, max_chunks: 64, scale: 8 },
    ];
}
