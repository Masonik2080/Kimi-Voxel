pub mod core;
mod culling;
mod passes;
mod systems;

use std::sync::Arc;

use crate::gpu::render::depth::create_depth_texture;
use crate::gpu::player::Camera;
use crate::gpu::player::Player;
use crate::gpu::terrain::WorldChanges;

use core::{RendererState, RenderComponents, LightingResources, TerrainResources, CachedCamera};

pub struct Renderer {
    state: RendererState,
    components: RenderComponents,
    lighting: LightingResources,
    terrain: TerrainResources,
    cached: CachedCamera,
}

impl Renderer {
    pub async fn new(window: Arc<winit::window::Window>) -> Self {
        let (surface, device, queue, config, size) = core::init_gpu(window).await;
        let (components, lighting, terrain) = core::init_components(&device, &queue, &config);

        Self {
            state: RendererState { surface, device, queue, config, size },
            components,
            lighting,
            terrain,
            cached: CachedCamera::default(),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.state.size = new_size;
            self.state.config.width = new_size.width;
            self.state.config.height = new_size.height;
            self.state.surface.configure(&self.state.device, &self.state.config);
            self.terrain.depth_texture = create_depth_texture(&self.state.device, &self.state.config);
        }
    }

    pub fn update(&mut self, camera: &Camera, player: &Player, time: f32, dt: f32, world_changes: &WorldChanges) {
        systems::frame::update(
            &self.state.queue,
            camera,
            player,
            time,
            dt,
            world_changes,
            &mut self.components,
            &mut self.lighting,
            &mut self.terrain,
            &mut self.cached,
        );
    }

    pub fn instant_chunk_update(&mut self, block_x: i32, block_y: i32, block_z: i32, world_changes: &WorldChanges) {
        systems::terrain::instant_chunk_update(
            &mut self.components.gpu_chunks,
            block_x,
            block_y,
            block_z,
            world_changes,
        );
    }

    pub fn update_block_highlight(&self, block_pos: Option<[i32; 3]>) {
        systems::terrain::update_block_highlight(
            &self.state.queue,
            &self.components.block_highlight,
            self.cached.view_proj,
            block_pos,
        );
    }
    
    /// Обновить выделение с произвольной позицией и размером (для суб-вокселей)
    pub fn update_block_highlight_sized(&self, pos: [f32; 3], size: f32) {
        self.components.block_highlight.update_with_size(
            &self.state.queue,
            self.cached.view_proj,
            pos,
            size,
        );
    }

