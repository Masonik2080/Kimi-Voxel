// ============================================
// Octree Mesher - Greedy Meshing напрямую по октодереву
// ============================================
//
// Без промежуточной декомпрессии в ChunkGrid.
// Работает напрямую с иерархией октодерева.
//
// Ключевые оптимизации:
// 1. Пропуск пустых поддеревьев (O(log N) вместо O(N))
// 2. Нет memory traffic от декомпрессии
// 3. Greedy на уровне листьев октодерева
// 4. Culling через соседние узлы

use crate::gpu::blocks::{BlockType, get_face_colors};
use crate::gpu::subvoxel::chunk::{ChunkSubVoxelStorage, SubVoxelChunkKey};
use crate::gpu::subvoxel::octree::{LinearOctree, NodeData};
use super::vertex::SubVoxelVertex;

/// Данные меша чанка (без декомпрессии)
#[derive(Default)]
pub struct OctreeMeshData {
    pub vertices: Vec<SubVoxelVertex>,
    pub indices: Vec<u32>,
    pub chunk_key: SubVoxelChunkKey,
    pub version: u64,
}

/// Контекст для octree meshing (переиспользуемые буферы)
pub struct OctreeMeshContext {
    /// Собранные грани для greedy
    faces: Vec<OctreeFace>,
    /// Выходные буферы
    vertices: Vec<SubVoxelVertex>,
    indices: Vec<u32>,
    /// Visited для greedy
    visited: Vec<bool>,
}

/// Грань субвокселя для greedy meshing
#[derive(Clone, Copy, Debug)]
struct OctreeFace {
    /// Мировые координаты грани
    x: f32,
    y: f32,
    z: f32,
    /// Размер грани
    size: f32,
    /// Тип блока
    block_type: BlockType,
    /// Направление грани
    dir: FaceDir,
    /// Координата по оси грани (для сортировки)
    axis_coord: f32,
}

/// Направление грани
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FaceDir {
    PosX, NegX,
    PosY, NegY,
    PosZ, NegZ,
}

impl FaceDir {
    #[inline]
    pub fn normal(&self) -> [f32; 3] {
        match self {
            FaceDir::PosX => [1.0, 0.0, 0.0],
            FaceDir::NegX => [-1.0, 0.0, 0.0],
            FaceDir::PosY => [0.0, 1.0, 0.0],
            FaceDir::NegY => [0.0, -1.0, 0.0],
            FaceDir::PosZ => [0.0, 0.0, 1.0],
            FaceDir::NegZ => [0.0, 0.0, -1.0],
        }
    }

    #[inline]
    pub fn axis(&self) -> usize {
        match self {
            FaceDir::PosX | FaceDir::NegX => 0,
            FaceDir::PosY | FaceDir::NegY => 1,
            FaceDir::PosZ | FaceDir::NegZ => 2,
        }
    }

    #[inline]
    pub fn is_positive(&self) -> bool {
        matches!(self, FaceDir::PosX | FaceDir::PosY | FaceDir::PosZ)
    }
}

impl OctreeMeshContext {
    pub fn new() -> Self {
        Self {
            faces: Vec::with_capacity(4096),
            vertices: Vec::with_capacity(8192),
            indices: Vec::with_capacity(16384),
            visited: Vec::with_capacity(4096),
        }
    }

    fn clear(&mut self) {
        self.faces.clear();
        self.vertices.clear();
        self.indices.clear();
    }
}

impl Default for OctreeMeshContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================
// Главная функция мешинга
// ============================================

/// Генерация меша напрямую из октодеревьев (без ChunkGrid)
pub fn mesh_chunk_octree(
    storage: &ChunkSubVoxelStorage,
    chunk_key: SubVoxelChunkKey,
    ctx: &mut OctreeMeshContext,
) -> OctreeMeshData {
    ctx.clear();

    if storage.is_empty() {
        return OctreeMeshData {
            vertices: Vec::new(),
            indices: Vec::new(),
            chunk_key,
            version: storage.version(),
        };
    }

    let base_x = (chunk_key.x * 16) as f32;
    let base_z = (chunk_key.z * 16) as f32;

    // Собираем все видимые грани из октодеревьев
    collect_visible_faces(storage, base_x, base_z, &mut ctx.faces);

    // Greedy meshing по собранным граням
    greedy_mesh_faces(&mut ctx.faces, &mut ctx.visited, &mut ctx.vertices, &mut ctx.indices);

    OctreeMeshData {
        vertices: std::mem::take(&mut ctx.vertices),
        indices: std::mem::take(&mut ctx.indices),
        chunk_key,
        version: storage.version(),
    }
}

