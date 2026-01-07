use crate::gpu::terrain::TerrainVertex;
use crate::gpu::player::PlayerVertex;

use super::bind_groups::BindGroupLayouts;

pub struct Pipelines {
    pub terrain: wgpu::RenderPipeline,
    pub shadow: wgpu::RenderPipeline,
    pub player: wgpu::RenderPipeline,
}

impl Pipelines {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        layouts: &BindGroupLayouts,
        model_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let terrain_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terrain Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/terrain_shadows.wgsl").into()),
        });

        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../lighting/shadow.wgsl").into()),
        });

        let player_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Player Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../player/player.wgsl").into()),
        });

        let terrain_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Terrain Layout"),
            bind_group_layouts: &[&layouts.uniform, &layouts.light, &layouts.shadow, &layouts.atlas],
            push_constant_ranges: &[],
        });

        let shadow_pl_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shadow PL Layout"),
            bind_group_layouts: &[&layouts.shadow_pass],
            push_constant_ranges: &[],
        });

        let player_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Player Layout"),
            bind_group_layouts: &[&layouts.uniform, model_layout],
            push_constant_ranges: &[],
        });

        let terrain = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Terrain Pipeline"),
            layout: Some(&terrain_layout),
            vertex: wgpu::VertexState {
                module: &terrain_shader,
                entry_point: Some("vs_main"),
                buffers: &[TerrainVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &terrain_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater, // Reversed-Z
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let shadow = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Pipeline"),
            layout: Some(&shadow_pl_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: Some("vs_main"),
                buffers: &[TerrainVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // Shadow pass остаётся Less
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: 4,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        let player = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Player Pipeline"),
            layout: Some(&player_layout),
            vertex: wgpu::VertexState {
                module: &player_shader,
                entry_point: Some("vs_main"),
                buffers: &[PlayerVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &player_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater, // Reversed-Z
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            terrain,
            shadow,
            player,
        }
    }
}
