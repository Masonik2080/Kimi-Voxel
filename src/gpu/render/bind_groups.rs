use wgpu::util::DeviceExt;

pub struct BindGroupLayouts {
    pub uniform: wgpu::BindGroupLayout,
    pub light: wgpu::BindGroupLayout,
    pub shadow: wgpu::BindGroupLayout,
    pub shadow_pass: wgpu::BindGroupLayout,
    pub atlas: wgpu::BindGroupLayout,
}

impl BindGroupLayouts {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Layout"),
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

        let light = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Light Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let shadow = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shadow Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let shadow_pass = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shadow Pass Layout"),
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

        // Текстурный атлас для кастомных блоков
        let atlas = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Atlas Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        Self {
            uniform,
            light,
            shadow,
            shadow_pass,
            atlas,
        }
    }
}

pub struct CoreBindGroups {
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub light_buffer: wgpu::Buffer,
    pub light_bind_group: wgpu::BindGroup,
}

impl CoreBindGroups {
    pub fn new(device: &wgpu::Device, layouts: &BindGroupLayouts) -> Self {
        use super::uniforms::{Uniforms, LightUniform};

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[Uniforms::new()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light Buffer"),
            contents: bytemuck::cast_slice(&[LightUniform::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform BG"),
            layout: &layouts.uniform,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light BG"),
            layout: &layouts.light,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        });

        Self {
            uniform_buffer,
            uniform_bind_group,
            light_buffer,
            light_bind_group,
        }
    }
}

/// Ресурсы текстурного атласа для кастомных блоков
pub struct AtlasResources {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
}

impl AtlasResources {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> Self {
        use crate::gpu::blocks::texture_atlas::{BlockTextureAtlas, ATLAS_PIXELS};
        
        // Генерируем атлас из реестра блоков
        let atlas = BlockTextureAtlas::from_registry();
        
        // Создаём GPU текстуру
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Block Atlas Texture"),
            size: wgpu::Extent3d {
                width: ATLAS_PIXELS,
                height: ATLAS_PIXELS,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        // Загружаем данные атласа в текстуру
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ATLAS_PIXELS * 4),
                rows_per_image: Some(ATLAS_PIXELS),
            },
            wgpu::Extent3d {
                width: ATLAS_PIXELS,
                height: ATLAS_PIXELS,
                depth_or_array_layers: 1,
            },
        );
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Nearest-neighbor сэмплер для пиксельного стиля
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Atlas Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Atlas Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        
        Self {
            texture,
            view,
            sampler,
            bind_group,
        }
    }
}
