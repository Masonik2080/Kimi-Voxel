// ============================================
// Cascaded Shadow Maps (CSM) - Главный класс
// ============================================
// Оптимизированная система теней для больших миров
// - 4 каскада для разных дистанций
// - PCF фильтрация для мягких теней
// - Стабилизация для устранения мерцания

use wgpu::util::DeviceExt;
use ultraviolet::{Vec3, Mat4};

use super::cascade::{Cascade, CascadeConfig};
use super::shadow_map::{ShadowMapArray, ShadowUniform};
use super::light::DirectionalLight;

/// Главная система Cascaded Shadow Maps
pub struct CascadedShadowMaps {
    /// Конфигурация
    config: CascadeConfig,
    /// Каскады
    cascades: Vec<Cascade>,
    /// GPU текстуры теней
    shadow_maps: ShadowMapArray,
    /// Uniform буфер для шейдера
    uniform_buffer: wgpu::Buffer,
    /// Bind group для сэмплирования теней
    bind_group: wgpu::BindGroup,
    /// Bind group layout
    bind_group_layout: wgpu::BindGroupLayout,
    /// Pipeline для рендеринга теней
    shadow_pipeline: wgpu::RenderPipeline,
    /// Текущие данные uniform
    uniform_data: ShadowUniform,
}

impl CascadedShadowMaps {
    pub fn new(
        device: &wgpu::Device,
        config: CascadeConfig,
        terrain_vertex_layout: &wgpu::VertexBufferLayout,
    ) -> Self {
        let num_cascades = config.num_cascades;
        let resolution = config.resolution;
        
        // Создаём каскады
        let mut cascades = Vec::with_capacity(num_cascades);
        let mut prev_dist = 0.0;
        
        for (i, &dist) in config.cascade_distances.iter().enumerate() {
            cascades.push(Cascade::new(i, prev_dist, dist));
            prev_dist = dist * (1.0 - config.overlap_factor);
        }
        
        // Создаём shadow map array
        let shadow_maps = ShadowMapArray::new(device, resolution, num_cascades as u32);
        
        // Uniform буфер
        let uniform_data = ShadowUniform::new();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("CSM Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Bind group layout для сэмплирования теней
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("CSM Bind Group Layout"),
            entries: &[
                // Shadow map array
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
                // Comparison sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                // Shadow uniforms
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
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CSM Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_maps.array_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_maps.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Shadow pipeline (только depth, без fragment shader)
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
        });
        
        // Layout для shadow pass (только light matrix uniform)
        let shadow_uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shadow Pass Uniform Layout"),
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
        
        let shadow_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shadow Pipeline Layout"),
            bind_group_layouts: &[&shadow_uniform_layout],
            push_constant_ranges: &[],
        });
        
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: Some("vs_main"),
                buffers: &[terrain_vertex_layout.clone()],
                compilation_options: Default::default(),
            },
            fragment: None, // Только depth
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // Depth bias для борьбы с shadow acne
                cull_mode: Some(wgpu::Face::Front), // Front-face culling для теней
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        
        Self {
            config,
            cascades,
            shadow_maps,
            uniform_buffer,
            bind_group,
            bind_group_layout,
            shadow_pipeline,
            uniform_data,
        }
    }
    
    /// Обновить каскады на основе камеры и света
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        camera_view: &Mat4,
        camera_proj: &Mat4,
        _camera_pos: Vec3,
        light: &DirectionalLight,
    ) {
        let camera_inv_view_proj = (*camera_proj * *camera_view).inversed();
        
        // Обновляем каждый каскад
        for cascade in &mut self.cascades {
            // Вычисляем frustum для этого каскада
            let near_ratio = cascade.near / self.config.cascade_distances.last().unwrap_or(&1000.0);
            let far_ratio = cascade.far / self.config.cascade_distances.last().unwrap_or(&1000.0);
            
            // Интерполируем frustum corners
            let full_corners = cascade.compute_frustum_corners(&camera_inv_view_proj);
            
            let mut cascade_corners = [Vec3::zero(); 8];
            for i in 0..4 {
                // Near plane
                cascade_corners[i] = full_corners[i] + (full_corners[i + 4] - full_corners[i]) * near_ratio;
                // Far plane
                cascade_corners[i + 4] = full_corners[i] + (full_corners[i + 4] - full_corners[i]) * far_ratio;
            }
            
            cascade.update_light_matrix(
                light.direction,
                &cascade_corners,
                self.config.resolution,
                self.config.stabilize,
            );
            
            // Обновляем uniform
            self.uniform_data.set_cascade(
                cascade.index,
                &cascade.light_view_proj,
                cascade.far,
            );
        }
        
        self.uniform_data.num_cascades = self.cascades.len() as u32;
        
        // Загружаем на GPU
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniform_data]),
        );
    }
    
    /// Получить bind group layout для интеграции в основной pipeline
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
    
    /// Получить bind group для сэмплирования теней
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
    
    /// Получить shadow pipeline
    pub fn shadow_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.shadow_pipeline
    }
    
    /// Получить view для рендеринга в конкретный каскад
    pub fn cascade_view(&self, index: usize) -> Option<&wgpu::TextureView> {
        self.shadow_maps.layer_views.get(index)
    }
    
    /// Получить матрицу света для каскада
    pub fn cascade_matrix(&self, index: usize) -> Option<Mat4> {
        self.cascades.get(index).map(|c| c.light_view_proj)
    }
    
    /// Количество каскадов
    pub fn num_cascades(&self) -> usize {
        self.cascades.len()
    }
    
    /// Разрешение shadow map
    pub fn resolution(&self) -> u32 {
        self.config.resolution
    }
}

/// Uniform для shadow pass (одна матрица)
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowPassUniform {
    pub light_view_proj: [[f32; 4]; 4],
}

impl ShadowPassUniform {
    pub fn new(matrix: &Mat4) -> Self {
        Self {
            light_view_proj: (*matrix).into(),
        }
    }
}
