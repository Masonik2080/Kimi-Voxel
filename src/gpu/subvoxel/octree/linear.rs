// ============================================
// Linear Octree - Кэш-дружественное октодерево
// ============================================
//
// Все узлы хранятся в непрерывном Vec, ссылки через u32 индексы.
// Это устраняет pointer chasing и cache misses.

use crate::gpu::blocks::{BlockType, AIR};

/// Невалидный индекс (аналог null)
pub const INVALID_INDEX: u32 = u32::MAX;

/// Максимальная глубина (0=1 блок, 1=1/2, 2=1/4)
pub const MAX_DEPTH: u8 = 2;

/// Данные узла
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeData {
    /// Пустой (воздух)
    Empty,
    /// Заполнен одним типом
    Solid(BlockType),
    /// Смешанный - есть дети
    Branch,
}

/// Узел октодерева (32 байта, выровнен для кэша)
#[derive(Clone, Copy)]
#[repr(C)]
pub struct OctreeNode {
    /// Данные узла
    pub data: NodeData,
    /// Глубина (0 = корень)
    pub depth: u8,
    /// Индекс первого ребенка (дети идут подряд: first, first+1, ..., first+7)
    /// INVALID_INDEX если нет детей
    pub first_child: u32,
}

impl OctreeNode {
    #[inline]
    pub fn empty(depth: u8) -> Self {
        Self {
            data: NodeData::Empty,
            depth,
            first_child: INVALID_INDEX,
        }
    }

    #[inline]
    pub fn solid(block_type: BlockType, depth: u8) -> Self {
        Self {
            data: NodeData::Solid(block_type),
            depth,
            first_child: INVALID_INDEX,
        }
    }

    #[inline]
    pub fn branch(depth: u8, first_child: u32) -> Self {
        Self {
            data: NodeData::Branch,
            depth,
            first_child,
        }
    }

    #[inline]
    pub fn has_children(&self) -> bool {
        self.first_child != INVALID_INDEX
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        matches!(self.data, NodeData::Empty)
    }

    #[inline]
    pub fn is_solid(&self) -> bool {
        matches!(self.data, NodeData::Solid(_))
    }
}

impl Default for OctreeNode {
    fn default() -> Self {
        Self::empty(0)
    }
}

/// Linear Octree - все узлы в одном Vec
#[derive(Clone)]
pub struct LinearOctree {
    /// Плоский массив узлов
    nodes: Vec<OctreeNode>,
    /// Свободные слоты (для переиспользования)
    free_list: Vec<u32>,
}

impl LinearOctree {
    /// Создать пустое октодерево
    pub fn new() -> Self {
        let mut octree = Self {
            nodes: Vec::with_capacity(64),
            free_list: Vec::new(),
        };
        // Корень всегда на индексе 0
        octree.nodes.push(OctreeNode::empty(0));
        octree
    }

    /// Создать заполненное октодерево
    pub fn solid(block_type: BlockType) -> Self {
        let mut octree = Self {
            nodes: Vec::with_capacity(64),
            free_list: Vec::new(),
        };
        octree.nodes.push(OctreeNode::solid(block_type, 0));
        octree
    }

