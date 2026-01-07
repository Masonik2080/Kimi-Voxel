// ============================================
// Procedural Foliage - Процедурная листва
// ============================================
// Генерирует субвоксели листвы на лету для красивого вида деревьев

use std::collections::HashSet;
use crate::gpu::blocks::{BlockType, OAK_LEAVES, BIRCH_LEAVES, SPRUCE_LEAVES};
use crate::gpu::terrain::generation::hash3d;
use crate::gpu::subvoxel::{SubVoxelPos, SubVoxelLevel, SubVoxelStorage};
use crate::gpu::biomes::{biome_selector, BIOME_TAIGA, BIOME_TUNDRA, BIOME_FOREST};
use crate::gpu::terrain::voxel::CHUNK_SIZE;

/// Проверяет, является ли блок листвой
#[inline]
pub fn is_leaf_block(block: BlockType) -> bool {
    matches!(block, OAK_LEAVES | BIRCH_LEAVES | SPRUCE_LEAVES)
}

/// Кэш сгенерированных деревьев
pub struct FoliageCache {
    /// Чанки для которых уже сгенерированы деревья
    generated_chunks: HashSet<(i32, i32)>,
    /// Последняя позиция игрока (для очистки)
    last_player_chunk: (i32, i32),
}

impl FoliageCache {
    pub fn new() -> Self {
        Self {
            generated_chunks: HashSet::new(),
            last_player_chunk: (0, 0),
        }
    }
    
    /// Обновить листву вокруг игрока
    /// Генерирует субвоксели для деревьев в радиусе видимости
    pub fn update(
        &mut self,
        storage: &mut SubVoxelStorage,
        player_x: f32,
        player_z: f32,
        _render_distance: i32,
    ) {
        let player_cx = (player_x / CHUNK_SIZE as f32).floor() as i32;
        let player_cz = (player_z / CHUNK_SIZE as f32).floor() as i32;
        
        // Лимит субвокселей
        if storage.count() > 2_000_000 {
            return;
        }
        
        // Проверяем нужно ли что-то делать
        let mut any_new = false;
        
        // 8 чанков вокруг игрока
        let limited_distance = 8;
        for dz in -limited_distance..=limited_distance {
            for dx in -limited_distance..=limited_distance {
                let cx = player_cx + dx;
                let cz = player_cz + dz;
                
                if !self.generated_chunks.contains(&(cx, cz)) {
                    self.generate_chunk_foliage(storage, cx, cz);
                    self.generated_chunks.insert((cx, cz));
                    any_new = true;
                }
            }
        }
        
        // Очистка далеких чанков только если игрок переместился
        if (player_cx, player_cz) != self.last_player_chunk {
            // Очищаем только если есть что очищать
            let old_count = self.generated_chunks.len();
            self.cleanup(player_cx, player_cz, limited_distance + 1);
            
            if self.generated_chunks.len() < old_count {
                self.cleanup_subvoxels(storage, player_cx, player_cz, limited_distance + 1);
            }
            
            self.last_player_chunk = (player_cx, player_cz);
        }
    }
    
