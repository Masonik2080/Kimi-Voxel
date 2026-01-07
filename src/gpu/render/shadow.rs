use wgpu::util::DeviceExt;
use ultraviolet::{Vec3, Mat4};

use super::uniforms::ShadowUniform;
use crate::gpu::lighting::{CascadeConfig, DayNightCycle};

pub struct ShadowResources {
    pub texture: wgpu::Texture,
    pub views: Vec<wgpu::TextureView>,
    pub array_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub pass_buffers: Vec<wgpu::Buffer>,
    pub pass_bind_groups: Vec<wgpu::BindGroup>,
    pub config: CascadeConfig,
    pub uniform: ShadowUniform,
}

impl ShadowResources {
    pub fn new(
        device: &wgpu::Device,
        shadow_layout: &wgpu::BindGroupLayout,
        shadow_pass_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let config = CascadeConfig::large_world();
        let num_cascades = config.num_cascades as u32;
        let shadow_res = config.resolution;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map Array"),
            size: wgpu::Extent3d {
                width: shadow_res,
                height: shadow_res,
                depth_or_array_layers: num_cascades,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let views: Vec<_> = (0..num_cascades)
            .map(|i| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(&format!("Shadow Layer {}", i)),
                    format: Some(wgpu::TextureFormat::Depth32Float),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::DepthOnly,
                    base_array_layer: i,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();

        let array_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Shadow Array View"),
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_array_layer: 0,
            array_layer_count: Some(num_cascades),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let uniform = ShadowUniform::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Shadow Uniform"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow BG"),
            layout: shadow_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&array_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let pass_buffers: Vec<_> = (0..num_cascades)
            .map(|i| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Shadow Pass {}", i)),
                    contents: bytemuck::cast_slice(&[[[0.0f32; 4]; 4]]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                })
            })
            .collect();

        let pass_bind_groups: Vec<_> = pass_buffers
            .iter()
            .enumerate()
            .map(|(i, buf)| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("Shadow Pass BG {}", i)),
                    layout: shadow_pass_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buf.as_entire_binding(),
                    }],
                })
            })
            .collect();

        println!("CSM Shadows: {} cascades @ {}x{}", num_cascades, shadow_res, shadow_res);

        Self {
            texture,
            views,
            array_view,
            sampler,
            uniform_buffer,
            bind_group,
            pass_buffers,
            pass_bind_groups,
            config,
            uniform,
        }
    }

    pub fn compute_cascade_matrix(&self, cascade_idx: usize, camera_pos: Vec3, day_night: &DayNightCycle) -> Mat4 {
        let dist = self.config.cascade_distances[cascade_idx];
        let light_dir = day_night.shadow_light_direction();
        let resolution = self.config.resolution as f32;

        let size = dist * 1.5;

        let up = if light_dir.y.abs() > 0.99 {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };

        // Создаём матрицу вида света
        let light_pos = camera_pos - light_dir * dist * 2.0;
        let light_view = Mat4::look_at(light_pos, camera_pos, up);

        let near = 1.0;
        let far = dist * 4.0;
        let light_proj = ultraviolet::projection::orthographic_wgpu_dx(-size, size, -size, size, near, far);

        let mut light_vp = light_proj * light_view;

        // === СТАБИЛИЗАЦИЯ ===
        // Размер одного текселя в NDC пространстве
        let texel_size_ndc = 2.0 / resolution;
        
        // Трансформируем точку (0,0,0) в light space чтобы найти смещение
        let shadow_origin = light_vp * ultraviolet::Vec4::new(0.0, 0.0, 0.0, 1.0);
        
        // Округляем до границы текселя
        let offset_x = (shadow_origin.x % texel_size_ndc);
        let offset_y = (shadow_origin.y % texel_size_ndc);
        
        // Применяем коррекцию напрямую к матрице (последний столбец - translation)
        light_vp.cols[3].x -= offset_x;
        light_vp.cols[3].y -= offset_y;

        light_vp
    }

    pub fn update(&mut self, queue: &wgpu::Queue, camera_pos: Vec3, day_night: &DayNightCycle) {
        for i in 0..self.config.num_cascades {
            let matrix = self.compute_cascade_matrix(i, camera_pos, day_night);
            let arr: [[f32; 4]; 4] = matrix.into();
            self.uniform.light_vp[i] = arr;
            queue.write_buffer(&self.pass_buffers[i], 0, bytemuck::cast_slice(&[arr]));
        }
        self.uniform.cascade_splits = [
            self.config.cascade_distances[0],
            self.config.cascade_distances[1],
            self.config.cascade_distances[2],
            self.config.cascade_distances[3],
        ];
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}
