// ============================================
// Init System - Инициализация игры
// ============================================

use std::sync::{Arc, RwLock};
use std::time::Instant;
use winit::window::Window;

use crate::gpu::core::GameResources;
use crate::gpu::player::Camera;
use crate::gpu::player::{Player, PlayerController};
use crate::gpu::render::Renderer;
use crate::gpu::blocks::BlockBreaker;
use crate::gpu::terrain::WorldChanges;
use crate::gpu::gui::{GameMenu, GuiRenderer};
use crate::gpu::subvoxel::{SubVoxelStorage, SubVoxelLevel};
use crate::gpu::subvoxel::SubVoxelRenderer;
use crate::gpu::audio::AudioSystem;
use crate::gpu::terrain::{get_height, CaveParams, is_cave};
use crate::gpu::blocks::AIR;
use crate::gpu::systems::save_system::SaveSystem;
use crate::gpu::biomes::FoliageCache;

/// Система инициализации
pub struct InitSystem;

impl InitSystem {
    /// Создать начальные ресурсы игры
    pub fn create_resources() -> GameResources {
        let loaded = SaveSystem::load_or_create();
        
        let mut player = Player::new(loaded.start_x, loaded.start_y, loaded.start_z);
        player.move_speed = 8.0;
        player.sprint_speed = 320.0; // x40 от базовой скорости
        
        let mut player_controller = PlayerController::new(0.5);
        
        // Устанавливаем функцию проверки твёрдости блока
        player_controller.set_block_solid_checker(|bx, by, bz, world_changes: &std::collections::HashMap<crate::gpu::terrain::BlockPos, crate::gpu::blocks::BlockType>| {
            use crate::gpu::terrain::BlockPos;
            
            let pos = BlockPos::new(bx, by, bz);
            
            // Сначала проверяем изменения мира
            if let Some(&block_type) = world_changes.get(&pos) {
                return block_type != AIR;
            }
            
            // Если нет изменений - используем процедурную генерацию
            let base_height = get_height(bx as f32, bz as f32) as i32;
            
            // Выше поверхности - воздух
            if by > base_height {
                return false;
            }
            
            // Проверяем пещеры
            let cave_params = CaveParams::default();
            let cave_ceiling = base_height - cave_params.surface_offset;
            
            if by >= cave_params.min_height && by < cave_ceiling {
                if is_cave(bx, by, bz, &cave_params) {
                    return false;
                }
            }
            
            true
        });
        
        // Создаём хранилище изменений мира
        let world_changes = Arc::new(RwLock::new(WorldChanges::new()));
        SaveSystem::apply_loaded_changes(&world_changes, loaded.changes);
        
        // Создаём хранилище суб-вокселей
        let mut subvoxel_storage_inner = SubVoxelStorage::new();
        SaveSystem::apply_loaded_subvoxels(&mut subvoxel_storage_inner, loaded.subvoxels);
        let subvoxel_storage = Arc::new(RwLock::new(subvoxel_storage_inner));
        
        // Устанавливаем checker для коллизий с суб-вокселями
        let subvoxel_storage_clone = Arc::clone(&subvoxel_storage);
        player_controller.set_subvoxel_collision_checker(move |min_x, min_y, min_z, max_x, max_y, max_z| {
            let storage = subvoxel_storage_clone.read().unwrap();
            storage.check_aabb_collision(min_x, min_y, min_z, max_x, max_y, max_z)
        });
        
        GameResources {
            window: None,
            renderer: None,
            gui_renderer: None,
            subvoxel_renderer: None,
            player,
            player_controller,
            camera: Camera::new(16.0 / 9.0),
            block_breaker: BlockBreaker::new(Arc::clone(&world_changes)),
            world_changes,
            subvoxel_storage,
            current_subvoxel_level: SubVoxelLevel::Full,
            foliage_cache: FoliageCache::new(),
            menu: GameMenu::new(1280, 720),
            audio_system: None,
            start_time: Instant::now(),
            last_frame: Instant::now(),
            cursor_grabbed: false,
            mouse_pos: (0.0, 0.0),
            menu_mouse_pressed: false,
            world_seed: loaded.world_seed,
        }
    }
    
    /// Инициализация рендеринга (вызывается при resumed)
    pub fn init_rendering(resources: &mut GameResources, window: Arc<Window>) {
        let renderer = pollster::block_on(Renderer::new(window.clone()));
        
        // GUI рендерер
        let gui_renderer = GuiRenderer::new(
            renderer.device(),
            renderer.queue(),
            renderer.surface_format(),
            renderer.uniform_bind_group_layout(),
            renderer.size().width,
            renderer.size().height,
        );
        
        // Рендерер суб-вокселей
        let subvoxel_renderer = SubVoxelRenderer::new(renderer.device());
        
        // Аудио система
        Self::init_audio(resources);
        
        resources.camera.resize(renderer.size().width, renderer.size().height);
        resources.menu.resize(renderer.size().width, renderer.size().height);
        resources.window = Some(window);
        resources.renderer = Some(renderer);
        resources.gui_renderer = Some(gui_renderer);
        resources.subvoxel_renderer = Some(subvoxel_renderer);
    }
    
    /// Инициализация аудио системы
    fn init_audio(resources: &mut GameResources) {
        match AudioSystem::new() {
            Ok(mut audio) => {
                if let Err(e) = audio.load_sounds() {
                    eprintln!("[AUDIO] Не удалось загрузить звуки: {}", e);
                }
                
                // Устанавливаем функцию проверки блоков для рейтрейсинга звука
                let world_changes_clone = Arc::clone(&resources.world_changes);
                audio.set_block_checker(move |bx, by, bz| {
                    // Проверяем изменения мира
                    if let Ok(changes) = world_changes_clone.try_read() {
                        if let Some(block_type) = changes.get_block(bx, by, bz) {
                            return block_type != AIR;
                        }
                    }
                    
                    // Процедурная генерация
                    let base_height = get_height(bx as f32, bz as f32) as i32;
                    if by > base_height {
                        return false;
                    }
                    
                    // Проверяем пещеры
                    let cave_params = CaveParams::default();
                    let cave_ceiling = base_height - cave_params.surface_offset;
                    if by >= cave_params.min_height && by < cave_ceiling {
                        if is_cave(bx, by, bz, &cave_params) {
                            return false;
                        }
                    }
                    
                    true
                });
                
                resources.audio_system = Some(audio);
            }
            Err(e) => {
                eprintln!("[AUDIO] Не удалось инициализировать аудио: {}", e);
            }
        }
    }
}
