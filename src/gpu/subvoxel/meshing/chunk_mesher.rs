// ============================================
// Chunk Mesher - Greedy meshing на уровне чанка
// ============================================
//
// Ключевые оптимизации:
// 1. Единая сетка для всего чанка (не по блокам)
// 2. Culling граней между соседними субвокселями
// 3. Greedy meshing на полотнах 64x64 (эффективно!)
// 4. Нет промежуточных структур - работаем напрямую

use crate::gpu::blocks::get_face_colors;
use crate::gpu::subvoxel::chunk::{ChunkSubVoxelStorage, SubVoxelChunkKey};
use super::chunk_grid::{ChunkGrid, CHUNK_GRID_SIZE};
use super::greedy::{FaceInfo, GreedyQuad, greedy_mesh_layer_into};
use super::vertex::SubVoxelVertex;

/// Данные меша чанка
pub struct ChunkMeshData {
    pub vertices: Vec<SubVoxelVertex>,
    pub indices: Vec<u32>,
    pub chunk_key: SubVoxelChunkKey,
    pub version: u64,
}

/// Контекст для chunk meshing (переиспользуемые буферы)
pub struct ChunkMeshContext {
    /// Маска для greedy meshing (64x64 = 4096)
    mask: Vec<Option<FaceInfo>>,
    /// Visited флаги
    visited: Vec<bool>,
    /// Результаты greedy
    quads: Vec<GreedyQuad>,
    /// Выходные буферы
    vertices: Vec<SubVoxelVertex>,
    indices: Vec<u32>,
}

impl ChunkMeshContext {
    pub fn new() -> Self {
        let mask_size = CHUNK_GRID_SIZE * CHUNK_GRID_SIZE;
        Self {
            mask: vec![None; mask_size],
            visited: vec![false; mask_size],
            quads: Vec::with_capacity(256),
            vertices: Vec::with_capacity(4096),
            indices: Vec::with_capacity(8192),
        }
    }

    fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    fn clear_mask(&mut self) {
        for m in self.mask.iter_mut() {
            *m = None;
        }
    }
}

impl Default for ChunkMeshContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Генерация меша для чанка с полным greedy meshing
pub fn mesh_chunk(
    storage: &ChunkSubVoxelStorage,
    chunk_key: SubVoxelChunkKey,
    ctx: &mut ChunkMeshContext,
) -> ChunkMeshData {
    ctx.clear();

    // Строим единую сетку чанка
    let Some(grid) = ChunkGrid::from_chunk_storage(storage) else {
        return ChunkMeshData {
            vertices: Vec::new(),
            indices: Vec::new(),
            chunk_key,
            version: storage.version(),
        };
    };

    let base_x = (chunk_key.x * 16) as f32;
    let base_z = (chunk_key.z * 16) as f32;
    let base_y = grid.min_y as f32 * 0.25; // Конвертируем субвоксельную Y в мировую
    let cell_size = grid.cell_size();

    // Greedy mesh для каждой оси
    mesh_y_faces(&grid, base_x, base_y, base_z, cell_size, ctx);
    mesh_x_faces(&grid, base_x, base_y, base_z, cell_size, ctx);
    mesh_z_faces(&grid, base_x, base_y, base_z, cell_size, ctx);

    ChunkMeshData {
        vertices: std::mem::take(&mut ctx.vertices),
        indices: std::mem::take(&mut ctx.indices),
        chunk_key,
        version: storage.version(),
    }
}

/// Генерация Y граней (+Y и -Y) с culling и greedy
fn mesh_y_faces(
    grid: &ChunkGrid,
    base_x: f32, base_y: f32, base_z: f32,
    cell_size: f32,
    ctx: &mut ChunkMeshContext,
) {
    let width = grid.width();
    let height = grid.height();
    let depth = grid.depth();

    // Проходим по каждому слою Y
    for y in 0..=height {
        ctx.clear_mask();

        // Заполняем маску для +Y граней
        for z in 0..depth {
            for x in 0..width {
                let idx = z * width + x;

                // +Y грань: блок снизу есть, сверху нет (или край)
                if y > 0 {
                    if let Some(block_type) = grid.get(x, y - 1, z) {
                        let above_empty = y >= height || grid.get(x, y, z).is_none();
                        if above_empty {
                            ctx.mask[idx] = Some(FaceInfo::new(block_type, true));
                        }
                    }
                }
            }
        }

        // Greedy mesh +Y
        greedy_mesh_layer_into(
            &ctx.mask[..width * depth],
            &mut ctx.visited[..width * depth],
            width, depth,
            &mut ctx.quads,
        );

        let world_y = base_y + y as f32 * cell_size;
        for quad in &ctx.quads {
            add_y_quad(
                &mut ctx.vertices, &mut ctx.indices,
                base_x + quad.u as f32 * cell_size,
                world_y,
                base_z + quad.v as f32 * cell_size,
                quad.width as f32 * cell_size,
                quad.height as f32 * cell_size,
                quad.face,
                true, // +Y
            );
        }

        // Заполняем маску для -Y граней
        ctx.clear_mask();
        for z in 0..depth {
            for x in 0..width {
                let idx = z * width + x;

                // -Y грань: блок есть, снизу нет (или край)
                if y < height {
                    if let Some(block_type) = grid.get(x, y, z) {
                        let below_empty = y == 0 || grid.get(x, y - 1, z).is_none();
                        if below_empty {
                            ctx.mask[idx] = Some(FaceInfo::new(block_type, false));
                        }
                    }
                }
            }
        }

        // Greedy mesh -Y
        greedy_mesh_layer_into(
            &ctx.mask[..width * depth],
            &mut ctx.visited[..width * depth],
            width, depth,
            &mut ctx.quads,
        );

        for quad in &ctx.quads {
            add_y_quad(
                &mut ctx.vertices, &mut ctx.indices,
                base_x + quad.u as f32 * cell_size,
                world_y,
                base_z + quad.v as f32 * cell_size,
                quad.width as f32 * cell_size,
                quad.height as f32 * cell_size,
                quad.face,
                false, // -Y
            );
        }
    }
}

