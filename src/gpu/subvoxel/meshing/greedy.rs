// ============================================
// Greedy Meshing для субвокселей
// ============================================
//
// Объединяет соседние грани одного типа в большие прямоугольники.
// Значительно сокращает количество вершин и draw calls.

use crate::gpu::blocks::BlockType;

/// Направление грани
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

/// Информация о грани для greedy meshing
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceInfo {
    pub block_type: BlockType,
    pub is_top: bool, // Для выбора цвета (top vs side)
}

impl FaceInfo {
    #[inline]
    pub fn new(block_type: BlockType, is_top: bool) -> Self {
        Self { block_type, is_top }
    }
}

/// Результат greedy mesh - объединенный прямоугольник
#[derive(Clone, Copy, Debug)]
pub struct GreedyQuad {
    /// Начальные координаты в 2D слое (u, v)
    pub u: u8,
    pub v: u8,
    /// Размеры
    pub width: u8,
    pub height: u8,
    /// Информация о грани
    pub face: FaceInfo,
}

/// Greedy meshing для 2D слоя
/// mask: 2D массив Option<FaceInfo>, размер width x height
/// Возвращает список объединенных прямоугольников
pub fn greedy_mesh_layer(
    mask: &[Option<FaceInfo>],
    width: usize,
    height: usize,
) -> Vec<GreedyQuad> {
    let mut result = Vec::new();
    let mut visited = vec![false; width * height];

    for v in 0..height {
        for u in 0..width {
            let idx = v * width + u;

            if visited[idx] {
                continue;
            }

            let Some(face) = mask[idx] else {
                continue;
            };

            // Находим максимальную ширину
            let mut w = 1usize;
            while u + w < width {
                let next_idx = v * width + (u + w);
                if visited[next_idx] {
                    break;
                }
                match mask[next_idx] {
                    Some(f) if f == face => w += 1,
                    _ => break,
                }
            }

            // Находим максимальную высоту
            let mut h = 1usize;
            'height_loop: while v + h < height {
                for du in 0..w {
                    let check_idx = (v + h) * width + (u + du);
                    if visited[check_idx] {
                        break 'height_loop;
                    }
                    match mask[check_idx] {
                        Some(f) if f == face => {}
                        _ => break 'height_loop,
                    }
                }
                h += 1;
            }

            // Помечаем как посещенные
            for dv in 0..h {
                for du in 0..w {
                    visited[(v + dv) * width + (u + du)] = true;
                }
            }

            result.push(GreedyQuad {
                u: u as u8,
                v: v as u8,
                width: w as u8,
                height: h as u8,
                face,
            });
        }
    }

    result
}

/// Greedy meshing с переиспользованием буферов (zero-allocation в hot path)
pub fn greedy_mesh_layer_into(
    mask: &[Option<FaceInfo>],
    visited: &mut [bool],
    width: usize,
    height: usize,
    result: &mut Vec<GreedyQuad>,
) {
    result.clear();

    // Очищаем visited
    for v in visited.iter_mut().take(width * height) {
        *v = false;
    }

    for v in 0..height {
        for u in 0..width {
            let idx = v * width + u;

            if visited[idx] {
                continue;
            }

            let Some(face) = mask[idx] else {
                continue;
            };

            // Находим максимальную ширину
            let mut w = 1usize;
            while u + w < width {
                let next_idx = v * width + (u + w);
                if visited[next_idx] {
                    break;
                }
                match mask[next_idx] {
                    Some(f) if f == face => w += 1,
                    _ => break,
                }
            }

            // Находим максимальную высоту
            let mut h = 1usize;
            'height_loop: while v + h < height {
                for du in 0..w {
                    let check_idx = (v + h) * width + (u + du);
                    if visited[check_idx] {
                        break 'height_loop;
                    }
                    match mask[check_idx] {
                        Some(f) if f == face => {}
                        _ => break 'height_loop,
                    }
                }
                h += 1;
            }

            // Помечаем как посещенные
            for dv in 0..h {
                for du in 0..w {
                    visited[(v + dv) * width + (u + du)] = true;
                }
            }

            result.push(GreedyQuad {
                u: u as u8,
                v: v as u8,
                width: w as u8,
                height: h as u8,
                face,
            });
        }
    }
}