    /// Генерация листвы для одного чанка
    fn generate_chunk_foliage(&self, storage: &mut SubVoxelStorage, chunk_x: i32, chunk_z: i32) {
        let base_x = chunk_x * CHUNK_SIZE;
        let base_z = chunk_z * CHUNK_SIZE;
        
        // Проходим по чанку и ищем места для деревьев
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let world_x = base_x + lx;
                let world_z = base_z + lz;
                
                // Получаем высоту terrain
                let terrain_height = crate::gpu::terrain::generation::get_height(world_x as f32, world_z as f32) as i32;
                
                let biome = biome_selector().get_biome_def(world_x, world_z);
                
                if biome.tree_density > 0.0001 {
                    let rng = hash3d(world_x, terrain_height, world_z);
                    
                    if rng < biome.tree_density {
                        let tree_height = 5 + ((rng * 1000.0) as i32 % 3);
                        let base_y = terrain_height + 1;
                        
                        // Определяем тип листвы
                        let leaf_type = match biome.id {
                            BIOME_TAIGA | BIOME_TUNDRA => SPRUCE_LEAVES,
                            BIOME_FOREST => {
                                if ((rng * 100.0) as i32) % 5 == 0 {
                                    BIRCH_LEAVES
                                } else {
                                    OAK_LEAVES
                                }
                            },
                            _ => OAK_LEAVES,
                        };
                        
                        // Генерируем листву как субвоксели
                        self.generate_tree_foliage(storage, world_x, base_y, world_z, tree_height, leaf_type, biome.id);
                    }
                }
            }
        }
    }
    
    /// Генерация субвокселей листвы для одного дерева
    fn generate_tree_foliage(
        &self,
        storage: &mut SubVoxelStorage,
        tree_x: i32,
        base_y: i32,
        tree_z: i32,
        height: i32,
        leaf_type: BlockType,
        biome_id: u8,
    ) {
        let is_spruce = matches!(biome_id, BIOME_TAIGA | BIOME_TUNDRA);
        
        if is_spruce {
            self.generate_spruce_foliage(storage, tree_x, base_y, tree_z, height, leaf_type);
        } else {
            self.generate_basic_foliage(storage, tree_x, base_y, tree_z, height, leaf_type);
        }
    }
    
    /// Листва для дуба/березы
    fn generate_basic_foliage(
        &self,
        storage: &mut SubVoxelStorage,
        tree_x: i32,
        base_y: i32,
        tree_z: i32,
        height: i32,
        leaf_type: BlockType,
    ) {
        let top_y = base_y + height;
        
        for y in (top_y - 3)..=(top_y + 1) {
            let y_offset = y - top_y;
            let radius: i32 = if y_offset >= 0 { 1 } else { 2 };
            
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    // Скругляем углы
                    if dx.abs() == radius && dz.abs() == radius && y_offset < 0 {
                        continue;
                    }
                    
                    let world_x = tree_x + dx;
                    let world_z = tree_z + dz;
                    
                    // Генерируем субвоксели для этого блока листвы
                    self.generate_leaf_subvoxels(storage, world_x, y, world_z, leaf_type);
                }
            }
        }
    }
    
    /// Листва для ели (конусом)
    fn generate_spruce_foliage(
        &self,
        storage: &mut SubVoxelStorage,
        tree_x: i32,
        base_y: i32,
        tree_z: i32,
        height: i32,
        leaf_type: BlockType,
    ) {
        let top_y = base_y + height;
        
        // Верхушка
        self.generate_leaf_subvoxels(storage, tree_x, top_y, tree_z, leaf_type);
        self.generate_leaf_subvoxels(storage, tree_x, top_y + 1, tree_z, leaf_type);
        
        // Слои вниз
        for y in (base_y + 2..top_y).rev() {
            let layer = (top_y - y) % 4;
            let radius = if layer == 1 || layer == 3 { 2 } else { 1 };
            
            if radius == 1 {
                self.generate_leaf_subvoxels(storage, tree_x + 1, y, tree_z, leaf_type);
                self.generate_leaf_subvoxels(storage, tree_x - 1, y, tree_z, leaf_type);
                self.generate_leaf_subvoxels(storage, tree_x, y, tree_z + 1, leaf_type);
                self.generate_leaf_subvoxels(storage, tree_x, y, tree_z - 1, leaf_type);
            } else {
                for dx in -1i32..=1 {
                    for dz in -1i32..=1 {
                        if dx.abs() + dz.abs() <= 2 {
                            self.generate_leaf_subvoxels(storage, tree_x + dx, y, tree_z + dz, leaf_type);
                        }
                    }
                }
            }
        }
    }
    
    /// Генерирует субвоксели для одного блока листвы
    fn generate_leaf_subvoxels(
        &self,
        storage: &mut SubVoxelStorage,
        world_x: i32,
        world_y: i32,
        world_z: i32,
        leaf_type: BlockType,
    ) {
        // Quarter level (4x4x4 = 64 субвокселей) для детальной листвы
        let level = SubVoxelLevel::Quarter;
        
        for sy in 0..4u8 {
            for sz in 0..4u8 {
                for sx in 0..4u8 {
                    let noise = hash3d(
                        world_x * 4 + sx as i32,
                        world_y * 4 + sy as i32,
                        world_z * 4 + sz as i32
                    );
                    
                    // ~40% заполнение для воздушной листвы
                    if noise < 0.4 {
                        let pos = SubVoxelPos::new(world_x, world_y, world_z, sx, sy, sz, level);
                        storage.set(pos, leaf_type);
                    }
                }
            }
        }
    }
    
    /// Очистка далеких чанков
    fn cleanup(&mut self, center_x: i32, center_z: i32, max_distance: i32) {
        self.generated_chunks.retain(|(cx, cz)| {
            (cx - center_x).abs() <= max_distance && (cz - center_z).abs() <= max_distance
        });
    }
    
    /// Очистка субвокселей далеких чанков
    fn cleanup_subvoxels(&self, storage: &mut SubVoxelStorage, center_x: i32, center_z: i32, max_distance: i32) {
        let min_x = (center_x - max_distance) * CHUNK_SIZE;
        let max_x = (center_x + max_distance + 1) * CHUNK_SIZE;
        let min_z = (center_z - max_distance) * CHUNK_SIZE;
        let max_z = (center_z + max_distance + 1) * CHUNK_SIZE;
        
        // Получаем все субвоксели и удаляем далекие
        let all = storage.get_all();
        for sv in all {
            let x = sv.pos.block_x;
            let z = sv.pos.block_z;
            if x < min_x || x >= max_x || z < min_z || z >= max_z {
                storage.remove(&sv.pos);
            }
        }
    }
}

impl Default for FoliageCache {
    fn default() -> Self {
        Self::new()
    }
}
