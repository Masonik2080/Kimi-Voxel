// ============================================
// Block Interaction System - Ломание и установка блоков
// ============================================

use crate::gpu::core::GameResources;
use crate::gpu::blocks::MouseButton;
use crate::gpu::terrain::BlockPos;
use crate::gpu::subvoxel::{SubVoxelLevel, SubVoxelHit, world_to_subvoxel, subvoxel_intersects_player, placement_pos_from_hit};
use crate::gpu::player::{PLAYER_HEIGHT, PLAYER_RADIUS};
use crate::gpu::blocks::BlockType;

/// Система взаимодействия с блоками
pub struct BlockInteractionSystem;

impl BlockInteractionSystem {
    /// Обработка левой кнопки мыши (ломание)
    pub fn handle_break(resources: &mut GameResources) {
        let eye_pos = resources.player.eye_position();
        let forward = resources.player.forward();
        let origin = [eye_pos.x, eye_pos.y, eye_pos.z];
        let direction = [forward.x, forward.y, forward.z];
        
        // Ищем ближайший суб-воксель
        let mut closest_subvoxel: Option<(SubVoxelHit, f32)> = None;
        {
            let subvoxels = resources.subvoxel_storage.read().unwrap();
            for level in [SubVoxelLevel::Quarter, SubVoxelLevel::Half] {
                if let Some(hit) = subvoxels.raycast(origin, direction, 5.0, level) {
                    if closest_subvoxel.is_none() || hit.distance < closest_subvoxel.as_ref().unwrap().1 {
                        closest_subvoxel = Some((hit, hit.distance));
                    }
                }
            }
        }
        
        // Проверяем обычный блок
        let block_dist = resources.block_breaker.target_block()
            .map(|b| b.distance)
            .unwrap_or(f32::MAX);
        
        if let Some((hit, dist)) = closest_subvoxel {
            if dist < block_dist {
                // Ломаем суб-воксель
                let mut subvoxels = resources.subvoxel_storage.write().unwrap();
                subvoxels.remove(&hit.pos);
                return;
            }
        }
        
        // Ломаем обычный блок
        if let Some(broken) = resources.block_breaker.process_mouse_button(MouseButton::Left, true) {
            if let Some(renderer) = &mut resources.renderer {
                let changes = resources.world_changes.read().unwrap();
                renderer.instant_chunk_update(
                    broken.block_pos[0],
                    broken.block_pos[1],
                    broken.block_pos[2],
                    &changes,
                );
            }
        }
    }
    
    /// Обработка правой кнопки мыши (установка)
    pub fn handle_place(resources: &mut GameResources) {
        // Получаем тип блока из хотбара
        let block_type = if let Some(gui) = &mut resources.gui_renderer {
            gui.hotbar().selected_block_type()
        } else {
            None
        };
        
        let Some(block_type) = block_type else { return };
        
        if resources.current_subvoxel_level == SubVoxelLevel::Full {
            Self::place_full_block(resources, block_type);
        } else {
            Self::place_subvoxel(resources, block_type);
        }
    }
    
    /// Установка полного блока
    fn place_full_block(resources: &mut GameResources, block_type: BlockType) {
        if let Some(place_pos) = resources.block_breaker.placement_pos() {
            if !Self::block_intersects_player(resources, place_pos) {
                // Ставим блок
                let mut changes = resources.world_changes.write().unwrap();
                changes.set_block(
                    BlockPos::new(place_pos[0], place_pos[1], place_pos[2]),
                    block_type,
                );
                drop(changes);
                
                if let Some(renderer) = &mut resources.renderer {
                    let changes = resources.world_changes.read().unwrap();
                    renderer.instant_chunk_update(
                        place_pos[0],
                        place_pos[1],
                        place_pos[2],
                        &changes,
                    );
                }
                
                // Звук установки блока
                if let Some(audio) = &mut resources.audio_system {
                    audio.play_place_block();
                }
            }
        }
    }
    
