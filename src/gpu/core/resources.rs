// ============================================
// Resources - Общие ресурсы игры
// ============================================

use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;
use winit::window::Window;

use crate::gpu::player::Camera;
use crate::gpu::player::{Player, PlayerController};
use crate::gpu::render::Renderer;
use crate::gpu::blocks::BlockBreaker;
use crate::gpu::terrain::WorldChanges;
use crate::gpu::gui::{GameMenu, GuiRenderer};
use crate::gpu::subvoxel::{SubVoxelStorage, SubVoxelLevel};
use crate::gpu::subvoxel::SubVoxelRenderer;
use crate::gpu::audio::AudioSystem;
use crate::gpu::biomes::FoliageCache;

/// Все игровые ресурсы в одном месте
pub struct GameResources {
    // Window & Rendering
    pub window: Option<Arc<Window>>,
    pub renderer: Option<Renderer>,
    pub gui_renderer: Option<GuiRenderer>,
    pub subvoxel_renderer: Option<SubVoxelRenderer>,
    
    // Player entity
    pub player: Player,
    pub player_controller: PlayerController,
    
    // Camera
    pub camera: Camera,
    
    // Block interaction
    pub block_breaker: BlockBreaker,
    
    // World data
    pub world_changes: Arc<RwLock<WorldChanges>>,
    pub subvoxel_storage: Arc<RwLock<SubVoxelStorage>>,
    pub current_subvoxel_level: SubVoxelLevel,
    pub world_seed: u64,
    pub foliage_cache: FoliageCache,
    
    // GUI
    pub menu: GameMenu,
    
    // Audio
    pub audio_system: Option<AudioSystem>,
    
    // Timing
    pub start_time: Instant,
    pub last_frame: Instant,
    
    // Input state
    pub cursor_grabbed: bool,
    pub mouse_pos: (f32, f32),
    pub menu_mouse_pressed: bool,
}
