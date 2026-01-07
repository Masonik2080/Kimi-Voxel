// ============================================
// Compact Octree - Минимальный размер узла
// ============================================
//
// Узел занимает 4 байта вместо 16+:
// - 1 байт: тип данных (Empty/Solid/Branch) + BlockType
// - 1 байт: child_mask (какие дети существуют)
// - 2 байта: offset к первому ребёнку
//
// Дети хранятся компактно: только существующие, без пустых слотов.

use crate::gpu::blocks::{BlockType, AIR};

/// Максимальная глубина (0=1 блок, 1=1/2, 2=1/4)
pub const MAX_DEPTH: u8 = 2;

/// Компактный узел октодерева (4 байта)
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct CompactNode {
    /// Биты 0-5: BlockType (0 = Empty, 1-63 = solid types)
    /// Бит 6: is_branch
    /// Бит 7: reserved
    pub data: u8,
    /// Битовая маска детей (какие из 8 существуют)
    pub child_mask: u8,
    /// Offset к первому ребёнку в массиве узлов
    pub child_offset: u16,
}

impl CompactNode {
    pub const EMPTY: Self = Self {
        data: 0,
        child_mask: 0,
        child_offset: 0,
    };

    #[inline]
    pub fn solid(block_type: BlockType) -> Self {
        Self {
            data: block_type as u8,
            child_mask: 0,
            child_offset: 0,
        }
    }

    #[inline]
    pub fn branch(child_mask: u8, child_offset: u16) -> Self {
        Self {
            data: 0x40, // is_branch bit
            child_mask,
            child_offset,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data == 0 && self.child_mask == 0
    }

    #[inline]
    pub fn is_solid(&self) -> bool {
        self.data != 0 && (self.data & 0x40) == 0
    }

    #[inline]
    pub fn is_branch(&self) -> bool {
        (self.data & 0x40) != 0
    }

    #[inline]
    pub fn block_type(&self) -> Option<BlockType> {
        if self.is_solid() {
            Some(self.data & 0x3F)
        } else {
            None
        }
    }

    #[inline]
    pub fn child_count(&self) -> u8 {
        self.child_mask.count_ones() as u8
    }

    /// Индекс ребёнка в компактном массиве (только существующие дети)
    #[inline]
    pub fn child_index(&self, octant: u8) -> Option<u16> {
        if (self.child_mask & (1 << octant)) == 0 {
            return None;
        }
        // Считаем сколько детей до этого октанта
        let mask_before = self.child_mask & ((1 << octant) - 1);
        Some(self.child_offset + mask_before.count_ones() as u16)
    }
}

/// Компактное октодерево
/// Все узлы в одном Vec, дети хранятся компактно
pub struct CompactOctree {
    nodes: Vec<CompactNode>,
}

impl CompactOctree {
    pub fn new() -> Self {
        Self {
            nodes: vec![CompactNode::EMPTY],
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.len() == 1 && self.nodes[0].is_empty()
    }

    #[inline]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Память в байтах
    #[inline]
    pub fn memory_usage(&self) -> usize {
        self.nodes.len() * 4
    }

    /// Установить субвоксель
    pub fn set(&mut self, x: u8, y: u8, z: u8, depth: u8, block_type: BlockType) {
        let target_depth = depth.min(MAX_DEPTH);
        self.set_recursive(0, x, y, z, 0, target_depth, block_type);
        self.try_simplify(0);
    }

    fn set_recursive(
        &mut self,
        node_idx: u16,
        x: u8, y: u8, z: u8,
        current_depth: u8,
        target_depth: u8,
        block_type: BlockType,
    ) {
        if current_depth == target_depth {
            // Достигли целевой глубины
            if block_type == AIR {
                self.nodes[node_idx as usize] = CompactNode::EMPTY;
            } else {
                self.nodes[node_idx as usize] = CompactNode::solid(block_type);
            }
            return;
        }

        // Определяем октант
        let shift = target_depth - current_depth - 1;
        let octant = ((x >> shift) & 1) | (((y >> shift) & 1) << 1) | (((z >> shift) & 1) << 2);

        // Нужно спуститься - создаём детей если нет
        let node = self.nodes[node_idx as usize];
        
        if !node.is_branch() {
            // Превращаем в branch, создаём детей
            let parent_data = node.data;
            let child_offset = self.nodes.len() as u16;
            
            // Создаём всех 8 детей с данными родителя
            for _ in 0..8 {
                if parent_data == 0 {
                    self.nodes.push(CompactNode::EMPTY);
                } else {
                    self.nodes.push(CompactNode::solid(parent_data & 0x3F));
                }
            }
            
            self.nodes[node_idx as usize] = CompactNode::branch(0xFF, child_offset);
        }

        let node = self.nodes[node_idx as usize];
        if let Some(child_idx) = node.child_index(octant) {
            self.set_recursive(child_idx, x, y, z, current_depth + 1, target_depth, block_type);
        }
    }

    /// Получить субвоксель
    pub fn get(&self, x: u8, y: u8, z: u8, depth: u8) -> Option<BlockType> {
        let target_depth = depth.min(MAX_DEPTH);
        self.get_recursive(0, x, y, z, 0, target_depth)
    }

    fn get_recursive(
        &self,
        node_idx: u16,
        x: u8, y: u8, z: u8,
        current_depth: u8,
        target_depth: u8,
    ) -> Option<BlockType> {
        let node = self.nodes[node_idx as usize];

        if node.is_empty() {
            return None;
        }

        if node.is_solid() {
            return node.block_type();
        }

        if current_depth >= target_depth {
            return None;
        }

        // Branch - спускаемся
        let shift = target_depth - current_depth - 1;
        let octant = ((x >> shift) & 1) | (((y >> shift) & 1) << 1) | (((z >> shift) & 1) << 2);

        let child_idx = node.child_index(octant)?;
        self.get_recursive(child_idx, x, y, z, current_depth + 1, target_depth)
    }

    /// Удалить субвоксель
    #[inline]
    pub fn remove(&mut self, x: u8, y: u8, z: u8, depth: u8) {
        self.set(x, y, z, depth, AIR);
    }

    /// Упростить дерево (объединить одинаковых детей)
    fn try_simplify(&mut self, node_idx: u16) -> bool {
        let node = self.nodes[node_idx as usize];
        
        if !node.is_branch() || node.child_mask == 0 {
            return false;
        }

        // Рекурсивно упрощаем детей
        for octant in 0..8u8 {
            if let Some(child_idx) = node.child_index(octant) {
                self.try_simplify(child_idx);
            }
        }

        // Проверяем все ли дети одинаковые solid
        let first_child_idx = node.child_offset;
        let first = self.nodes[first_child_idx as usize];
        
        if !first.is_solid() && !first.is_empty() {
            return false;
        }

        let all_same = (1..8u8).all(|i| {
            if let Some(child_idx) = node.child_index(i) {
                let child = self.nodes[child_idx as usize];
                child.data == first.data && !child.is_branch()
            } else {
                false
            }
        });

        if all_same {
            self.nodes[node_idx as usize] = first;
            return true;
        }

        false
    }

    /// Итератор по solid субвокселям
    pub fn iter_solid(&self) -> CompactOctreeIterator<'_> {
        CompactOctreeIterator::new(self)
    }

