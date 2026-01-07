use std::sync::Arc;

use crate::gpu::render::depth::create_depth_texture;
use crate::gpu::render::bind_groups::{BindGroupLayouts, CoreBindGroups, AtlasResources};
use crate::gpu::render::shadow::ShadowResources;
use crate::gpu::render::pipelines::Pipelines;

use crate::gpu::player::PlayerModel;
use crate::gpu::gui::{Crosshair, BlockHighlight};
use crate::gpu::terrain::{HybridTerrainManager, GpuChunkManager, SectionTerrainManager};
use crate::gpu::gui::FpsCounter;
use crate::gpu::lighting::DayNightCycle;
use crate::gpu::lighting::CelestialRenderer;

use super::state::{RenderComponents, LightingResources, TerrainResources};

/// Инициализация GPU устройства и surface
pub async fn init_gpu(window: Arc<winit::window::Window>) -> (
    wgpu::Surface<'static>,
    Arc<wgpu::Device>,
    Arc<wgpu::Queue>,
    wgpu::SurfaceConfiguration,
    winit::dpi::PhysicalSize<u32>,
) {
    let size = window.inner_size();
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = instance.create_surface(window).unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("GPU Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            },
        )
        .await
        .unwrap();

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    (surface, device, queue, config, size)
}

/// Инициализация всех компонентов рендеринга
pub fn init_components(
    device: &Arc<wgpu::Device>,
    queue: &Arc<wgpu::Queue>,
    config: &wgpu::SurfaceConfiguration,
) -> (RenderComponents, LightingResources, TerrainResources) {
    let depth_texture = create_depth_texture(device, config);

    // Bind group layouts
    let layouts = BindGroupLayouts::new(device);
    let model_layout = PlayerModel::create_bind_group_layout(device);

    // Core bind groups
    let core_bind_groups = CoreBindGroups::new(device, &layouts);

    // Atlas resources (текстурный атлас для кастомных блоков)
    let atlas = AtlasResources::new(device, queue, &layouts.atlas);

    // Shadow resources
    let shadow = ShadowResources::new(device, &layouts.shadow, &layouts.shadow_pass);

    // Pipelines
    let pipelines = Pipelines::new(device, config.format, &layouts, &model_layout);

    // Terrain
    let mut gpu_chunks = GpuChunkManager::new(Arc::clone(device));
    let mut terrain_manager = HybridTerrainManager::new();
    let initial_mesh = terrain_manager.generate_initial(0.0, 0.0);
    let section_manager = SectionTerrainManager::new();

    for chunk_data in &initial_mesh.new_chunks {
        gpu_chunks.upload(chunk_data.key, &chunk_data.vertices, &chunk_data.indices);
    }

    // Other components
    let player_model = PlayerModel::new(device, &model_layout);
    let crosshair = Crosshair::new(device, config.format);
    let block_highlight = BlockHighlight::new(device, config.format);
    let fps_counter = FpsCounter::new(device, Arc::clone(queue), config.format);
    let celestial = CelestialRenderer::new(device, config.format);

    let mut day_night = DayNightCycle::new();
    day_night.set_time(0.35);
    day_night.set_speed(3.0);

    let components = RenderComponents {
        pipelines,
        gpu_chunks,
        player_model,
        crosshair,
        block_highlight,
        fps_counter,
        celestial,
    };

    let lighting = LightingResources {
        core_bind_groups,
        shadow,
        day_night,
        layouts,
        atlas,
    };

    let terrain = TerrainResources {
        depth_texture,
        terrain_manager,
        section_manager,
    };

    (components, lighting, terrain)
}
