use std::sync::Arc;
use ultraviolet::{Mat4, Vec3};

use crate::gpu::render::uniforms::Uniforms;
use crate::gpu::render::shadow::ShadowResources;
use crate::gpu::render::pipelines::Pipelines;
use crate::gpu::render::bind_groups::{CoreBindGroups, AtlasResources};

use crate::gpu::player::PlayerModel;
use crate::gpu::gui::{Crosshair, BlockHighlight};
use crate::gpu::terrain::{HybridTerrainManager, GpuChunkManager, SectionTerrainManager};
use crate::gpu::gui::FpsCounter;
use crate::gpu::lighting::DayNightCycle;
use crate::gpu::lighting::CelestialRenderer;

/// Основное состояние рендерера (GPU ресурсы)
pub struct RendererState {
    pub surface: wgpu::Surface<'static>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
}

/// Компоненты рендеринга
pub struct RenderComponents {
    pub pipelines: Pipelines,
    pub gpu_chunks: GpuChunkManager,
    pub player_model: PlayerModel,
    pub crosshair: Crosshair,
    pub block_highlight: BlockHighlight,
    pub fps_counter: FpsCounter,
    pub celestial: CelestialRenderer,
}

/// Ресурсы освещения и теней
pub struct LightingResources {
    pub core_bind_groups: CoreBindGroups,
    pub shadow: ShadowResources,
    pub day_night: DayNightCycle,
    pub layouts: crate::gpu::render::bind_groups::BindGroupLayouts,
    pub atlas: AtlasResources,
}

/// Ресурсы террейна
pub struct TerrainResources {
    pub depth_texture: wgpu::TextureView,
    pub terrain_manager: HybridTerrainManager,
    #[allow(dead_code)]
    pub section_manager: SectionTerrainManager,
}

/// Кэшированные данные камеры
pub struct CachedCamera {
    pub view_proj: [[f32; 4]; 4],
    pub view: Mat4,
    pub proj: Mat4,
    pub position: Vec3,
}

impl Default for CachedCamera {
    fn default() -> Self {
        Self {
            view_proj: Mat4::identity().into(),
            view: Mat4::identity(),
            proj: Mat4::identity(),
            position: Vec3::zero(),
        }
    }
}

impl CachedCamera {
    pub fn update(&mut self, uniforms: &Uniforms, view: Mat4, proj: Mat4, position: Vec3) {
        self.view_proj = uniforms.view_proj;
        self.view = view;
        self.proj = proj;
        self.position = position;
    }
}
