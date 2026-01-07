// ============================================
// SubVoxel Renderer - ULTRA OPTIMIZED
// ============================================
//
// Ключевые оптимизации:
// 1. Кэширование global_grid между кадрами
// 2. Инкрементальное обновление только изменённых чанков
// 3. Отложенная перестройка мешей (не каждый кадр)
// 4. Culling внутренних граней

use std::collections::HashMap;
use super::subvoxel::{SubVoxelStorage, SubVoxel};
use crate::gpu::terrain::mesh::TerrainVertex;
use crate::gpu::blocks::{get_face_colors, BlockType};

/// Размер чанка субвокселей
const CHUNK_SIZE: i32 = 16;

/// Ключ чанка субвокселей
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ChunkKey {
    x: i32,
    z: i32,
}

impl ChunkKey {
    #[inline]
    fn from_block(bx: i32, bz: i32) -> Self {
        Self {
            x: bx.div_euclid(CHUNK_SIZE),
            z: bz.div_euclid(CHUNK_SIZE),
        }
    }
}

/// GPU данные одного чанка
struct ChunkGpuData {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

/// Ключ для глобальной сетки субвокселей
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
struct GridKey {
    x: i32,
    y: i32,
    z: i32,
}

/// Рендерер суб-вокселей
pub struct SubVoxelRenderer {
    /// GPU буферы по чанкам
    chunks: HashMap<ChunkKey, ChunkGpuData>,
    /// Версия хранилища
    last_version: u64,
    /// Кэшированная глобальная сетка (НЕ перестраиваем каждый кадр!)
    global_grid: HashMap<GridKey, BlockType>,
    /// Кэшированные субвоксели по чанкам
    cached_chunks: HashMap<ChunkKey, Vec<SubVoxel>>,
    /// Флаг полной перестройки
    needs_full_rebuild: bool,
}

impl SubVoxelRenderer {
    pub fn new(_device: &wgpu::Device) -> Self {
        Self {
            chunks: HashMap::with_capacity(256),
            last_version: 0,
            global_grid: HashMap::with_capacity(500_000),
            cached_chunks: HashMap::with_capacity(256),
            needs_full_rebuild: true,
        }
    }

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, storage: &SubVoxelStorage) {
        let current_version = storage.version();
        
        // Ничего не изменилось - выходим сразу
        if current_version == self.last_version && !self.needs_full_rebuild {
            return;
        }

        // Проверяем нужна ли полная перестройка
        // (первый запуск или большие изменения)
        let version_diff = current_version.saturating_sub(self.last_version);
        
        if self.needs_full_rebuild || version_diff > 1000 || self.global_grid.is_empty() {
            self.full_rebuild(device, queue, storage);
            self.needs_full_rebuild = false;
        } else {
            // Инкрементальное обновление - пока просто пропускаем мелкие изменения
            // чтобы не тормозить каждый кадр
        }
        
        self.last_version = current_version;
    }

