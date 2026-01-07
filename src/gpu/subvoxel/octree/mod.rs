// ============================================
// Octree Module - Compact Octree для субвокселей
// ============================================
//
// Две реализации:
// - LinearOctree: Оригинальная (для совместимости)
// - CompactOctree: Оптимизированная (4 байта на узел)

mod linear;
mod compact;

pub use linear::{LinearOctree, OctreeNode, NodeData, LinearOctreeIterator, OctreeRaycastHit, MAX_DEPTH, INVALID_INDEX};
pub use compact::{CompactOctree, CompactNode, CompactOctreeIterator};
