// ============================================
// Player Model - Меш и рендеринг модели игрока
// ============================================
// Простая модель игрока (куб/капсула) для режима 3-го лица

use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use ultraviolet::{Mat4, Vec3};

use super::player::{Player, PLAYER_HEIGHT, PLAYER_RADIUS};

/// Вершина модели игрока (такая же как TerrainVertex для совместимости)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PlayerVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

impl PlayerVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PlayerVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

/// Генератор меша игрока
pub struct PlayerMeshGenerator;

impl PlayerMeshGenerator {
    /// Создать меш куба (простейшая модель)
    pub fn create_cube_mesh() -> (Vec<PlayerVertex>, Vec<u32>) {
        let half_w = PLAYER_RADIUS; // Радиус = половина ширины
        let height = PLAYER_HEIGHT;
        
        // Цвета частей тела
        let body_color = [0.2, 0.4, 0.8];   // Синий (тело)
        let head_color = [0.9, 0.75, 0.6];  // Телесный (голова)
        let leg_color = [0.3, 0.3, 0.5];    // Тёмно-синий (ноги)
        
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        // === Тело (центральный куб) ===
        let body_bottom = 0.4;
        let body_top = 1.4;
        Self::add_box(
            &mut vertices, &mut indices,
            -half_w, body_bottom, -half_w * 0.6,
            half_w, body_top, half_w * 0.6,
            body_color,
        );
        
        // === Голова ===
        let head_size = 0.35;
        let head_bottom = body_top;
        let head_top = head_bottom + head_size * 2.0;
        Self::add_box(
            &mut vertices, &mut indices,
            -head_size, head_bottom, -head_size,
            head_size, head_top, head_size,
            head_color,
        );
        
        // === Ноги ===
        let leg_width = half_w * 0.4;
        let leg_gap = 0.02;
        
        // Левая нога
        Self::add_box(
            &mut vertices, &mut indices,
            -half_w, 0.0, -leg_width,
            -leg_gap, body_bottom, leg_width,
            leg_color,
        );
        
        // Правая нога
        Self::add_box(
            &mut vertices, &mut indices,
            leg_gap, 0.0, -leg_width,
            half_w, body_bottom, leg_width,
            leg_color,
        );
        
        // === Руки ===
        let arm_width = 0.12;
        let arm_length = 0.6;
        let arm_top = body_top - 0.1;
        let arm_bottom = arm_top - arm_length;
        
        // Левая рука
        Self::add_box(
            &mut vertices, &mut indices,
            -half_w - arm_width, arm_bottom, -arm_width,
            -half_w, arm_top, arm_width,
            body_color,
        );
        
        // Правая рука
        Self::add_box(
            &mut vertices, &mut indices,
            half_w, arm_bottom, -arm_width,
            half_w + arm_width, arm_top, arm_width,
            body_color,
        );
        
        (vertices, indices)
    }
    
    /// Добавить куб (box) в меш
    fn add_box(
        vertices: &mut Vec<PlayerVertex>,
        indices: &mut Vec<u32>,
        x0: f32, y0: f32, z0: f32,
        x1: f32, y1: f32, z1: f32,
        color: [f32; 3],
    ) {
        let base_idx = vertices.len() as u32;
        
        // 8 вершин куба
        let corners = [
            [x0, y0, z0], // 0: left-bottom-back
            [x1, y0, z0], // 1: right-bottom-back
            [x1, y1, z0], // 2: right-top-back
            [x0, y1, z0], // 3: left-top-back
            [x0, y0, z1], // 4: left-bottom-front
            [x1, y0, z1], // 5: right-bottom-front
            [x1, y1, z1], // 6: right-top-front
            [x0, y1, z1], // 7: left-top-front
        ];
        
        // 6 граней с нормалями
        let faces = [
            // Back face (Z-)
            ([0, 1, 2, 3], [0.0, 0.0, -1.0]),
            // Front face (Z+)
            ([5, 4, 7, 6], [0.0, 0.0, 1.0]),
            // Left face (X-)
            ([4, 0, 3, 7], [-1.0, 0.0, 0.0]),
            // Right face (X+)
            ([1, 5, 6, 2], [1.0, 0.0, 0.0]),
            // Bottom face (Y-)
            ([4, 5, 1, 0], [0.0, -1.0, 0.0]),
            // Top face (Y+)
            ([3, 2, 6, 7], [0.0, 1.0, 0.0]),
        ];
        
        for (face_indices, normal) in faces {
            let face_base = vertices.len() as u32;
            
            for &corner_idx in &face_indices {
                vertices.push(PlayerVertex {
                    position: corners[corner_idx],
                    normal,
                    color,
                });
            }
            
            // Два треугольника на грань
            indices.push(face_base);
            indices.push(face_base + 1);
            indices.push(face_base + 2);
            
            indices.push(face_base);
            indices.push(face_base + 2);
            indices.push(face_base + 3);
        }
    }
}

/// GPU буферы модели игрока
pub struct PlayerModel {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    
    // Uniform буфер для матрицы модели
    model_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,
}

impl PlayerModel {
    pub fn new(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let (vertices, indices) = PlayerMeshGenerator::create_cube_mesh();
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Player Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Player Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        // Матрица модели (identity изначально)
        let model_matrix: [[f32; 4]; 4] = Mat4::identity().into();
        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Player Model Buffer"),
            contents: bytemuck::cast_slice(&model_matrix),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Player Model Bind Group"),
            layout: bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
        });
        
        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            model_buffer,
            model_bind_group,
        }
    }
    
    /// Обновить матрицу модели на основе позиции игрока
    pub fn update(&self, queue: &wgpu::Queue, player: &Player) {
        // Матрица трансформации: перемещение + поворот по yaw
        let translation = Mat4::from_translation(player.position);
        let rotation = Mat4::from_rotation_y(player.yaw);
        let model_matrix = translation * rotation;
        
        let matrix_data: [[f32; 4]; 4] = model_matrix.into();
        queue.write_buffer(&self.model_buffer, 0, bytemuck::cast_slice(&matrix_data));
    }
    
    /// Рендеринг модели
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_bind_group(1, &self.model_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
    
    /// Создать bind group layout для матрицы модели
    pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Player Model Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }
}