    pub fn render(&mut self, render_player: bool, highlight_block: Option<[i32; 3]>) -> Result<(), wgpu::SurfaceError> {
        self.components.fps_counter.update();

        let output = self.state.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Shadow pass
        passes::shadow::render(
            &mut encoder,
            &self.lighting.shadow,
            &self.components.pipelines,
            &self.components.gpu_chunks,
            None, // No subvoxels in basic render
        );

        // Main 3D pass
        passes::main_pass::render(
            &mut encoder,
            &view,
            &self.terrain.depth_texture,
            self.lighting.day_night.sky_color,
            &self.cached.view_proj,
            &self.components.pipelines,
            &self.lighting.core_bind_groups,
            &self.lighting.shadow,
            &self.lighting.atlas,
            &self.components,
            render_player,
            highlight_block,
        );

        // UI pass
        passes::ui::render(&mut encoder, &view, &self.components);

        self.state.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
    
    /// Рендерит с GUI поверх
    pub fn render_with_gui<F>(&mut self, render_player: bool, highlight_block: Option<[i32; 3]>, gui_render: F) -> Result<(), wgpu::SurfaceError>
    where
        F: FnOnce(&wgpu::Device, &mut wgpu::CommandEncoder, &wgpu::TextureView, &wgpu::Queue),
    {
        self.components.fps_counter.update();

        let output = self.state.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Shadow pass
        passes::shadow::render(
            &mut encoder,
            &self.lighting.shadow,
            &self.components.pipelines,
            &self.components.gpu_chunks,
            None, // No subvoxels in basic render_with_gui
        );

        // Main 3D pass
        passes::main_pass::render(
            &mut encoder,
            &view,
            &self.terrain.depth_texture,
            self.lighting.day_night.sky_color,
            &self.cached.view_proj,
            &self.components.pipelines,
            &self.lighting.core_bind_groups,
            &self.lighting.shadow,
            &self.lighting.atlas,
            &self.components,
            render_player,
            highlight_block,
        );

        // UI pass
        passes::ui::render(&mut encoder, &view, &self.components);
        
        // GUI pass (меню и т.п.)
        gui_render(&self.state.device, &mut encoder, &view, &self.state.queue);

        self.state.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
    
    /// Рендерит с GUI и суб-вокселями
    pub fn render_with_subvoxels<F>(
        &mut self, 
        render_player: bool, 
        highlight_block: Option<[i32; 3]>,
        subvoxel_renderer: Option<&crate::gpu::subvoxel::SubVoxelRenderer>,
        gui_render: F
    ) -> Result<(), wgpu::SurfaceError>
    where
        F: FnOnce(&wgpu::Device, &mut wgpu::CommandEncoder, &wgpu::TextureView, &wgpu::Queue),
    {
        self.components.fps_counter.update();

        let output = self.state.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Shadow pass
        passes::shadow::render(
            &mut encoder,
            &self.lighting.shadow,
            &self.components.pipelines,
            &self.components.gpu_chunks,
            subvoxel_renderer,
        );

        // Main 3D pass
        passes::main_pass::render(
            &mut encoder,
            &view,
            &self.terrain.depth_texture,
            self.lighting.day_night.sky_color,
            &self.cached.view_proj,
            &self.components.pipelines,
            &self.lighting.core_bind_groups,
            &self.lighting.shadow,
            &self.lighting.atlas,
            &self.components,
            render_player,
            highlight_block,
        );
        
        // SubVoxel pass
        if let Some(sv_renderer) = subvoxel_renderer {
            if sv_renderer.has_content() {
                passes::subvoxel::render(
                    &mut encoder,
                    &view,
                    &self.terrain.depth_texture,
                    &self.components.pipelines,
                    &self.lighting.core_bind_groups,
                    &self.lighting.shadow,
                    &self.lighting.atlas,
                    sv_renderer,
                );
            }
        }

        // UI pass
        passes::ui::render(&mut encoder, &view, &self.components);
        
        // GUI pass
        gui_render(&self.state.device, &mut encoder, &view, &self.state.queue);

        self.state.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    pub fn set_time_of_day(&mut self, time: f32) {
        self.lighting.day_night.set_time(time);
    }

    pub fn set_time_speed(&mut self, speed: f32) {
        self.lighting.day_night.set_speed(speed);
    }

    pub fn time_of_day(&self) -> f32 {
        self.lighting.day_night.time.time
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.state.size
    }
    
    pub fn device(&self) -> &wgpu::Device {
        &self.state.device
    }
    
    pub fn queue(&self) -> &wgpu::Queue {
        &self.state.queue
    }
    
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.state.config.format
    }
    
    /// Возвращает uniform bind group layout для GUI
    pub fn uniform_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.lighting.layouts.uniform
    }
    
    /// Возвращает uniform bind group для GUI рендеринга
    pub fn uniform_bind_group(&self) -> &wgpu::BindGroup {
        &self.lighting.core_bind_groups.uniform_bind_group
    }
    
    /// Установить дистанции LOD (в чанках)
    /// distances: [LOD0, LOD1, LOD2, LOD3] - максимальные дистанции для каждого уровня
    pub fn set_lod_distances(&mut self, distances: [i32; 4]) {
        self.terrain.terrain_manager.set_lod_distances(distances);
    }
    
    /// Получить текущие дистанции LOD
    pub fn get_lod_distances(&self) -> [i32; 4] {
        self.terrain.terrain_manager.get_lod_distances()
    }
}