// ============================================
// Сбор видимых граней из октодеревьев
// ============================================

/// Собирает видимые грани из всех октодеревьев чанка
fn collect_visible_faces(
    storage: &ChunkSubVoxelStorage,
    base_x: f32,
    base_z: f32,
    faces: &mut Vec<OctreeFace>,
) {
    for (key, octree) in storage.iter_blocks() {
        let block_x = base_x + key.x as f32;
        let block_y = key.y as f32;
        let block_z = base_z + key.z as f32;

        // Получаем соседние октодеревья для culling
        let neighbors = BlockNeighbors {
            pos_x: storage.get_block_octree(key.x.wrapping_add(1), key.y, key.z),
            neg_x: if key.x > 0 { storage.get_block_octree(key.x - 1, key.y, key.z) } else { None },
            pos_y: storage.get_block_octree(key.x, key.y.wrapping_add(1), key.z),
            neg_y: if key.y > 0 { storage.get_block_octree(key.x, key.y - 1, key.z) } else { None },
            pos_z: storage.get_block_octree(key.x, key.y, key.z.wrapping_add(1)),
            neg_z: if key.z > 0 { storage.get_block_octree(key.x, key.y, key.z - 1) } else { None },
        };

        // Рекурсивно обходим октодерево
        collect_faces_recursive(
            octree,
            0, // root node
            block_x, block_y, block_z,
            1.0, // size = 1 block
            &neighbors,
            faces,
        );
    }
}

/// Соседние октодеревья для culling
struct BlockNeighbors<'a> {
    pos_x: Option<&'a LinearOctree>,
    neg_x: Option<&'a LinearOctree>,
    pos_y: Option<&'a LinearOctree>,
    neg_y: Option<&'a LinearOctree>,
    pos_z: Option<&'a LinearOctree>,
    neg_z: Option<&'a LinearOctree>,
}

/// Рекурсивный обход октодерева для сбора граней
fn collect_faces_recursive(
    octree: &LinearOctree,
    node_idx: u32,
    x: f32, y: f32, z: f32,
    size: f32,
    neighbors: &BlockNeighbors,
    faces: &mut Vec<OctreeFace>,
) {
    let node = octree.get_node(node_idx);

    match node.data {
        NodeData::Empty => {
            // Пустой узел - пропускаем всё поддерево
            return;
        }
        NodeData::Solid(block_type) => {
            // Листовой узел - генерируем грани с culling
            generate_leaf_faces(x, y, z, size, block_type, neighbors, octree, faces);
        }
        NodeData::Branch => {
            if !node.has_children() {
                return;
            }

            let half = size * 0.5;
            let first_child = node.first_child;

            // Рекурсивно обходим детей
            for i in 0..8u32 {
                let (lx, ly, lz) = child_offset_to_local(i);
                let child_x = x + lx as f32 * half;
                let child_y = y + ly as f32 * half;
                let child_z = z + lz as f32 * half;

                collect_faces_recursive(
                    octree,
                    first_child + i,
                    child_x, child_y, child_z,
                    half,
                    neighbors,
                    faces,
                );
            }
        }
    }
}

/// Генерация граней для листового узла с culling
fn generate_leaf_faces(
    x: f32, y: f32, z: f32,
    size: f32,
    block_type: BlockType,
    neighbors: &BlockNeighbors,
    octree: &LinearOctree,
    faces: &mut Vec<OctreeFace>,
) {
    // +X грань
    if !is_occluded_positive_x(x, y, z, size, octree, neighbors.pos_x) {
        faces.push(OctreeFace {
            x: x + size, y, z,
            size,
            block_type,
            dir: FaceDir::PosX,
            axis_coord: x + size,
        });
    }

    // -X грань
    if !is_occluded_negative_x(x, y, z, size, octree, neighbors.neg_x) {
        faces.push(OctreeFace {
            x, y, z,
            size,
            block_type,
            dir: FaceDir::NegX,
            axis_coord: x,
        });
    }

    // +Y грань
    if !is_occluded_positive_y(x, y, z, size, octree, neighbors.pos_y) {
        faces.push(OctreeFace {
            x, y: y + size, z,
            size,
            block_type,
            dir: FaceDir::PosY,
            axis_coord: y + size,
        });
    }

    // -Y грань
    if !is_occluded_negative_y(x, y, z, size, octree, neighbors.neg_y) {
        faces.push(OctreeFace {
            x, y, z,
            size,
            block_type,
            dir: FaceDir::NegY,
            axis_coord: y,
        });
    }

    // +Z грань
    if !is_occluded_positive_z(x, y, z, size, octree, neighbors.pos_z) {
        faces.push(OctreeFace {
            x, y, z: z + size,
            size,
            block_type,
            dir: FaceDir::PosZ,
            axis_coord: z + size,
        });
    }

    // -Z грань
    if !is_occluded_negative_z(x, y, z, size, octree, neighbors.neg_z) {
        faces.push(OctreeFace {
            x, y, z,
            size,
            block_type,
            dir: FaceDir::NegZ,
            axis_coord: z,
        });
    }
}