    /// Проверка solid в области (для culling)
    pub fn is_solid_at(&self, x: f32, y: f32, z: f32, size: f32) -> bool {
        self.is_solid_at_recursive(0, x, y, z, size, 0.0, 0.0, 0.0, 1.0)
    }

    fn is_solid_at_recursive(
        &self,
        node_idx: u16,
        qx: f32, qy: f32, qz: f32, qsize: f32,
        nx: f32, ny: f32, nz: f32, nsize: f32,
    ) -> bool {
        // AABB test
        if qx + qsize <= nx || qx >= nx + nsize ||
           qy + qsize <= ny || qy >= ny + nsize ||
           qz + qsize <= nz || qz >= nz + nsize {
            return false;
        }

        let node = self.nodes[node_idx as usize];

        if node.is_empty() {
            return false;
        }

        if node.is_solid() {
            return true;
        }

        // Branch
        let half = nsize * 0.5;
        for octant in 0..8u8 {
            if let Some(child_idx) = node.child_index(octant) {
                let cx = nx + ((octant & 1) as f32) * half;
                let cy = ny + (((octant >> 1) & 1) as f32) * half;
                let cz = nz + (((octant >> 2) & 1) as f32) * half;

                if self.is_solid_at_recursive(child_idx, qx, qy, qz, qsize, cx, cy, cz, half) {
                    return true;
                }
            }
        }

        false
    }
}

impl Default for CompactOctree {
    fn default() -> Self {
        Self::new()
    }
}

/// Итератор по solid субвокселям
pub struct CompactOctreeIterator<'a> {
    octree: &'a CompactOctree,
    stack: Vec<(u16, f32, f32, f32, f32)>, // (node_idx, x, y, z, size)
}

impl<'a> CompactOctreeIterator<'a> {
    fn new(octree: &'a CompactOctree) -> Self {
        let mut iter = Self {
            octree,
            stack: Vec::with_capacity(32),
        };
        if !octree.nodes.is_empty() && !octree.nodes[0].is_empty() {
            iter.stack.push((0, 0.0, 0.0, 0.0, 1.0));
        }
        iter
    }
}

impl<'a> Iterator for CompactOctreeIterator<'a> {
    type Item = (f32, f32, f32, f32, BlockType); // x, y, z, size, block_type

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node_idx, x, y, z, size)) = self.stack.pop() {
            let node = self.octree.nodes[node_idx as usize];

            if node.is_empty() {
                continue;
            }

            if let Some(bt) = node.block_type() {
                return Some((x, y, z, size, bt));
            }

            if node.is_branch() {
                let half = size * 0.5;
                for octant in (0..8u8).rev() {
                    if let Some(child_idx) = node.child_index(octant) {
                        let cx = x + ((octant & 1) as f32) * half;
                        let cy = y + (((octant >> 1) & 1) as f32) * half;
                        let cz = z + (((octant >> 2) & 1) as f32) * half;
                        
                        let child = self.octree.nodes[child_idx as usize];
                        if !child.is_empty() {
                            self.stack.push((child_idx, cx, cy, cz, half));
                        }
                    }
                }
            }
        }
        None
    }
}
