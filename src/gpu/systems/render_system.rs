// ============================================
// Render System - Система рендеринга
// ============================================

use winit::event_loop::ActiveEventLoop;

use crate::gpu::core::GameResources;
use crate::gpu::subvoxel::SubVoxelLevel;
use crate::gpu::systems::menu_system::MenuSystem;

/// Система рендеринга
pub struct RenderSystem;

impl RenderSystem {
    /// Основной рендер-пасс
    pub fn render(resources: &mut GameResources, time: f32, dt: f32, event_loop: &ActiveEventLoop) {
        let Some(renderer) = &mut resources.renderer else { return };
        
        // Обновляем рендерер
        {
            let changes = resources.world_changes.read().unwrap();
            renderer.update(&resources.camera, &resources.player, time, dt, &changes);
        }
        
        // Обновляем листву деревьев (субвоксели)
        {
            let mut subvoxels = resources.subvoxel_storage.write().unwrap();
            resources.foliage_cache.update(
                &mut subvoxels,
                resources.player.position.x,
                resources.player.position.z,
                4, // render distance в чанках для листвы
            );
        }
        
        // Обновляем суб-воксели
        if let Some(sv_renderer) = &mut resources.subvoxel_renderer {
            let subvoxels = resources.subvoxel_storage.read().unwrap();
            sv_renderer.update(renderer.device(), renderer.queue(), &subvoxels);
        }
        
        // Raycast для выделения
        let (highlight_block, should_highlight) = Self::calculate_highlight(resources);
        
        // Обновляем hover меню
        MenuSystem::update_hover(resources);
        
        // Рендерим
        let render_player = resources.camera.should_render_player();
        let sv_renderer = resources.subvoxel_renderer.as_ref();
        let highlight_for_render = if should_highlight { Some([0, 0, 0]) } else { None };
        let mouse_pos = resources.mouse_pos;
        
        let result = if resources.gui_renderer.is_some() {
            let gui = resources.gui_renderer.as_mut().unwrap();
            let renderer = resources.renderer.as_mut().unwrap();
            renderer.render_with_subvoxels(render_player, highlight_for_render, sv_renderer, |device, encoder, view, queue| {
                gui.render(device, encoder, view, queue, mouse_pos);
            })
        } else {
            let renderer = resources.renderer.as_mut().unwrap();
            renderer.render(render_player, highlight_block)
        };
        
        match result {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost) => {
                let renderer = resources.renderer.as_mut().unwrap();
                renderer.resize(renderer.size());
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                event_loop.exit();
            }
            Err(e) => eprintln!("Render error: {:?}", e),
        }
    }
    
    /// Вычисление подсветки блока/суб-вокселя
    fn calculate_highlight(resources: &mut GameResources) -> (Option<[i32; 3]>, bool) {
        let eye_pos = resources.player.eye_position();
        let forward = resources.player.forward();
        let origin = [eye_pos.x, eye_pos.y, eye_pos.z];
        let direction = [forward.x, forward.y, forward.z];
        
        // Ищем ближайший суб-воксель
        let mut closest_subvoxel: Option<crate::gpu::subvoxel::SubVoxelHit> = None;
        {
            let subvoxels = resources.subvoxel_storage.read().unwrap();
            for level in [SubVoxelLevel::Quarter, SubVoxelLevel::Half] {
                if let Some(hit) = subvoxels.raycast(origin, direction, 5.0, level) {
                    if closest_subvoxel.is_none() || hit.distance < closest_subvoxel.as_ref().unwrap().distance {
                        closest_subvoxel = Some(hit);
                    }
                }
            }
        }
        
        // Получаем позицию обычного блока
        let block_hit = resources.block_breaker.target_block();
        let block_dist = block_hit.map(|b| b.distance).unwrap_or(f32::MAX);
        
        // Выбираем что выделять
        let highlight_block = if let Some(sv_hit) = &closest_subvoxel {
            if sv_hit.distance < block_dist {
                // Выделяем суб-воксель
                let [x, y, z] = sv_hit.pos.world_min();
                let size = sv_hit.pos.level.size();
                if let Some(renderer) = &mut resources.renderer {
                    renderer.update_block_highlight_sized([x, y, z], size);
                }
                None
            } else {
                resources.block_breaker.highlight_block_pos()
            }
        } else {
            resources.block_breaker.highlight_block_pos()
        };
        
        if let Some(pos) = highlight_block {
            if let Some(renderer) = &mut resources.renderer {
                renderer.update_block_highlight(Some(pos));
            }
        }
        
        let should_highlight = highlight_block.is_some() 
            || closest_subvoxel.as_ref().map(|sv| sv.distance < block_dist).unwrap_or(false);
        
        (highlight_block, should_highlight)
    }
}
