// ============================================
// Greedy Meshing - Оптимизация мешей
// ============================================

use crate::gpu::blocks::BlockType;
use crate::gpu::terrain::mesh::TerrainVertex;

#[derive(Clone, Copy)]
pub enum FaceDir {
    PosX, NegX, PosY, NegY, PosZ, NegZ,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FaceInfo {
    pub block_type: BlockType,
    pub is_top: bool,
}

impl FaceInfo {
    #[inline]
    pub fn new(block_type: BlockType, is_top: bool) -> Self {
        Self { block_type, is_top }
    }
}

/// Zero-allocation greedy meshing для одного слоя
/// 
/// Записывает результаты в предоставленный буфер вместо создания нового Vec.
/// Буфер visited должен быть предварительно очищен (заполнен false).
#[inline]
pub fn greedy_mesh_layer_into(
    mask: &[Option<FaceInfo>],
    visited: &mut [bool],
    size_u: usize,
    size_v: usize,
    results: &mut Vec<(usize, usize, usize, usize, FaceInfo)>,
) {
    results.clear();
    
    for v in 0..size_v {
        for u in 0..size_u {
            let idx = v * size_u + u;
            if visited[idx] { continue; }
            
            let face = match mask[idx] {
                Some(f) => f,
                None => continue,
            };
            
            // Расширяем по U
            let mut width = 1;
            while u + width < size_u {
                let next_idx = v * size_u + (u + width);
                if visited[next_idx] || mask[next_idx] != Some(face) { break; }
                width += 1;
            }

            // Расширяем по V
            let mut height = 1;
            'outer: while v + height < size_v {
                for du in 0..width {
                    let check_idx = (v + height) * size_u + (u + du);
                    if visited[check_idx] || mask[check_idx] != Some(face) { break 'outer; }
                }
                height += 1;
            }
            
            // Помечаем как посещённые
            for dv in 0..height {
                for du in 0..width {
                    visited[(v + dv) * size_u + (u + du)] = true;
                }
            }
            
            results.push((u, v, width, height, face));
        }
    }
}

/// Legacy версия для обратной совместимости (создаёт новые Vec)
#[allow(dead_code)]
pub fn greedy_mesh_layer(
    mask: &[Option<FaceInfo>],
    size_u: usize,
    size_v: usize,
) -> Vec<(usize, usize, usize, usize, FaceInfo)> {
    let mut result = Vec::new();
    let mut visited = vec![false; size_u * size_v];
    greedy_mesh_layer_into(mask, &mut visited, size_u, size_v, &mut result);
    result
}

/// Добавляет объединённую грань в буферы
#[inline]
pub fn add_greedy_face(
    vertices: &mut Vec<TerrainVertex>,
    indices: &mut Vec<u32>,
    x: f32, y: f32, z: f32,
    width_u: f32, height_v: f32,
    normal: [f32; 3],
    color: [f32; 3],
    dir: FaceDir,
) {
    add_greedy_face_with_block(vertices, indices, x, y, z, width_u, height_v, normal, color, dir, 0);
}

/// Добавляет объединённую грань в буферы с block_id
#[inline]
pub fn add_greedy_face_with_block(
    vertices: &mut Vec<TerrainVertex>,
    indices: &mut Vec<u32>,
    x: f32, y: f32, z: f32,
    width_u: f32, height_v: f32,
    normal: [f32; 3],
    color: [f32; 3],
    dir: FaceDir,
    block_id: u8,
) {
    let base = vertices.len() as u32;
    let bid = block_id as u32;
    
    match dir {
        FaceDir::PosX => {
            let x1 = x + 1.0;
            vertices.push(TerrainVertex { position: [x1, y, z + width_u], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x1, y, z], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x1, y + height_v, z], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x1, y + height_v, z + width_u], normal, color, block_id: bid });
        }
        FaceDir::NegX => {
            vertices.push(TerrainVertex { position: [x, y, z], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x, y, z + width_u], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x, y + height_v, z + width_u], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x, y + height_v, z], normal, color, block_id: bid });
        }
        FaceDir::PosY => {
            let y1 = y + 1.0;
            vertices.push(TerrainVertex { position: [x, y1, z], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x, y1, z + height_v], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x + width_u, y1, z + height_v], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x + width_u, y1, z], normal, color, block_id: bid });
        }
        FaceDir::NegY => {
            vertices.push(TerrainVertex { position: [x, y, z + height_v], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x, y, z], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x + width_u, y, z], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x + width_u, y, z + height_v], normal, color, block_id: bid });
        }
        FaceDir::PosZ => {
            let z1 = z + 1.0;
            vertices.push(TerrainVertex { position: [x, y, z1], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x + width_u, y, z1], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x + width_u, y + height_v, z1], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x, y + height_v, z1], normal, color, block_id: bid });
        }
        FaceDir::NegZ => {
            vertices.push(TerrainVertex { position: [x + width_u, y, z], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x, y, z], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x, y + height_v, z], normal, color, block_id: bid });
            vertices.push(TerrainVertex { position: [x + width_u, y + height_v, z], normal, color, block_id: bid });
        }
    }
    
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}