// ============================================
// Функции проверки окклюзии (culling)
// ============================================

/// Проверка окклюзии +X грани
#[inline]
fn is_occluded_positive_x(
    x: f32, y: f32, z: f32,
    size: f32,
    octree: &LinearOctree,
    neighbor: Option<&LinearOctree>,
) -> bool {
    let check_x = x + size;
    
    // Внутри блока
    if check_x < 1.0 {
        return octree.is_solid_at(check_x, y, z, size);
    }
    
    // В соседнем блоке
    if let Some(neighbor_octree) = neighbor {
        return neighbor_octree.is_solid_at(0.0, y, z, size);
    }
    
    false
}

/// Проверка окклюзии -X грани
#[inline]
fn is_occluded_negative_x(
    x: f32, y: f32, z: f32,
    size: f32,
    octree: &LinearOctree,
    neighbor: Option<&LinearOctree>,
) -> bool {
    // Внутри блока
    if x > 0.0 {
        return octree.is_solid_at(x - size, y, z, size);
    }
    
    // В соседнем блоке
    if let Some(neighbor_octree) = neighbor {
        return neighbor_octree.is_solid_at(1.0 - size, y, z, size);
    }
    
    false
}

/// Проверка окклюзии +Y грани
#[inline]
fn is_occluded_positive_y(
    x: f32, y: f32, z: f32,
    size: f32,
    octree: &LinearOctree,
    neighbor: Option<&LinearOctree>,
) -> bool {
    let check_y = y + size;
    
    if check_y < 1.0 {
        return octree.is_solid_at(x, check_y, z, size);
    }
    
    if let Some(neighbor_octree) = neighbor {
        return neighbor_octree.is_solid_at(x, 0.0, z, size);
    }
    
    false
}

/// Проверка окклюзии -Y грани
#[inline]
fn is_occluded_negative_y(
    x: f32, y: f32, z: f32,
    size: f32,
    octree: &LinearOctree,
    neighbor: Option<&LinearOctree>,
) -> bool {
    if y > 0.0 {
        return octree.is_solid_at(x, y - size, z, size);
    }
    
    if let Some(neighbor_octree) = neighbor {
        return neighbor_octree.is_solid_at(x, 1.0 - size, z, size);
    }
    
    false
}

/// Проверка окклюзии +Z грани
#[inline]
fn is_occluded_positive_z(
    x: f32, y: f32, z: f32,
    size: f32,
    octree: &LinearOctree,
    neighbor: Option<&LinearOctree>,
) -> bool {
    let check_z = z + size;
    
    if check_z < 1.0 {
        return octree.is_solid_at(x, y, check_z, size);
    }
    
    if let Some(neighbor_octree) = neighbor {
        return neighbor_octree.is_solid_at(x, y, 0.0, size);
    }
    
    false
}

/// Проверка окклюзии -Z грани
#[inline]
fn is_occluded_negative_z(
    x: f32, y: f32, z: f32,
    size: f32,
    octree: &LinearOctree,
    neighbor: Option<&LinearOctree>,
) -> bool {
    if z > 0.0 {
        return octree.is_solid_at(x, y, z - size, size);
    }
    
    if let Some(neighbor_octree) = neighbor {
        return neighbor_octree.is_solid_at(x, y, 1.0 - size, size);
    }
    
    false
}

// ============================================
// Greedy Meshing по собранным граням
// ============================================

