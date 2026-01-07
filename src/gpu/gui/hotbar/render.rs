// ============================================
// Hotbar GPU Renderer - Hi-Tech glassmorphism style
// ============================================

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use std::time::Instant;

use super::{Hotbar, HotbarItem, HOTBAR_SLOTS, SLOT_SIZE, SLOT_GAP, BOTTOM_PADDING};

/// Uniforms для шейдера хотбара
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct HotbarUniforms {
    pub screen_size: [f32; 2],
    pub time: f32,
    pub selected_slot: f32,
}

/// Данные одного слота для GPU
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct HotbarSlot {
    pub pos: [f32; 2],         // Позиция слота
    pub size: [f32; 2],        // Размер слота
    pub slot_index: u32,       // Индекс слота (0-8)
    pub is_selected: u32,      // 1 если выбран, 0 иначе
    pub has_item: u32,         // 1 если есть предмет
    pub _padding: u32,
    pub top_color: [f32; 4],   // Цвет верхней грани (RGBA)
    pub side_color: [f32; 4],  // Цвет боковых граней (RGBA)
}

/// GPU рендерер хотбара
pub struct HotbarRenderer {
    // GPU ресурсы
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    
    // Состояние
    screen_width: f32,
    screen_height: f32,
    start_time: Instant,
}

impl HotbarRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Hotbar Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        
        // Uniform buffer
        let uniforms = HotbarUniforms {
            screen_size: [width as f32, height as f32],
            time: 0.0,
            selected_slot: 0.0,
        };
        
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Hotbar Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Hotbar Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        
        // Vertex buffer (квадрат из 2 треугольников)
        let vertices: Vec<[f32; 2]> = vec![
            [0.0, 0.0], [1.0, 0.0], [1.0, 1.0],
            [0.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        ];
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Hotbar Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        // Instance buffer (для всех слотов + фон)
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hotbar Instance Buffer"),
            size: (std::mem::size_of::<HotbarSlot>() * (HOTBAR_SLOTS + 1)) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Hotbar Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("hotbar.wgsl").into()),
        });
        
        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Hotbar Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Hotbar Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    // Vertex buffer
                    wgpu::VertexBufferLayout {
                        array_stride: 8,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        }],
                    },
                    // Instance buffer
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<HotbarSlot>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 1, // pos
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 8,
                                shader_location: 2, // size
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Uint32,
                                offset: 16,
                                shader_location: 3, // slot_index
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Uint32,
                                offset: 20,
                                shader_location: 4, // is_selected
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Uint32,
                                offset: 24,
                                shader_location: 5, // has_item
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 32,
                                shader_location: 6, // top_color
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 48,
                                shader_location: 7, // side_color
                            },
                        ],
                    },
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        
        Self {
            pipeline,
            vertex_buffer,
            instance_buffer,
            uniform_buffer,
            bind_group,
            screen_width: width as f32,
            screen_height: height as f32,
            start_time: Instant::now(),
        }
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.screen_width = width as f32;
        self.screen_height = height as f32;
    }
    
    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        hotbar: &Hotbar,
    ) {
        if !hotbar.is_visible() {
            return;
        }
        
        let time = self.start_time.elapsed().as_secs_f32();
        
        // Обновляем uniforms
        let uniforms = HotbarUniforms {
            screen_size: [self.screen_width, self.screen_height],
            time,
            selected_slot: hotbar.selected() as f32,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        
        // Вычисляем позиции слотов
        let hotbar_width = HOTBAR_SLOTS as f32 * SLOT_SIZE + (HOTBAR_SLOTS - 1) as f32 * SLOT_GAP;
        let hotbar_x = (self.screen_width - hotbar_width) / 2.0;
        let hotbar_y = self.screen_height - BOTTOM_PADDING - SLOT_SIZE;
        
        let mut instances: Vec<HotbarSlot> = Vec::with_capacity(HOTBAR_SLOTS + 1);
        
        // Фон хотбара (первый instance с slot_index = 99)
        let bg_padding = 10.0;
        instances.push(HotbarSlot {
            pos: [hotbar_x - bg_padding, hotbar_y - bg_padding],
            size: [hotbar_width + bg_padding * 2.0, SLOT_SIZE + bg_padding * 2.0],
            slot_index: 99, // Специальный индекс для фона
            is_selected: 0,
            has_item: 0,
            _padding: 0,
            top_color: [0.0, 0.0, 0.0, 0.0],
            side_color: [0.0, 0.0, 0.0, 0.0],
        });
        
        // Слоты
        for i in 0..HOTBAR_SLOTS {
            let slot_x = hotbar_x + i as f32 * (SLOT_SIZE + SLOT_GAP);
            let item = hotbar.get_item(i);
            
            let (top_color, side_color) = if let Some(it) = item {
                ([it.top_color[0], it.top_color[1], it.top_color[2], 1.0],
                 [it.side_color[0], it.side_color[1], it.side_color[2], 1.0])
            } else {
                ([0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0])
            };
            
            instances.push(HotbarSlot {
                pos: [slot_x, hotbar_y],
                size: [SLOT_SIZE, SLOT_SIZE],
                slot_index: i as u32,
                is_selected: if i == hotbar.selected() { 1 } else { 0 },
                has_item: if item.is_some() { 1 } else { 0 },
                _padding: 0,
                top_color,
                side_color,
            });
        }
        
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw(0..6, 0..instances.len() as u32);
    }
}
