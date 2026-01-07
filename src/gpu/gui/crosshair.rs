// ============================================
// Crosshair & Block Highlight - UI элементы
// ============================================
// Прицел в центре экрана и выделение блока

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// Вершина для UI (2D позиция + цвет)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct UiVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

impl UiVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<UiVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Прицел (crosshair)
pub struct Crosshair {
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    pipeline: wgpu::RenderPipeline,
}

impl Crosshair {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        // Создаём вершины прицела (крест в центре экрана)
        let size = 0.02; // Размер в NDC
        let thickness = 0.003;
        let color = [1.0, 1.0, 1.0, 0.8]; // Белый полупрозрачный
        
        let vertices = vec![
            // Горизонтальная линия
            UiVertex { position: [-size, -thickness], color },
            UiVertex { position: [size, -thickness], color },
            UiVertex { position: [size, thickness], color },
            UiVertex { position: [-size, -thickness], color },
            UiVertex { position: [size, thickness], color },
            UiVertex { position: [-size, thickness], color },
            
            // Вертикальная линия
            UiVertex { position: [-thickness, -size], color },
            UiVertex { position: [thickness, -size], color },
            UiVertex { position: [thickness, size], color },
            UiVertex { position: [-thickness, -size], color },
            UiVertex { position: [thickness, size], color },
            UiVertex { position: [-thickness, size], color },
        ];
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Crosshair Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        // Шейдер для UI
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ui.wgsl").into()),
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("UI Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Crosshair Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[UiVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None, // UI рисуется поверх всего
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        
        Self {
            vertex_buffer,
            vertex_count: vertices.len() as u32,
            pipeline,
        }
    }
    
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.vertex_count, 0..1);
    }
}

/// Вершина для 3D wireframe
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct WireVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl WireVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WireVertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Выделение блока (wireframe куб)
pub struct BlockHighlight {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    pipeline: wgpu::RenderPipeline,
    
    // Uniform для позиции блока и view-proj матрицы
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct HighlightUniforms {
    view_proj: [[f32; 4]; 4],
    block_pos: [f32; 3],
    block_size: f32,
}

impl BlockHighlight {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        // Вершины единичного куба (будет масштабироваться в шейдере)
        let color = [0.0, 0.0, 0.0, 0.6]; // Чёрный полупрозрачный
        
        let vertices = vec![
            // 8 вершин куба (0 to 1)
            WireVertex { position: [0.0, 0.0, 0.0], color },
            WireVertex { position: [1.0, 0.0, 0.0], color },
            WireVertex { position: [1.0, 1.0, 0.0], color },
            WireVertex { position: [0.0, 1.0, 0.0], color },
            WireVertex { position: [0.0, 0.0, 1.0], color },
            WireVertex { position: [1.0, 0.0, 1.0], color },
            WireVertex { position: [1.0, 1.0, 1.0], color },
            WireVertex { position: [0.0, 1.0, 1.0], color },
        ];
        
        // Индексы для линий (12 рёбер куба)
        let indices: Vec<u32> = vec![
            // Нижняя грань
            0, 1, 1, 2, 2, 3, 3, 0,
            // Верхняя грань
            4, 5, 5, 6, 6, 7, 7, 4,
            // Вертикальные рёбра
            0, 4, 1, 5, 2, 6, 3, 7,
        ];
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Block Highlight Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Block Highlight Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        // Uniform buffer
        let uniforms = HighlightUniforms {
            view_proj: ultraviolet::Mat4::identity().into(),
            block_pos: [0.0, 0.0, 0.0],
            block_size: 1.0,
        };
        
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Block Highlight Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Block Highlight Bind Group Layout"),
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
        });
        
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Block Highlight Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        
        // Шейдер
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Block Highlight Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/highlight.wgsl").into()),
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Block Highlight Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Block Highlight Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[WireVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::GreaterEqual, // Reversed-Z
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        
        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            pipeline,
            uniform_buffer,
            uniform_bind_group,
        }
    }
    
    /// Обновить позицию выделяемого блока
    pub fn update(&self, queue: &wgpu::Queue, view_proj: [[f32; 4]; 4], block_pos: [i32; 3]) {
        self.update_with_size(queue, view_proj, [block_pos[0] as f32, block_pos[1] as f32, block_pos[2] as f32], 1.0);
    }
    
    /// Обновить позицию и размер выделяемого блока (для суб-вокселей)
    pub fn update_with_size(&self, queue: &wgpu::Queue, view_proj: [[f32; 4]; 4], block_pos: [f32; 3], size: f32) {
        let uniforms = HighlightUniforms {
            view_proj,
            block_pos,
            block_size: size,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }
    
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}