    /// Проверить пустоту
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.get(0).map(|n| n.is_empty()).unwrap_or(true)
    }

    /// Количество узлов
    #[inline]
    pub fn node_count(&self) -> usize {
        self.nodes.len() - self.free_list.len()
    }

    /// Установить субвоксель по дискретным координатам
    /// x, y, z: 0..divisions, divisions = 2^depth
    pub fn set_discrete(&mut self, x: u8, y: u8, z: u8, depth: u8, block_type: BlockType) {
        let target_depth = depth.min(MAX_DEPTH);
        self.set_at_node(0, x, y, z, 0, target_depth, block_type);
        self.try_simplify(0);
    }

    fn set_at_node(
        &mut self,
        node_idx: u32,
        x: u8, y: u8, z: u8,
        current_depth: u8,
        target_depth: u8,
        block_type: BlockType,
    ) {
        // Достигли целевой глубины
        if current_depth == target_depth {
            // Сначала проверяем и освобождаем детей
            let first_child = self.nodes[node_idx as usize].first_child;
            if first_child != INVALID_INDEX {
                self.free_children(first_child);
            }
            // Теперь обновляем узел
            let node = &mut self.nodes[node_idx as usize];
            node.first_child = INVALID_INDEX;
            node.data = if block_type == AIR {
                NodeData::Empty
            } else {
                NodeData::Solid(block_type)
            };
            return;
        }

        // Нужно спуститься глубже - создаем детей если нет
        let first_child = {
            let node = &self.nodes[node_idx as usize];
            if node.has_children() {
                node.first_child
            } else {
                // Создаем 8 детей с данными родителя
                let parent_data = node.data;
                let child_depth = current_depth + 1;
                let first = self.alloc_children(parent_data, child_depth);
                self.nodes[node_idx as usize].first_child = first;
                self.nodes[node_idx as usize].data = NodeData::Branch;
                first
            }
        };

        // Определяем какой ребенок
        let shift = target_depth - current_depth - 1;
        let lx = (x >> shift) & 1;
        let ly = (y >> shift) & 1;
        let lz = (z >> shift) & 1;
        let child_idx = first_child + Self::child_offset(lx, ly, lz);

        // Рекурсия
        self.set_at_node(child_idx, x, y, z, current_depth + 1, target_depth, block_type);
    }

    /// Получить субвоксель
    pub fn get_discrete(&self, x: u8, y: u8, z: u8, depth: u8) -> Option<BlockType> {
        let target_depth = depth.min(MAX_DEPTH);
        self.get_at_node(0, x, y, z, 0, target_depth)
    }

    fn get_at_node(
        &self,
        node_idx: u32,
        x: u8, y: u8, z: u8,
        current_depth: u8,
        target_depth: u8,
    ) -> Option<BlockType> {
        let node = &self.nodes[node_idx as usize];

        match node.data {
            NodeData::Empty => None,
            NodeData::Solid(bt) => Some(bt),
            NodeData::Branch => {
                if current_depth >= target_depth || !node.has_children() {
                    return None;
                }

                let shift = target_depth - current_depth - 1;
                let lx = (x >> shift) & 1;
                let ly = (y >> shift) & 1;
                let lz = (z >> shift) & 1;
                let child_idx = node.first_child + Self::child_offset(lx, ly, lz);

                self.get_at_node(child_idx, x, y, z, current_depth + 1, target_depth)
            }
        }
    }

    /// Удалить субвоксель
    #[inline]
    pub fn remove_discrete(&mut self, x: u8, y: u8, z: u8, depth: u8) {
        self.set_discrete(x, y, z, depth, AIR);
    }

    /// Попытаться упростить узел (если все дети одинаковые)
    fn try_simplify(&mut self, node_idx: u32) -> bool {
        let node = &self.nodes[node_idx as usize];
        if !node.has_children() {
            return false;
        }

        let first_child = node.first_child;

        // Сначала рекурсивно упрощаем детей
        for i in 0..8 {
            self.try_simplify(first_child + i);
        }

        // Проверяем все ли дети одинаковые листья
        let first_data = self.nodes[first_child as usize].data;
        if matches!(first_data, NodeData::Branch) {
            return false;
        }

        let all_same = (1..8).all(|i| {
            let child = &self.nodes[(first_child + i) as usize];
            !child.has_children() && child.data == first_data
        });

        if all_same {
            // Освобождаем детей
            self.free_children(first_child);
            // Упрощаем узел
            let node = &mut self.nodes[node_idx as usize];
            node.first_child = INVALID_INDEX;
            node.data = first_data;
            return true;
        }

        false
    }

    /// Аллоцировать 8 детей
    fn alloc_children(&mut self, data: NodeData, depth: u8) -> u32 {
        // Пытаемся переиспользовать из free_list (нужно 8 подряд)
        // Для простоты всегда аллоцируем новые
        let first = self.nodes.len() as u32;
        let child_node = OctreeNode {
            data,
            depth,
            first_child: INVALID_INDEX,
        };
        for _ in 0..8 {
            self.nodes.push(child_node);
        }
        first
    }

    /// Освободить 8 детей
    fn free_children(&mut self, first_child: u32) {
        // Рекурсивно освобождаем внуков
        for i in 0..8 {
            let child_idx = first_child + i;
            let child = &self.nodes[child_idx as usize];
            if child.has_children() {
                self.free_children(child.first_child);
            }
        }
        // Добавляем в free_list
        for i in 0..8 {
            self.free_list.push(first_child + i);
        }
    }

    /// Индекс ребенка по локальным координатам
    #[inline]
    fn child_offset(lx: u8, ly: u8, lz: u8) -> u32 {
        ((lz as u32) << 2) | ((ly as u32) << 1) | (lx as u32)
    }

    /// Локальные координаты из offset
    #[inline]
    pub fn offset_to_local(offset: u32) -> (u8, u8, u8) {
        let lx = (offset & 1) as u8;
        let ly = ((offset >> 1) & 1) as u8;
        let lz = ((offset >> 2) & 1) as u8;
        (lx, ly, lz)
    }

    /// Получить узел по индексу (для внешнего обхода)
    #[inline]
    pub fn get_node(&self, idx: u32) -> &OctreeNode {
        &self.nodes[idx as usize]
    }

    /// Проверка, есть ли solid воксель в заданной области
    /// x, y, z в нормализованных координатах [0, 1)
    /// size - размер проверяемой области
    pub fn is_solid_at(&self, x: f32, y: f32, z: f32, size: f32) -> bool {
        self.is_solid_at_node(0, x, y, z, size, 0.0, 0.0, 0.0, 1.0)
    }

    fn is_solid_at_node(
        &self,
        node_idx: u32,
        qx: f32, qy: f32, qz: f32, qsize: f32,
        nx: f32, ny: f32, nz: f32, nsize: f32,
    ) -> bool {
        // Проверяем пересечение AABB
        if qx + qsize <= nx || qx >= nx + nsize ||
           qy + qsize <= ny || qy >= ny + nsize ||
           qz + qsize <= nz || qz >= nz + nsize {
            return false;
        }

        let node = &self.nodes[node_idx as usize];

        match node.data {
            NodeData::Empty => false,
            NodeData::Solid(_) => true,
            NodeData::Branch => {
                if !node.has_children() {
                    return false;
                }

                let half = nsize * 0.5;
                let first = node.first_child;

                for i in 0..8u32 {
                    let (lx, ly, lz) = Self::offset_to_local(i);
                    let cx = nx + lx as f32 * half;
                    let cy = ny + ly as f32 * half;
                    let cz = nz + lz as f32 * half;

                    if self.is_solid_at_node(first + i, qx, qy, qz, qsize, cx, cy, cz, half) {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Итератор по всем непустым субвокселям
    /// Возвращает (x, y, z, size, block_type) в нормализованных координатах [0, 1)
    pub fn iter_solid(&self) -> LinearOctreeIterator<'_> {
        LinearOctreeIterator::new(self)
    }

    /// Подсчет непустых вокселей
    pub fn count_solid(&self) -> usize {
        self.count_solid_at(0)
    }

    fn count_solid_at(&self, node_idx: u32) -> usize {
        let node = &self.nodes[node_idx as usize];
        match node.data {
            NodeData::Empty => 0,
            NodeData::Solid(_) => 1,
            NodeData::Branch => {
                if node.has_children() {
                    (0..8).map(|i| self.count_solid_at(node.first_child + i)).sum()
                } else {
                    0
                }
            }
        }
    }

    /// Raycast через октодерево - O(log N)
    /// Пропускает пустые поддеревья через Ray-AABB тест
    /// Возвращает (x, y, z, size, block_type, t, normal)
    pub fn raycast(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_t: f32,
    ) -> Option<OctreeRaycastHit> {
        if self.nodes.is_empty() || self.nodes[0].is_empty() {
            return None;
        }
        self.raycast_node(0, origin, direction, [0.0, 0.0, 0.0], 1.0, max_t)
    }

    fn raycast_node(
        &self,
        node_idx: u32,
        origin: [f32; 3],
        direction: [f32; 3],
        node_min: [f32; 3],
        size: f32,
        max_t: f32,
    ) -> Option<OctreeRaycastHit> {
        let node = &self.nodes[node_idx as usize];

        // Ray-AABB тест - пропускаем узел если луч не пересекает
        let node_max = [node_min[0] + size, node_min[1] + size, node_min[2] + size];
        let Some((t_entry, normal)) = octree_ray_aabb(origin, direction, node_min, node_max) else {
            return None;
        };

        if t_entry > max_t {
            return None;
        }

        match node.data {
            NodeData::Empty => None,
            NodeData::Solid(block_type) => Some(OctreeRaycastHit {
                x: node_min[0],
                y: node_min[1],
                z: node_min[2],
                size,
                block_type,
                t: t_entry.max(0.0),
                normal,
            }),
            NodeData::Branch => {
                if !node.has_children() {
                    return None;
                }

                let half = size * 0.5;
                let first = node.first_child;
                let mut closest: Option<OctreeRaycastHit> = None;

                // Собираем детей с их t_entry для сортировки
                let mut children: [(u32, f32); 8] = [(0, f32::MAX); 8];
                for i in 0..8u32 {
                    let (lx, ly, lz) = Self::offset_to_local(i);
                    let child_min = [
                        node_min[0] + lx as f32 * half,
                        node_min[1] + ly as f32 * half,
                        node_min[2] + lz as f32 * half,
                    ];
                    let child_max = [child_min[0] + half, child_min[1] + half, child_min[2] + half];
                    
                    let t = octree_ray_aabb(origin, direction, child_min, child_max)
                        .map(|(t, _)| t)
                        .unwrap_or(f32::MAX);
                    children[i as usize] = (i, t);
                }

                // Сортируем по расстоянию для early-exit
                children.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                for (i, t_child) in children {
                    // Early exit: если ближайший ребёнок дальше текущего hit - выходим
                    let current_max = closest.as_ref().map(|c| c.t).unwrap_or(max_t);
                    if t_child > current_max {
                        break;
                    }

                    let (lx, ly, lz) = Self::offset_to_local(i);
                    let child_min = [
                        node_min[0] + lx as f32 * half,
                        node_min[1] + ly as f32 * half,
                        node_min[2] + lz as f32 * half,
                    ];

                    if let Some(hit) = self.raycast_node(first + i, origin, direction, child_min, half, current_max) {
                        if closest.is_none() || hit.t < closest.as_ref().unwrap().t {
                            closest = Some(hit);
                        }
                    }
                }
                closest
            }
        }
    }
}

/// Результат raycast в октодереве
#[derive(Clone, Copy, Debug)]
pub struct OctreeRaycastHit {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub size: f32,
    pub block_type: BlockType,
    pub t: f32,
    pub normal: [f32; 3],
}

/// Ray-AABB intersection для октодерева
#[inline]
fn octree_ray_aabb(
    origin: [f32; 3],
    direction: [f32; 3],
    aabb_min: [f32; 3],
    aabb_max: [f32; 3],
) -> Option<(f32, [f32; 3])> {
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;
    let mut normal = [0.0f32; 3];

    for i in 0..3 {
        if direction[i].abs() < 1e-8 {
            if origin[i] < aabb_min[i] || origin[i] > aabb_max[i] {
                return None;
            }
        } else {
            let inv_d = 1.0 / direction[i];
            let mut t1 = (aabb_min[i] - origin[i]) * inv_d;
            let mut t2 = (aabb_max[i] - origin[i]) * inv_d;

            let mut n = [0.0f32; 3];
            n[i] = -inv_d.signum();

            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
                n[i] = inv_d.signum();
            }

            if t1 > t_min {
                t_min = t1;
                normal = n;
            }
            t_max = t_max.min(t2);

            if t_min > t_max {
                return None;
            }
        }
    }

    Some((t_min, normal))
}

impl Default for LinearOctree {
    fn default() -> Self {
        Self::new()
    }
}

/// Итератор по непустым субвокселям (stack-based, без аллокаций)
pub struct LinearOctreeIterator<'a> {
    octree: &'a LinearOctree,
    /// Стек: (node_idx, x, y, z, size)
    stack: Vec<(u32, f32, f32, f32, f32)>,
}

impl<'a> LinearOctreeIterator<'a> {
    fn new(octree: &'a LinearOctree) -> Self {
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

impl<'a> Iterator for LinearOctreeIterator<'a> {
    type Item = (f32, f32, f32, f32, BlockType); // x, y, z, size, block_type

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node_idx, x, y, z, size)) = self.stack.pop() {
            let node = &self.octree.nodes[node_idx as usize];

            match node.data {
                NodeData::Empty => continue,
                NodeData::Solid(bt) => {
                    return Some((x, y, z, size, bt));
                }
                NodeData::Branch => {
                    if node.has_children() {
                        let half = size * 0.5;
                        let first = node.first_child;
                        // Добавляем детей в обратном порядке для правильного обхода
                        for i in (0..8).rev() {
                            let (lx, ly, lz) = LinearOctree::offset_to_local(i);
                            let cx = x + lx as f32 * half;
                            let cy = y + ly as f32 * half;
                            let cz = z + lz as f32 * half;
                            let child = &self.octree.nodes[(first + i) as usize];
                            if !child.is_empty() {
                                self.stack.push((first + i, cx, cy, cz, half));
                            }
                        }
                    }
                }
            }
        }
        None
    }
}
