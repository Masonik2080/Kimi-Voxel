// ============================================
// Save System - Сохранение и загрузка мира
// ============================================

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::gpu::core::{GameResources, SAVE_FILE, DEFAULT_SEED};
use crate::gpu::save::WorldFile;
use crate::gpu::terrain::{WorldChanges, BlockPos};
use crate::gpu::blocks::BlockType;
use crate::gpu::subvoxel::{SubVoxelStorage, SubVoxel};
use crate::gpu::terrain::get_height;

/// Система сохранения/загрузки
pub struct SaveSystem;

/// Данные загруженного мира
pub struct LoadedWorld {
    pub start_x: f32,
    pub start_y: f32,
    pub start_z: f32,
    pub world_seed: u64,
    pub changes: HashMap<BlockPos, BlockType>,
    pub subvoxels: Vec<SubVoxel>,
}

impl SaveSystem {
    /// Загрузить мир из файла или создать новый
    pub fn load_or_create() -> LoadedWorld {
        if let Ok(loaded) = WorldFile::load(SAVE_FILE) {
            println!("[SAVE] Загружен мир из {}", SAVE_FILE);
            println!("[SAVE] Seed: {}, Позиция: {:?}, Изменений: {}, Суб-вокселей: {}", 
                loaded.seed, loaded.player_pos, loaded.changes.len(), loaded.subvoxels.len());
            
            LoadedWorld {
                start_x: loaded.player_pos[0],
                start_y: loaded.player_pos[1],
                start_z: loaded.player_pos[2],
                world_seed: loaded.seed,
                changes: loaded.changes,
                subvoxels: loaded.subvoxels,
            }
        } else {
            // Новый мир
            let start_x = 0.0;
            let start_z = 0.0;
            let start_y = get_height(start_x, start_z) + 2.0;
            println!("[SAVE] Новый мир (seed: {})", DEFAULT_SEED);
            
            LoadedWorld {
                start_x,
                start_y,
                start_z,
                world_seed: DEFAULT_SEED,
                changes: HashMap::new(),
                subvoxels: Vec::new(),
            }
        }
    }
    
    /// Сохранить мир в файл
    pub fn save_world(resources: &GameResources) {
        let player_pos = [
            resources.player.position.x,
            resources.player.position.y,
            resources.player.position.z,
        ];
        
        let changes = resources.world_changes.read().unwrap();
        let subvoxels = resources.subvoxel_storage.read().unwrap();
        
        match WorldFile::save(SAVE_FILE, resources.world_seed, player_pos, &changes, &subvoxels) {
            Ok(_) => {
                println!("[SAVE] Мир сохранён в {} ({} изменений, {} суб-вокселей)", 
                    SAVE_FILE, changes.change_count(), subvoxels.count());
            }
            Err(e) => {
                eprintln!("[SAVE] Ошибка сохранения: {:?}", e);
            }
        }
    }
    
    /// Применить загруженные изменения к миру
    pub fn apply_loaded_changes(
        world_changes: &Arc<RwLock<WorldChanges>>,
        loaded_changes: HashMap<BlockPos, BlockType>,
    ) {
        if !loaded_changes.is_empty() {
            let mut changes = world_changes.write().unwrap();
            for (pos, block) in loaded_changes {
                changes.set_block(pos, block);
            }
        }
    }
    
    /// Применить загруженные суб-воксели
    pub fn apply_loaded_subvoxels(
        subvoxel_storage: &mut SubVoxelStorage,
        loaded_subvoxels: Vec<SubVoxel>,
    ) {
        if !loaded_subvoxels.is_empty() {
            subvoxel_storage.load(loaded_subvoxels);
        }
    }
}
