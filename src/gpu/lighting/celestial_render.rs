// ============================================
// Celestial Bodies Renderer - Солнце и Луна
// ============================================

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use ultraviolet::{Vec3, Mat4};

use crate::gpu::lighting::DayNightCycle;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CelestialVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

impl CelestialVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CelestialVertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}


/// Uniform данные - все vec3 заменены на vec4 для WGSL alignment
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CelestialUniforms {
    pub view_proj: [[f32; 4]; 4],    // 64 bytes
    pub camera_pos: [f32; 4],         // 16 bytes (xyz + pad)
    pub sun_direction: [f32; 4],      // 16 bytes (xyz + visibility)
    pub sun_color: [f32; 4],          // 16 bytes (rgb + size)
    pub moon_direction: [f32; 4],     // 16 bytes (xyz + visibility)
    pub moon_color: [f32; 4],         // 16 bytes (rgb + phase)
    pub time_of_day: [f32; 4],        // 16 bytes (time + pad)
}

impl Default for CelestialUniforms {
    fn default() -> Self {
        Self {
            view_proj: Mat4::identity().into(),
            camera_pos: [0.0, 0.0, 0.0, 0.0],
            sun_direction: [0.0, 1.0, 0.0, 1.0],
            sun_color: [1.0, 0.95, 0.8, 0.15],
            moon_direction: [0.0, -1.0, 0.0, 0.0],
            moon_color: [0.8, 0.85, 1.0, 0.5],
            time_of_day: [0.5, 0.0, 0.0, 0.0],
        }
    }
}

pub struct CelestialRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}


impl CelestialRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let vertices = vec![
            CelestialVertex { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] },
            CelestialVertex { position: [ 1.0, -1.0, 0.0], uv: [1.0, 1.0] },
            CelestialVertex { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] },
            CelestialVertex { position: [-1.0,  1.0, 0.0], uv: [0.0, 0.0] },
        ];
        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Celestial VB"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Celestial IB"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Celestial UB"),
            contents: bytemuck::cast_slice(&[CelestialUniforms::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Celestial BGL"),
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Celestial BG"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Celestial Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/celestial.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Celestial PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Celestial Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[CelestialVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Max,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::GreaterEqual, // Reversed-Z
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self { vertex_buffer, index_buffer, uniform_buffer, bind_group, pipeline }
    }


    pub fn update(&self, queue: &wgpu::Queue, view_proj: [[f32; 4]; 4], camera_pos: Vec3, day_night: &DayNightCycle) {
        let sun_dir = day_night.sun.body.direction;
        let moon_dir = day_night.moon.body.direction;
        let sun_col = day_night.sun.body.color;
        let moon_col = day_night.moon.body.color;

        let uniforms = CelestialUniforms {
            view_proj,
            camera_pos: [camera_pos.x, camera_pos.y, camera_pos.z, 0.0],
            sun_direction: [sun_dir.x, sun_dir.y, sun_dir.z, day_night.sun.body.visibility],
            sun_color: [sun_col.x, sun_col.y, sun_col.z, 0.12],
            moon_direction: [moon_dir.x, moon_dir.y, moon_dir.z, day_night.moon.body.visibility],
            moon_color: [moon_col.x, moon_col.y, moon_col.z, day_night.moon.phase],
            time_of_day: [day_night.time.time, 0.0, 0.0, 0.0],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..6, 0, 0..2);
    }
}