/// Генерация X граней (+X и -X) с culling и greedy
fn mesh_x_faces(
    grid: &ChunkGrid,
    base_x: f32, base_y: f32, base_z: f32,
    cell_size: f32,
    ctx: &mut ChunkMeshContext,
) {
    let width = grid.width();
    let height = grid.height();
    let depth = grid.depth();

    for x in 0..=width {
        ctx.clear_mask();

        // +X грань: блок слева есть, справа нет
        for y in 0..height {
            for z in 0..depth {
                let idx = y * depth + z;

                if x > 0 {
                    if let Some(block_type) = grid.get(x - 1, y, z) {
                        let right_empty = x >= width || grid.get(x, y, z).is_none();
                        if right_empty {
                            ctx.mask[idx] = Some(FaceInfo::new(block_type, false));
                        }
                    }
                }
            }
        }

        greedy_mesh_layer_into(
            &ctx.mask[..height * depth],
            &mut ctx.visited[..height * depth],
            depth, height,
            &mut ctx.quads,
        );

        let world_x = base_x + x as f32 * cell_size;
        for quad in &ctx.quads {
            add_x_quad(
                &mut ctx.vertices, &mut ctx.indices,
                world_x,
                base_y + quad.v as f32 * cell_size,
                base_z + quad.u as f32 * cell_size,
                quad.width as f32 * cell_size,
                quad.height as f32 * cell_size,
                quad.face,
                true, // +X
            );
        }

        // -X грань
        ctx.clear_mask();
        for y in 0..height {
            for z in 0..depth {
                let idx = y * depth + z;

                if x < width {
                    if let Some(block_type) = grid.get(x, y, z) {
                        let left_empty = x == 0 || grid.get(x - 1, y, z).is_none();
                        if left_empty {
                            ctx.mask[idx] = Some(FaceInfo::new(block_type, false));
                        }
                    }
                }
            }
        }

        greedy_mesh_layer_into(
            &ctx.mask[..height * depth],
            &mut ctx.visited[..height * depth],
            depth, height,
            &mut ctx.quads,
        );

        for quad in &ctx.quads {
            add_x_quad(
                &mut ctx.vertices, &mut ctx.indices,
                world_x,
                base_y + quad.v as f32 * cell_size,
                base_z + quad.u as f32 * cell_size,
                quad.width as f32 * cell_size,
                quad.height as f32 * cell_size,
                quad.face,
                false, // -X
            );
        }
    }
}

/// Генерация Z граней (+Z и -Z) с culling и greedy
fn mesh_z_faces(
    grid: &ChunkGrid,
    base_x: f32, base_y: f32, base_z: f32,
    cell_size: f32,
    ctx: &mut ChunkMeshContext,
) {
    let width = grid.width();
    let height = grid.height();
    let depth = grid.depth();

    for z in 0..=depth {
        ctx.clear_mask();

        // +Z грань: блок сзади есть, спереди нет
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;

                if z > 0 {
                    if let Some(block_type) = grid.get(x, y, z - 1) {
                        let front_empty = z >= depth || grid.get(x, y, z).is_none();
                        if front_empty {
                            ctx.mask[idx] = Some(FaceInfo::new(block_type, false));
                        }
                    }
                }
            }
        }

        greedy_mesh_layer_into(
            &ctx.mask[..height * width],
            &mut ctx.visited[..height * width],
            width, height,
            &mut ctx.quads,
        );

        let world_z = base_z + z as f32 * cell_size;
        for quad in &ctx.quads {
            add_z_quad(
                &mut ctx.vertices, &mut ctx.indices,
                base_x + quad.u as f32 * cell_size,
                base_y + quad.v as f32 * cell_size,
                world_z,
                quad.width as f32 * cell_size,
                quad.height as f32 * cell_size,
                quad.face,
                true, // +Z
            );
        }

        // -Z грань
        ctx.clear_mask();
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;

                if z < depth {
                    if let Some(block_type) = grid.get(x, y, z) {
                        let back_empty = z == 0 || grid.get(x, y, z - 1).is_none();
                        if back_empty {
                            ctx.mask[idx] = Some(FaceInfo::new(block_type, false));
                        }
                    }
                }
            }
        }

        greedy_mesh_layer_into(
            &ctx.mask[..height * width],
            &mut ctx.visited[..height * width],
            width, height,
            &mut ctx.quads,
        );

        for quad in &ctx.quads {
            add_z_quad(
                &mut ctx.vertices, &mut ctx.indices,
                base_x + quad.u as f32 * cell_size,
                base_y + quad.v as f32 * cell_size,
                world_z,
                quad.width as f32 * cell_size,
                quad.height as f32 * cell_size,
                quad.face,
                false, // -Z
            );
        }
    }
}