/// Greedy meshing по собранным граням
fn greedy_mesh_faces(
    faces: &mut Vec<OctreeFace>,
    visited: &mut Vec<bool>,
    vertices: &mut Vec<SubVoxelVertex>,
    indices: &mut Vec<u32>,
) {
    if faces.is_empty() {
        return;
    }

    // Сортируем грани по направлению и координате оси
    faces.sort_by(|a, b| {
        match (a.dir as u8).cmp(&(b.dir as u8)) {
            std::cmp::Ordering::Equal => {
                a.axis_coord.partial_cmp(&b.axis_coord).unwrap_or(std::cmp::Ordering::Equal)
            }
            other => other,
        }
    });

    // Greedy для каждого направления отдельно
    let mut start = 0;
    while start < faces.len() {
        let dir = faces[start].dir;
        let mut end = start + 1;
        
        // Находим конец группы с одинаковым направлением
        while end < faces.len() && faces[end].dir == dir {
            end += 1;
        }

        // Greedy mesh для этой группы
        greedy_mesh_face_group(&faces[start..end], visited, vertices, indices, dir);
        
        start = end;
    }
}

/// Greedy meshing для группы граней одного направления
fn greedy_mesh_face_group(
    faces: &[OctreeFace],
    visited: &mut Vec<bool>,
    vertices: &mut Vec<SubVoxelVertex>,
    indices: &mut Vec<u32>,
    dir: FaceDir,
) {
    if faces.is_empty() {
        return;
    }

    // Подготавливаем visited
    visited.clear();
    visited.resize(faces.len(), false);

    // Группируем по axis_coord (слои)
    let mut layer_start = 0;
    while layer_start < faces.len() {
        let axis_coord = faces[layer_start].axis_coord;
        let mut layer_end = layer_start + 1;
        
        while layer_end < faces.len() && 
              (faces[layer_end].axis_coord - axis_coord).abs() < 0.001 {
            layer_end += 1;
        }

        // Greedy для этого слоя
        greedy_mesh_layer_direct(
            &faces[layer_start..layer_end],
            &mut visited[layer_start..layer_end],
            vertices,
            indices,
            dir,
        );

        layer_start = layer_end;
    }
}

/// Greedy meshing для одного слоя граней
fn greedy_mesh_layer_direct(
    faces: &[OctreeFace],
    visited: &mut [bool],
    vertices: &mut Vec<SubVoxelVertex>,
    indices: &mut Vec<u32>,
    dir: FaceDir,
) {
    for i in 0..faces.len() {
        if visited[i] {
            continue;
        }

        let face = &faces[i];
        visited[i] = true;

        // Пытаемся расширить грань
        let (merged_width, merged_height) = try_merge_faces(
            faces, visited, i, face, dir
        );

        // Генерируем квад
        emit_quad(vertices, indices, face, merged_width, merged_height, dir);
    }
}

/// Попытка объединить соседние грани
fn try_merge_faces(
    faces: &[OctreeFace],
    visited: &mut [bool],
    start_idx: usize,
    start_face: &OctreeFace,
    dir: FaceDir,
) -> (f32, f32) {
    let mut width = start_face.size;
    let mut height = start_face.size;

    // Определяем оси для расширения в зависимости от направления грани
    let (u_axis, v_axis) = match dir {
        FaceDir::PosX | FaceDir::NegX => (2, 1), // Z, Y
        FaceDir::PosY | FaceDir::NegY => (0, 2), // X, Z
        FaceDir::PosZ | FaceDir::NegZ => (0, 1), // X, Y
    };

    let start_pos = [start_face.x, start_face.y, start_face.z];

    // Расширяем по U
    'expand_u: loop {
        let next_u = start_pos[u_axis] + width;
        
        for j in (start_idx + 1)..faces.len() {
            if visited[j] {
                continue;
            }
            
            let other = &faces[j];
            if other.block_type != start_face.block_type || other.size != start_face.size {
                continue;
            }

            let other_pos = [other.x, other.y, other.z];
            
            // Проверяем что грань примыкает по U
            if (other_pos[u_axis] - next_u).abs() < 0.001 &&
               (other_pos[v_axis] - start_pos[v_axis]).abs() < 0.001 {
                visited[j] = true;
                width += other.size;
                continue 'expand_u;
            }
        }
        break;
    }

    // Расширяем по V
    'expand_v: loop {
        let next_v = start_pos[v_axis] + height;
        let mut row_complete = true;
        let mut row_faces = Vec::new();

        // Проверяем всю строку
        let mut check_u = start_pos[u_axis];
        while check_u < start_pos[u_axis] + width {
            let mut found = false;
            
            for j in (start_idx + 1)..faces.len() {
                if visited[j] {
                    continue;
                }
                
                let other = &faces[j];
                if other.block_type != start_face.block_type || other.size != start_face.size {
                    continue;
                }

                let other_pos = [other.x, other.y, other.z];
                
                if (other_pos[u_axis] - check_u).abs() < 0.001 &&
                   (other_pos[v_axis] - next_v).abs() < 0.001 {
                    row_faces.push(j);
                    found = true;
                    check_u += other.size;
                    break;
                }
            }
            
            if !found {
                row_complete = false;
                break;
            }
        }

        if row_complete && !row_faces.is_empty() {
            for j in row_faces {
                visited[j] = true;
            }
            height += start_face.size;
        } else {
            break 'expand_v;
        }
    }

    (width, height)
}

