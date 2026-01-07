// ============================================
// Update System - Обновление игровой логики
// ============================================

use crate::gpu::core::GameResources;

/// Система обновления игровой логики
pub struct UpdateSystem;

impl UpdateSystem {
    /// Основной цикл обновления
    pub fn update(resources: &mut GameResources, dt: f32, _time: f32) {
        // 1. Обновляем игрока (физика, движение)
        Self::update_player(resources, dt);
        
        // 2. Обновляем камеру
        resources.camera.update_from_player(&resources.player);
        
        // 3. Обновляем аудио
        Self::update_audio(resources, dt);
        
        // 4. Обновляем систему ломания блоков
        resources.block_breaker.update(&resources.player, dt);
    }
    
    /// Обновление игрока
    fn update_player(resources: &mut GameResources, dt: f32) {
        let changes = resources.world_changes.read().unwrap();
        let changes_map = changes.get_all_changes_copy();
        drop(changes);
        resources.player_controller.update(&mut resources.player, dt, &changes_map);
    }
    
    /// Обновление аудио системы
    fn update_audio(resources: &mut GameResources, dt: f32) {
        if let Some(audio) = &mut resources.audio_system {
            let is_moving = resources.player_controller.forward 
                || resources.player_controller.backward 
                || resources.player_controller.left 
                || resources.player_controller.right;
            
            audio.update(
                resources.player.eye_position(),
                resources.player.forward(),
                resources.player.velocity.y,
                is_moving,
                resources.player.on_ground,
                resources.player.is_sprinting,
                resources.player_controller.jump,
                dt,
            );
        }
    }
}