// ============================================
// Функции добавления квадов
// ============================================

#[inline]
fn add_y_quad(
    vertices: &mut Vec<SubVoxelVertex>,
    indices: &mut Vec<u32>,
    x: f32, y: f32, z: f32,
    w: f32, h: f32,
    face: FaceInfo,
    positive: bool,
) {
    let base = vertices.len() as u32;
    let (top_color, side_color) = get_face_colors(face.block_type);
    let color = if face.is_top { top_color } else { 
        [side_color[0] * 0.5, side_color[1] * 0.5, side_color[2] * 0.5] 
    };

    if positive {
        let normal = [0.0, 1.0, 0.0];
        vertices.push(SubVoxelVertex::new([x, y, z], normal, top_color));
        vertices.push(SubVoxelVertex::new([x, y, z + h], normal, top_color));
        vertices.push(SubVoxelVertex::new([x + w, y, z + h], normal, top_color));
        vertices.push(SubVoxelVertex::new([x + w, y, z], normal, top_color));
    } else {
        let normal = [0.0, -1.0, 0.0];
        vertices.push(SubVoxelVertex::new([x, y, z], normal, color));
        vertices.push(SubVoxelVertex::new([x + w, y, z], normal, color));
        vertices.push(SubVoxelVertex::new([x + w, y, z + h], normal, color));
        vertices.push(SubVoxelVertex::new([x, y, z + h], normal, color));
    }

    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[inline]
fn add_x_quad(
    vertices: &mut Vec<SubVoxelVertex>,
    indices: &mut Vec<u32>,
    x: f32, y: f32, z: f32,
    w: f32, h: f32,
    face: FaceInfo,
    positive: bool,
) {
    let base = vertices.len() as u32;
    let (_, side_color) = get_face_colors(face.block_type);

    if positive {
        let normal = [1.0, 0.0, 0.0];
        vertices.push(SubVoxelVertex::new([x, y, z], normal, side_color));
        vertices.push(SubVoxelVertex::new([x, y, z + w], normal, side_color));
        vertices.push(SubVoxelVertex::new([x, y + h, z + w], normal, side_color));
        vertices.push(SubVoxelVertex::new([x, y + h, z], normal, side_color));
    } else {
        let normal = [-1.0, 0.0, 0.0];
        vertices.push(SubVoxelVertex::new([x, y, z], normal, side_color));
        vertices.push(SubVoxelVertex::new([x, y + h, z], normal, side_color));
        vertices.push(SubVoxelVertex::new([x, y + h, z + w], normal, side_color));
        vertices.push(SubVoxelVertex::new([x, y, z + w], normal, side_color));
    }

    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[inline]
fn add_z_quad(
    vertices: &mut Vec<SubVoxelVertex>,
    indices: &mut Vec<u32>,
    x: f32, y: f32, z: f32,
    w: f32, h: f32,
    face: FaceInfo,
    positive: bool,
) {
    let base = vertices.len() as u32;
    let (_, side_color) = get_face_colors(face.block_type);

    if positive {
        let normal = [0.0, 0.0, 1.0];
        vertices.push(SubVoxelVertex::new([x, y, z], normal, side_color));
        vertices.push(SubVoxelVertex::new([x + w, y, z], normal, side_color));
        vertices.push(SubVoxelVertex::new([x + w, y + h, z], normal, side_color));
        vertices.push(SubVoxelVertex::new([x, y + h, z], normal, side_color));
    } else {
        let normal = [0.0, 0.0, -1.0];
        vertices.push(SubVoxelVertex::new([x, y, z], normal, side_color));
        vertices.push(SubVoxelVertex::new([x, y + h, z], normal, side_color));
        vertices.push(SubVoxelVertex::new([x + w, y + h, z], normal, side_color));
        vertices.push(SubVoxelVertex::new([x + w, y, z], normal, side_color));
    }

    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// Генерация меша (создает новый контекст)
pub fn mesh_chunk_new(
    storage: &ChunkSubVoxelStorage,
    chunk_key: SubVoxelChunkKey,
) -> ChunkMeshData {
    let mut ctx = ChunkMeshContext::new();
    mesh_chunk(storage, chunk_key, &mut ctx)
}