/// Генерация квада
fn emit_quad(
    vertices: &mut Vec<SubVoxelVertex>,
    indices: &mut Vec<u32>,
    face: &OctreeFace,
    width: f32,
    height: f32,
    dir: FaceDir,
) {
    let base = vertices.len() as u32;
    let normal = dir.normal();
    let (top_color, side_color) = get_face_colors(face.block_type);
    
    let color = if matches!(dir, FaceDir::PosY) {
        top_color
    } else if matches!(dir, FaceDir::NegY) {
        [side_color[0] * 0.5, side_color[1] * 0.5, side_color[2] * 0.5]
    } else {
        side_color
    };

    let (x, y, z) = (face.x, face.y, face.z);

    match dir {
        FaceDir::PosX => {
            vertices.push(SubVoxelVertex::new([x, y, z], normal, color));
            vertices.push(SubVoxelVertex::new([x, y, z + width], normal, color));
            vertices.push(SubVoxelVertex::new([x, y + height, z + width], normal, color));
            vertices.push(SubVoxelVertex::new([x, y + height, z], normal, color));
        }
        FaceDir::NegX => {
            vertices.push(SubVoxelVertex::new([x, y, z], normal, color));
            vertices.push(SubVoxelVertex::new([x, y + height, z], normal, color));
            vertices.push(SubVoxelVertex::new([x, y + height, z + width], normal, color));
            vertices.push(SubVoxelVertex::new([x, y, z + width], normal, color));
        }
        FaceDir::PosY => {
            vertices.push(SubVoxelVertex::new([x, y, z], normal, color));
            vertices.push(SubVoxelVertex::new([x, y, z + height], normal, color));
            vertices.push(SubVoxelVertex::new([x + width, y, z + height], normal, color));
            vertices.push(SubVoxelVertex::new([x + width, y, z], normal, color));
        }
        FaceDir::NegY => {
            vertices.push(SubVoxelVertex::new([x, y, z], normal, color));
            vertices.push(SubVoxelVertex::new([x + width, y, z], normal, color));
            vertices.push(SubVoxelVertex::new([x + width, y, z + height], normal, color));
            vertices.push(SubVoxelVertex::new([x, y, z + height], normal, color));
        }
        FaceDir::PosZ => {
            vertices.push(SubVoxelVertex::new([x, y, z], normal, color));
            vertices.push(SubVoxelVertex::new([x + width, y, z], normal, color));
            vertices.push(SubVoxelVertex::new([x + width, y + height, z], normal, color));
            vertices.push(SubVoxelVertex::new([x, y + height, z], normal, color));
        }
        FaceDir::NegZ => {
            vertices.push(SubVoxelVertex::new([x, y, z], normal, color));
            vertices.push(SubVoxelVertex::new([x, y + height, z], normal, color));
            vertices.push(SubVoxelVertex::new([x + width, y + height, z], normal, color));
            vertices.push(SubVoxelVertex::new([x + width, y, z], normal, color));
        }
    }

    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

// ============================================
// Вспомогательные функции
// ============================================

/// Конвертация offset ребёнка в локальные координаты
#[inline]
fn child_offset_to_local(offset: u32) -> (u8, u8, u8) {
    let lx = (offset & 1) as u8;
    let ly = ((offset >> 1) & 1) as u8;
    let lz = ((offset >> 2) & 1) as u8;
    (lx, ly, lz)
}

/// Создание меша (создает новый контекст)
pub fn mesh_chunk_octree_new(
    storage: &ChunkSubVoxelStorage,
    chunk_key: SubVoxelChunkKey,
) -> OctreeMeshData {
    let mut ctx = OctreeMeshContext::new();
    mesh_chunk_octree(storage, chunk_key, &mut ctx)
}
