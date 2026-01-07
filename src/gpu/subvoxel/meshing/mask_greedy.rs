// ============================================
// Mask-Based Greedy Meshing - Без сортировки и аллокаций
// ============================================
//
// Классический алгоритм greedy meshing через битовые маски.
// Работает за один проход по каждому слою без промежуточных структур.
//
// Сложность: O(W*H*D) где W,H,D - размеры сетки
// Память: O(W*H) для маски одного слоя

use crate::gpu::blocks::{BlockType, get_face_colors, STONE};
use super::packed_vertex::{PackedVertex, NormalIndex, pack_color};

/// Размер маски (64x64 для субвокселей в чанке)
pub const MASK_SIZE: usize = 64;
pub const MASK_WORDS: usize = MASK_SIZE; // 64 бита = 1 u64 на строку

/// Контекст для mask-based greedy meshing
pub struct MaskGreedyContext {
    /// Битовая маска слоя [row] = u64 битов
    mask: [u64; MASK_SIZE],
    /// Типы блоков для маски
    types: [[u8; MASK_SIZE]; MASK_SIZE],
    /// Выходные буферы
    pub vertices: Vec<PackedVertex>,
    pub indices: Vec<u32>,
}

impl MaskGreedyContext {
    pub fn new() -> Self {
        Self {
            mask: [0; MASK_SIZE],
            types: [[0; MASK_SIZE]; MASK_SIZE],
            vertices: Vec::with_capacity(4096),
            indices: Vec::with_capacity(8192),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    fn clear_mask(&mut self) {
        self.mask = [0; MASK_SIZE];
    }
}

impl Default for MaskGreedyContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Интерфейс для доступа к данным вокселей
pub trait VoxelAccess {
    /// Получить тип блока в точке (None = воздух)
    fn get(&self, x: i32, y: i32, z: i32) -> Option<BlockType>;
    
    /// Границы данных
    fn bounds(&self) -> (i32, i32, i32, i32, i32, i32); // min_x, min_y, min_z, max_x, max_y, max_z
}

/// Greedy meshing через битовые маски
pub fn greedy_mesh_masked<V: VoxelAccess>(
    voxels: &V,
    ctx: &mut MaskGreedyContext,
    chunk_offset: [f32; 3],
) {
    ctx.clear();
    
    let (min_x, min_y, min_z, max_x, max_y, max_z) = voxels.bounds();
    
    // Проходим по каждой оси
    mesh_axis::<V>(voxels, ctx, chunk_offset, Axis::X, min_x, max_x, min_y, max_y, min_z, max_z);
    mesh_axis::<V>(voxels, ctx, chunk_offset, Axis::Y, min_y, max_y, min_x, max_x, min_z, max_z);
    mesh_axis::<V>(voxels, ctx, chunk_offset, Axis::Z, min_z, max_z, min_x, max_x, min_y, max_y);
}

#[derive(Clone, Copy)]
enum Axis { X, Y, Z }

fn mesh_axis<V: VoxelAccess>(
    voxels: &V,
    ctx: &mut MaskGreedyContext,
    chunk_offset: [f32; 3],
    axis: Axis,
    axis_min: i32, axis_max: i32,
    u_min: i32, u_max: i32,
    v_min: i32, v_max: i32,
) {
    let u_size = (u_max - u_min + 1) as usize;
    let v_size = (v_max - v_min + 1) as usize;
    
    if u_size > MASK_SIZE || v_size > MASK_SIZE {
        return; // Слишком большой слой
    }

    // Проходим по слоям вдоль оси
    for d in axis_min..=axis_max + 1 {
        ctx.clear_mask();
        
        // Заполняем маску для положительного направления
        for v in 0..v_size {
            for u in 0..u_size {
                let (x, y, z, nx, ny, nz) = match axis {
                    Axis::X => (d - 1, u_min + u as i32, v_min + v as i32, d, u_min + u as i32, v_min + v as i32),
                    Axis::Y => (u_min + u as i32, d - 1, v_min + v as i32, u_min + u as i32, d, v_min + v as i32),
                    Axis::Z => (u_min + u as i32, v_min + v as i32, d - 1, u_min + u as i32, v_min + v as i32, d),
                };
                
                let current = voxels.get(x, y, z);
                let neighbor = voxels.get(nx, ny, nz);
                
                // Грань видна если текущий solid и сосед пустой
                if let Some(bt) = current {
                    if neighbor.is_none() {
                        ctx.mask[v] |= 1u64 << u;
                        ctx.types[v][u] = bt as u8;
                    }
                }
            }
        }
        
        // Greedy merge для положительного направления
        let normal = match axis {
            Axis::X => NormalIndex::PosX,
            Axis::Y => NormalIndex::PosY,
            Axis::Z => NormalIndex::PosZ,
        };
        greedy_merge_layer(ctx, d as f32, axis, normal, chunk_offset, u_size, v_size, true);
        
        // Заполняем маску для отрицательного направления
        ctx.clear_mask();
        for v in 0..v_size {
            for u in 0..u_size {
                let (x, y, z, nx, ny, nz) = match axis {
                    Axis::X => (d, u_min + u as i32, v_min + v as i32, d - 1, u_min + u as i32, v_min + v as i32),
                    Axis::Y => (u_min + u as i32, d, v_min + v as i32, u_min + u as i32, d - 1, v_min + v as i32),
                    Axis::Z => (u_min + u as i32, v_min + v as i32, d, u_min + u as i32, v_min + v as i32, d - 1),
                };
                
                let current = voxels.get(x, y, z);
                let neighbor = voxels.get(nx, ny, nz);
                
                if let Some(bt) = current {
                    if neighbor.is_none() {
                        ctx.mask[v] |= 1u64 << u;
                        ctx.types[v][u] = bt as u8;
                    }
                }
            }
        }
        
        let normal = match axis {
            Axis::X => NormalIndex::NegX,
            Axis::Y => NormalIndex::NegY,
            Axis::Z => NormalIndex::NegZ,
        };
        greedy_merge_layer(ctx, d as f32, axis, normal, chunk_offset, u_size, v_size, false);
    }
}

/// Greedy merge одного слоя через битовые операции
fn greedy_merge_layer(
    ctx: &mut MaskGreedyContext,
    d: f32,
    axis: Axis,
    normal: NormalIndex,
    chunk_offset: [f32; 3],
    u_size: usize,
    v_size: usize,
    positive: bool,
) {
    for v in 0..v_size {
        let mut u = 0;
        while u < u_size {
            // Пропускаем пустые биты
            if (ctx.mask[v] & (1u64 << u)) == 0 {
                u += 1;
                continue;
            }
            
            let block_type = ctx.types[v][u];
            
            // Находим ширину (расширяем по U)
            let mut width = 1;
            while u + width < u_size {
                let bit = 1u64 << (u + width);
                if (ctx.mask[v] & bit) == 0 || ctx.types[v][u + width] != block_type {
                    break;
                }
                width += 1;
            }
            
            // Находим высоту (расширяем по V)
            let mut height = 1;
            'height: while v + height < v_size {
                for du in 0..width {
                    let bit = 1u64 << (u + du);
                    if (ctx.mask[v + height] & bit) == 0 || ctx.types[v + height][u + du] != block_type {
                        break 'height;
                    }
                }
                height += 1;
            }
            
            // Очищаем использованные биты
            for dv in 0..height {
                for du in 0..width {
                    ctx.mask[v + dv] &= !(1u64 << (u + du));
                }
            }
            
            // Генерируем квад
            emit_quad_packed(
                ctx,
                d, u as f32, v as f32,
                width as f32, height as f32,
                axis, normal, chunk_offset,
                block_type, positive,
            );
            
            u += width;
        }
    }
}

/// Генерация квада с упакованными вершинами
fn emit_quad_packed(
    ctx: &mut MaskGreedyContext,
    d: f32, u: f32, v: f32,
    w: f32, h: f32,
    axis: Axis,
    normal: NormalIndex,
    offset: [f32; 3],
    block_type: u8,
    positive: bool,
) {
    let base = ctx.vertices.len() as u32;
    
    // Получаем цвет из типа блока
    let bt: BlockType = block_type;
    let (top_color, side_color) = get_face_colors(bt);
    
    let color = match normal {
        NormalIndex::PosY => pack_color(top_color[0], top_color[1], top_color[2], 1.0),
        NormalIndex::NegY => pack_color(side_color[0] * 0.5, side_color[1] * 0.5, side_color[2] * 0.5, 1.0),
        _ => pack_color(side_color[0], side_color[1], side_color[2], 1.0),
    };
    
    // Конвертируем координаты в позиции вершин
    let (p0, p1, p2, p3) = match axis {
        Axis::X => {
            let x = (d + offset[0]) * 4.0;
            let y0 = (u + offset[1]) * 4.0;
            let y1 = (u + w + offset[1]) * 4.0;
            let z0 = (v + offset[2]) * 4.0;
            let z1 = (v + h + offset[2]) * 4.0;
            if positive {
                ([x, y0, z0], [x, y0, z1], [x, y1, z1], [x, y1, z0])
            } else {
                ([x, y0, z0], [x, y1, z0], [x, y1, z1], [x, y0, z1])
            }
        }
        Axis::Y => {
            let y = (d + offset[1]) * 4.0;
            let x0 = (u + offset[0]) * 4.0;
            let x1 = (u + w + offset[0]) * 4.0;
            let z0 = (v + offset[2]) * 4.0;
            let z1 = (v + h + offset[2]) * 4.0;
            if positive {
                ([x0, y, z0], [x0, y, z1], [x1, y, z1], [x1, y, z0])
            } else {
                ([x0, y, z0], [x1, y, z0], [x1, y, z1], [x0, y, z1])
            }
        }
        Axis::Z => {
            let z = (d + offset[2]) * 4.0;
            let x0 = (u + offset[0]) * 4.0;
            let x1 = (u + w + offset[0]) * 4.0;
            let y0 = (v + offset[1]) * 4.0;
            let y1 = (v + h + offset[1]) * 4.0;
            if positive {
                ([x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z])
            } else {
                ([x0, y0, z], [x0, y1, z], [x1, y1, z], [x1, y0, z])
            }
        }
    };
    
    // Добавляем вершины
    let to_u8 = |v: f32| v.clamp(0.0, 255.0) as u8;
    
    ctx.vertices.push(PackedVertex {
        pos_x: to_u8(p0[0]), pos_y: to_u8(p0[1]), pos_z: to_u8(p0[2]),
        normal_flags: normal as u8,
        color,
    });
    ctx.vertices.push(PackedVertex {
        pos_x: to_u8(p1[0]), pos_y: to_u8(p1[1]), pos_z: to_u8(p1[2]),
        normal_flags: normal as u8,
        color,
    });
    ctx.vertices.push(PackedVertex {
        pos_x: to_u8(p2[0]), pos_y: to_u8(p2[1]), pos_z: to_u8(p2[2]),
        normal_flags: normal as u8,
        color,
    });
    ctx.vertices.push(PackedVertex {
        pos_x: to_u8(p3[0]), pos_y: to_u8(p3[1]), pos_z: to_u8(p3[2]),
        normal_flags: normal as u8,
        color,
    });
    
    // Индексы
    ctx.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}