    /// Установка суб-вокселя
    fn place_subvoxel(resources: &mut GameResources, block_type: BlockType) {
        let eye_pos = resources.player.eye_position();
        let forward = resources.player.forward();
        let origin = [eye_pos.x, eye_pos.y, eye_pos.z];
        let direction = [forward.x, forward.y, forward.z];
        
        // Ищем ближайший суб-воксель любого уровня
        let mut closest_hit: Option<SubVoxelHit> = None;
        {
            let subvoxels = resources.subvoxel_storage.read().unwrap();
            for level in [SubVoxelLevel::Quarter, SubVoxelLevel::Half] {
                if let Some(hit) = subvoxels.raycast(origin, direction, 5.0, level) {
                    if closest_hit.is_none() || hit.distance < closest_hit.as_ref().unwrap().distance {
                        closest_hit = Some(hit);
                    }
                }
            }
        }
        
        // Также проверяем обычный блок
        let block_dist = resources.block_breaker.target_block()
            .map(|b| b.distance)
            .unwrap_or(f32::MAX);
        
        let subvoxel_pos = if let Some(hit) = closest_hit {
            if hit.distance < block_dist {
                // Ставим рядом с существующим суб-вокселем
                Some(placement_pos_from_hit(&hit, resources.current_subvoxel_level))
            } else if let Some(hit_pos) = resources.block_breaker.placement_world_pos() {
                // Ставим на обычный блок (он ближе)
                Some(world_to_subvoxel(
                    hit_pos[0], hit_pos[1], hit_pos[2],
                    resources.current_subvoxel_level
                ))
            } else {
                None
            }
        } else if let Some(hit_pos) = resources.block_breaker.placement_world_pos() {
            // Нет суб-вокселей, ставим на обычный блок
            Some(world_to_subvoxel(
                hit_pos[0], hit_pos[1], hit_pos[2],
                resources.current_subvoxel_level
            ))
        } else {
            None
        };
        
        if let Some(subvoxel_pos) = subvoxel_pos {
            let mut subvoxels = resources.subvoxel_storage.write().unwrap();
            // Проверяем что позиция не занята
            if subvoxels.get(&subvoxel_pos).is_none() {
                // Проверяем коллизию с игроком
                if !subvoxel_intersects_player(
                    &subvoxel_pos,
                    resources.player.position.x,
                    resources.player.position.y,
                    resources.player.position.z,
                    PLAYER_RADIUS,
                    PLAYER_HEIGHT
                ) {
                    subvoxels.set(subvoxel_pos, block_type);
                    drop(subvoxels);
                    
                    // Звук установки блока
                    if let Some(audio) = &mut resources.audio_system {
                        audio.play_place_block();
                    }
                }
            }
        }
    }
    
    /// Обработка средней кнопки мыши (pick block)
    pub fn handle_pick_block(resources: &mut GameResources) {
        if let Some(target) = resources.block_breaker.target_block() {
            let block_type = target.block_type;
            if let Some(gui) = &mut resources.gui_renderer {
                gui.hotbar().pick_block(block_type);
            }
        }
    }
    
    /// Проверяет, пересекается ли блок с хитбоксом игрока
    fn block_intersects_player(resources: &GameResources, block_pos: [i32; 3]) -> bool {
        let player_pos = resources.player.position;
        
        // Границы хитбокса игрока (AABB)
        let player_min_x = player_pos.x - PLAYER_RADIUS;
        let player_max_x = player_pos.x + PLAYER_RADIUS;
        let player_min_y = player_pos.y;
        let player_max_y = player_pos.y + PLAYER_HEIGHT;
        let player_min_z = player_pos.z - PLAYER_RADIUS;
        let player_max_z = player_pos.z + PLAYER_RADIUS;
        
        // Границы блока
        let block_min_x = block_pos[0] as f32;
        let block_max_x = block_pos[0] as f32 + 1.0;
        let block_min_y = block_pos[1] as f32;
        let block_max_y = block_pos[1] as f32 + 1.0;
        let block_min_z = block_pos[2] as f32;
        let block_max_z = block_pos[2] as f32 + 1.0;
        
        // Проверка пересечения AABB
        player_max_x > block_min_x && player_min_x < block_max_x &&
        player_max_y > block_min_y && player_min_y < block_max_y &&
        player_max_z > block_min_z && player_min_z < block_max_z
    }
}
