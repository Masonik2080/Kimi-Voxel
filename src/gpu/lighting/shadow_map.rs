// ============================================
// Shadow Map - GPU текстура глубины для теней
// ============================================

use bytemuck::{Pod, Zeroable};
use ultraviolet::Mat4;

/// GPU ресурсы для одного shadow map
pub struct ShadowMap {
    /// Текстура глубины
    pub texture: wgpu::Texture,
    /// View для рендеринга в текстуру
    pub depth_view: wgpu::TextureView,
    /// View для сэмплирования в шейдере
    pub sample_view: wgpu::TextureView,
    /// Sampler для PCF фильтрации
    pub sampler: wgpu::Sampler,
    /// Разрешение
    pub resolution: u32,
}

impl ShadowMap {
    pub fn new(device: &wgpu::Device, resolution: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map Texture"),
            size: wgpu::Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT 
                 | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        
        let depth_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Shadow Map Depth View"),
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });
        
        let sample_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Shadow Map Sample View"),
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });
        
        // Sampler с comparison для PCF
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Map Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        
        Self {
            texture,
            depth_view,
            sample_view,
            sampler,
            resolution,
        }
    }
}

/// Shadow map array для CSM (все каскады в одной текстуре)
pub struct ShadowMapArray {
    /// Текстура-массив глубины
    pub texture: wgpu::Texture,
    /// Views для рендеринга каждого слоя
    pub layer_views: Vec<wgpu::TextureView>,
    /// View для сэмплирования всего массива
    pub array_view: wgpu::TextureView,
    /// Sampler с comparison
    pub sampler: wgpu::Sampler,
    /// Разрешение каждого слоя
    pub resolution: u32,
    /// Количество каскадов
    pub num_cascades: u32,
}

impl ShadowMapArray {
    pub fn new(device: &wgpu::Device, resolution: u32, num_cascades: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("CSM Shadow Map Array"),
            size: wgpu::Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: num_cascades,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT 
                 | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        
        // View для каждого слоя (для рендеринга)
        let layer_views: Vec<_> = (0..num_cascades)
            .map(|i| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(&format!("Shadow Map Layer {} View", i)),
                    format: Some(wgpu::TextureFormat::Depth32Float),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::DepthOnly,
                    base_array_layer: i,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        
        // View для всего массива (для сэмплирования)
        let array_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("CSM Shadow Map Array View"),
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_array_layer: 0,
            array_layer_count: Some(num_cascades),
            ..Default::default()
        });
        
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("CSM Shadow Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        
        Self {
            texture,
            layer_views,
            array_view,
            sampler,
            resolution,
            num_cascades,
        }
    }
}

/// Uniform буфер для shadow матриц
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ShadowUniform {
    /// Матрицы view-projection для каждого каскада (макс 4)
    pub light_view_proj: [[f32; 16]; 4],
    /// Дальности каскадов
    pub cascade_distances: [f32; 4],
    /// Количество активных каскадов
    pub num_cascades: u32,
    /// Размер текселя для bias
    pub texel_size: f32,
    /// Shadow bias
    pub bias: f32,
    /// Normal offset bias
    pub normal_bias: f32,
}

impl ShadowUniform {
    pub fn new() -> Self {
        Self {
            light_view_proj: [[0.0; 16]; 4],
            cascade_distances: [16.0, 64.0, 256.0, 1024.0],
            num_cascades: 4,
            texel_size: 0.001,
            bias: 0.005,
            normal_bias: 0.02,
        }
    }
    
    pub fn set_cascade(&mut self, index: usize, matrix: &Mat4, distance: f32) {
        if index < 4 {
            let arr: [[f32; 4]; 4] = (*matrix).into();
            for (i, row) in arr.iter().enumerate() {
                for (j, val) in row.iter().enumerate() {
                    self.light_view_proj[index][i * 4 + j] = *val;
                }
            }
            self.cascade_distances[index] = distance;
        }
    }
}

impl Default for ShadowUniform {
    fn default() -> Self {
        Self::new()
    }
}