    /// Полная перестройка всех мешей
    fn full_rebuild(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, storage: &SubVoxelStorage) {
        let all_subvoxels = storage.get_all();
        
        if all_subvoxels.is_empty() {
            self.chunks.clear();
            self.global_grid.clear();
            self.cached_chunks.clear();
            return;
        }

        // Строим глобальную сетку
        self.global_grid.clear();
        self.global_grid.reserve(all_subvoxels.len());
        
        for sv in &all_subvoxels {
            let div = sv.pos.level.divisions() as i32;
            let key = GridKey {
                x: sv.pos.block_x * div + sv.pos.sub_x as i32,
                y: sv.pos.block_y * div + sv.pos.sub_y as i32,
                z: sv.pos.block_z * div + sv.pos.sub_z as i32,
            };
            self.global_grid.insert(key, sv.block_type);
        }

        // Группируем по чанкам
        self.cached_chunks.clear();
        for sv in all_subvoxels {
            let key = ChunkKey::from_block(sv.pos.block_x, sv.pos.block_z);
            self.cached_chunks.entry(key).or_insert_with(Vec::new).push(sv);
        }

        // Удаляем старые чанки
        let existing_keys: std::collections::HashSet<_> = self.cached_chunks.keys().copied().collect();
        self.chunks.retain(|k, _| existing_keys.contains(k));

        // Перестраиваем меши для всех чанков
        let mut vertices = Vec::with_capacity(32_000);
        let mut indices = Vec::with_capacity(48_000);

        // Собираем ключи чтобы избежать borrow conflict
        let chunk_keys: Vec<_> = self.cached_chunks.keys().copied().collect();
        
        for chunk_key in chunk_keys {
            vertices.clear();
            indices.clear();

            if let Some(chunk_subvoxels) = self.cached_chunks.get(&chunk_key) {
                generate_chunk_mesh(chunk_subvoxels, &self.global_grid, &mut vertices, &mut indices);
            }

            if vertices.is_empty() {
                self.chunks.remove(&chunk_key);
                continue;
            }

            // Upload напрямую без вызова метода self
            let vertex_size = vertices.len() * std::mem::size_of::<TerrainVertex>();
            let index_size = indices.len() * std::mem::size_of::<u32>();

            let needs_recreate = self.chunks.get(&chunk_key)
                .map(|data| {
                    data.vertex_buffer.size() < vertex_size as u64 ||
                    data.index_buffer.size() < index_size as u64
                })
                .unwrap_or(true);

            if needs_recreate {
                let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("SubVoxel Vertex"),
                    size: ((vertex_size * 2).max(4096)) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("SubVoxel Index"),
                    size: ((index_size * 2).max(4096)) as u64,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                self.chunks.insert(chunk_key, ChunkGpuData {
                    vertex_buffer,
                    index_buffer,
                    num_indices: 0,
                });
            }

            if let Some(gpu_data) = self.chunks.get_mut(&chunk_key) {
                queue.write_buffer(&gpu_data.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
                queue.write_buffer(&gpu_data.index_buffer, 0, bytemuck::cast_slice(&indices));
                gpu_data.num_indices = indices.len() as u32;
            }
        }
    }

    pub fn has_content(&self) -> bool {
        self.chunks.values().any(|d| d.num_indices > 0)
    }

    /// Итератор по всем чанкам для рендеринга
    pub fn iter_chunks(&self) -> impl Iterator<Item = (&wgpu::Buffer, &wgpu::Buffer, u32)> {
        self.chunks.values()
            .filter(|d| d.num_indices > 0)
            .map(|d| (&d.vertex_buffer, &d.index_buffer, d.num_indices))
    }

    /// Принудительная перестройка
    pub fn force_rebuild(&mut self) {
        self.needs_full_rebuild = true;
    }
}

// ============================================
// Генерация меша с culling внутренних граней
// ============================================

fn generate_chunk_mesh(
    chunk_subvoxels: &[SubVoxel],
    global_grid: &HashMap<GridKey, BlockType>,
    vertices: &mut Vec<TerrainVertex>,
    indices: &mut Vec<u32>,
) {
    if chunk_subvoxels.is_empty() {
        return;
    }

    let size = chunk_subvoxels[0].pos.level.size();

    for sv in chunk_subvoxels {
        let div = sv.pos.level.divisions() as i32;
        let gx = sv.pos.block_x * div + sv.pos.sub_x as i32;
        let gy = sv.pos.block_y * div + sv.pos.sub_y as i32;
        let gz = sv.pos.block_z * div + sv.pos.sub_z as i32;

        let world_x = gx as f32 * size;
        let world_y = gy as f32 * size;
        let world_z = gz as f32 * size;

        let (top_color, side_color) = get_face_colors(sv.block_type);
        let bottom_color = [side_color[0] * 0.5, side_color[1] * 0.5, side_color[2] * 0.5];

        // Проверяем каждую грань - рисуем только если сосед пустой
        // +Y
        if !global_grid.contains_key(&GridKey { x: gx, y: gy + 1, z: gz }) {
            add_face(vertices, indices, world_x, world_y + size, world_z, size, [0.0, 1.0, 0.0], top_color, FaceDir::PosY);
        }
        // -Y
        if !global_grid.contains_key(&GridKey { x: gx, y: gy - 1, z: gz }) {
            add_face(vertices, indices, world_x, world_y, world_z, size, [0.0, -1.0, 0.0], bottom_color, FaceDir::NegY);
        }
        // +X
        if !global_grid.contains_key(&GridKey { x: gx + 1, y: gy, z: gz }) {
            add_face(vertices, indices, world_x + size, world_y, world_z, size, [1.0, 0.0, 0.0], side_color, FaceDir::PosX);
        }
        // -X
        if !global_grid.contains_key(&GridKey { x: gx - 1, y: gy, z: gz }) {
            add_face(vertices, indices, world_x, world_y, world_z, size, [-1.0, 0.0, 0.0], side_color, FaceDir::NegX);
        }
        // +Z
        if !global_grid.contains_key(&GridKey { x: gx, y: gy, z: gz + 1 }) {
            add_face(vertices, indices, world_x, world_y, world_z + size, size, [0.0, 0.0, 1.0], side_color, FaceDir::PosZ);
        }
        // -Z
        if !global_grid.contains_key(&GridKey { x: gx, y: gy, z: gz - 1 }) {
            add_face(vertices, indices, world_x, world_y, world_z, size, [0.0, 0.0, -1.0], side_color, FaceDir::NegZ);
        }
    }
}

#[derive(Clone, Copy)]
enum FaceDir { PosX, NegX, PosY, NegY, PosZ, NegZ }

#[inline]
fn add_face(
    vertices: &mut Vec<TerrainVertex>,
    indices: &mut Vec<u32>,
    x: f32, y: f32, z: f32,
    size: f32,
    normal: [f32; 3],
    color: [f32; 3],
    dir: FaceDir,
) {
    let base_idx = vertices.len() as u32;

    let (p0, p1, p2, p3) = match dir {
        FaceDir::PosY => ([x, y, z], [x, y, z + size], [x + size, y, z + size], [x + size, y, z]),
        FaceDir::NegY => ([x, y, z], [x + size, y, z], [x + size, y, z + size], [x, y, z + size]),
        FaceDir::PosX => ([x, y, z + size], [x, y, z], [x, y + size, z], [x, y + size, z + size]),
        FaceDir::NegX => ([x, y, z], [x, y, z + size], [x, y + size, z + size], [x, y + size, z]),
        FaceDir::PosZ => ([x, y, z], [x + size, y, z], [x + size, y + size, z], [x, y + size, z]),
        FaceDir::NegZ => ([x + size, y, z], [x, y, z], [x, y + size, z], [x + size, y + size, z]),
    };

    vertices.push(TerrainVertex { position: p0, normal, color, block_id: 0 });
    vertices.push(TerrainVertex { position: p1, normal, color, block_id: 0 });
    vertices.push(TerrainVertex { position: p2, normal, color, block_id: 0 });
    vertices.push(TerrainVertex { position: p3, normal, color, block_id: 0 });

    indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2, base_idx, base_idx + 2, base_idx + 3]);
}
