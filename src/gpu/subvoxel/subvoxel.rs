// ============================================
// SubVoxel System - Система ку-вокселей
// ============================================
// Позволяет размещать блоки меньшего размера (1/2, 1/4 от обычного)

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::gpu::blocks::{BlockType, AIR};

/// Уровень детализации суб-вокселя
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubVoxelLevel {
    /// Обычный блок 1x1x1
    Full = 0,
    /// Половинный блок 1/2 (8 в одном полном)
    Half = 1,
    /// Четвертинный блок 1/4 (64 в одном полном)
    Quarter = 2,
}

impl SubVoxelLevel {
    /// Размер блока на этом уровне
    pub fn size(&self) -> f32 {
        match self {
            SubVoxelLevel::Full => 1.0,
            SubVoxelLevel::Half => 0.5,
            SubVoxelLevel::Quarter => 0.25,
        }
    }
    
    /// Количество делений на ось
    pub fn divisions(&self) -> u8 {
        match self {
            SubVoxelLevel::Full => 1,
            SubVoxelLevel::Half => 2,
            SubVoxelLevel::Quarter => 4,
        }
    }
    
    /// Следующий уровень (меньше)
    pub fn next(&self) -> Self {
        match self {
            SubVoxelLevel::Full => SubVoxelLevel::Half,
            SubVoxelLevel::Half => SubVoxelLevel::Quarter,
            SubVoxelLevel::Quarter => SubVoxelLevel::Full, // Цикл обратно
        }
    }
    
    /// Название уровня
    pub fn name(&self) -> &'static str {
        match self {
            SubVoxelLevel::Full => "1x1x1",
            SubVoxelLevel::Half => "1/2",
            SubVoxelLevel::Quarter => "1/4",
        }
    }
}

impl Default for SubVoxelLevel {
    fn default() -> Self {
        SubVoxelLevel::Full
    }
}

/// Позиция суб-вокселя в мире
/// Для Half: sub_x/y/z = 0 или 1
/// Для Quarter: sub_x/y/z = 0, 1, 2 или 3
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubVoxelPos {
    /// Позиция базового блока
    pub block_x: i32,
    pub block_y: i32,
    pub block_z: i32,
    /// Позиция внутри блока (зависит от уровня)
    pub sub_x: u8,
    pub sub_y: u8,
    pub sub_z: u8,
    /// Уровень детализации
    pub level: SubVoxelLevel,
}

impl SubVoxelPos {
    pub fn new(block_x: i32, block_y: i32, block_z: i32, sub_x: u8, sub_y: u8, sub_z: u8, level: SubVoxelLevel) -> Self {
        Self { block_x, block_y, block_z, sub_x, sub_y, sub_z, level }
    }
    
    /// Создать позицию для полного блока
    pub fn full(x: i32, y: i32, z: i32) -> Self {
        Self::new(x, y, z, 0, 0, 0, SubVoxelLevel::Full)
    }
    
    /// Мировые координаты центра суб-вокселя
    pub fn world_center(&self) -> [f32; 3] {
        let size = self.level.size();
        let half = size / 2.0;
        [
            self.block_x as f32 + self.sub_x as f32 * size + half,
            self.block_y as f32 + self.sub_y as f32 * size + half,
            self.block_z as f32 + self.sub_z as f32 * size + half,
        ]
    }
    
    /// Мировые координаты минимального угла суб-вокселя
    pub fn world_min(&self) -> [f32; 3] {
        let size = self.level.size();
        [
            self.block_x as f32 + self.sub_x as f32 * size,
            self.block_y as f32 + self.sub_y as f32 * size,
            self.block_z as f32 + self.sub_z as f32 * size,
        ]
    }
}

/// Суб-воксель с типом блока
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SubVoxel {
    pub pos: SubVoxelPos,
    pub block_type: BlockType,
}

/// Хранилище суб-вокселей
pub struct SubVoxelStorage {
    /// Суб-воксели по позиции
    subvoxels: HashMap<SubVoxelPos, BlockType>,
    /// Версия для отслеживания изменений
    version: u64,
}

impl SubVoxelStorage {
    pub fn new() -> Self {
        Self {
            subvoxels: HashMap::new(),
            version: 0,
        }
    }
    
    /// Проверить коллизию AABB с любым суб-вокселем
    pub fn check_aabb_collision(&self, min_x: f32, min_y: f32, min_z: f32, max_x: f32, max_y: f32, max_z: f32) -> bool {
        for (pos, block_type) in &self.subvoxels {
            if *block_type == AIR {
                continue;
            }
            
            let size = pos.level.size();
            let [sv_min_x, sv_min_y, sv_min_z] = pos.world_min();
            let sv_max_x = sv_min_x + size;
            let sv_max_y = sv_min_y + size;
            let sv_max_z = sv_min_z + size;
            
            // AABB intersection test
            if max_x > sv_min_x && min_x < sv_max_x &&
               max_y > sv_min_y && min_y < sv_max_y &&
               max_z > sv_min_z && min_z < sv_max_z {
                return true;
            }
        }
        false
    }
    
    /// Добавить суб-воксель
    pub fn set(&mut self, pos: SubVoxelPos, block_type: BlockType) {
        if block_type == AIR {
            self.subvoxels.remove(&pos);
        } else {
            self.subvoxels.insert(pos, block_type);
        }
        self.version += 1;
    }
    
    /// Получить суб-воксель
    pub fn get(&self, pos: &SubVoxelPos) -> Option<BlockType> {
        self.subvoxels.get(pos).copied()
    }
    
    /// Удалить суб-воксель
    pub fn remove(&mut self, pos: &SubVoxelPos) -> Option<BlockType> {
        self.version += 1;
        self.subvoxels.remove(pos)
    }
    
    /// Версия хранилища
    pub fn version(&self) -> u64 {
        self.version
    }
    
    /// Количество суб-вокселей
    pub fn count(&self) -> usize {
        self.subvoxels.len()
    }
    
    /// Получить все суб-воксели для сериализации
    pub fn get_all(&self) -> Vec<SubVoxel> {
        self.subvoxels.iter()
            .map(|(pos, block_type)| SubVoxel { pos: *pos, block_type: *block_type })
            .collect()
    }
    
    /// Загрузить суб-воксели
    pub fn load(&mut self, subvoxels: Vec<SubVoxel>) {
        self.subvoxels.clear();
        for sv in subvoxels {
            self.subvoxels.insert(sv.pos, sv.block_type);
        }
        self.version += 1;
    }
    
    /// Получить суб-воксели в области (для рендеринга)
    pub fn get_in_region(&self, min_x: i32, min_y: i32, min_z: i32, max_x: i32, max_y: i32, max_z: i32) -> Vec<SubVoxel> {
        self.subvoxels.iter()
            .filter(|(pos, _)| {
                pos.block_x >= min_x && pos.block_x <= max_x &&
                pos.block_y >= min_y && pos.block_y <= max_y &&
                pos.block_z >= min_z && pos.block_z <= max_z
            })
            .map(|(pos, block_type)| SubVoxel { pos: *pos, block_type: *block_type })
            .collect()
    }
}

impl Default for SubVoxelStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Вычислить позицию суб-вокселя из мировых координат
pub fn world_to_subvoxel(world_x: f32, world_y: f32, world_z: f32, level: SubVoxelLevel) -> SubVoxelPos {
    let size = level.size();
    let divisions = level.divisions() as f32;
    
    // Базовый блок
    let block_x = world_x.floor() as i32;
    let block_y = world_y.floor() as i32;
    let block_z = world_z.floor() as i32;
    
    // Позиция внутри блока
    let local_x = world_x - block_x as f32;
    let local_y = world_y - block_y as f32;
    let local_z = world_z - block_z as f32;
    
    // Индекс суб-вокселя
    let sub_x = ((local_x / size).floor() as u8).min(level.divisions() - 1);
    let sub_y = ((local_y / size).floor() as u8).min(level.divisions() - 1);
    let sub_z = ((local_z / size).floor() as u8).min(level.divisions() - 1);
    
    SubVoxelPos::new(block_x, block_y, block_z, sub_x, sub_y, sub_z, level)
}

/// Проверка пересечения суб-вокселя с AABB игрока
pub fn subvoxel_intersects_player(
    pos: &SubVoxelPos,
    player_x: f32, player_y: f32, player_z: f32,
    player_radius: f32, player_height: f32
) -> bool {
    let size = pos.level.size();
    let [sv_x, sv_y, sv_z] = pos.world_min();
    
    // AABB суб-вокселя
    let sv_max_x = sv_x + size;
    let sv_max_y = sv_y + size;
    let sv_max_z = sv_z + size;
    
    // AABB игрока
    let p_min_x = player_x - player_radius;
    let p_max_x = player_x + player_radius;
    let p_min_y = player_y;
    let p_max_y = player_y + player_height;
    let p_min_z = player_z - player_radius;
    let p_max_z = player_z + player_radius;
    
    // Проверка пересечения
    p_max_x > sv_x && p_min_x < sv_max_x &&
    p_max_y > sv_y && p_min_y < sv_max_y &&
    p_max_z > sv_z && p_min_z < sv_max_z
}

/// Результат raycast по суб-вокселям
#[derive(Clone, Copy, Debug)]
pub struct SubVoxelHit {
    pub pos: SubVoxelPos,
    pub block_type: BlockType,
    pub hit_point: [f32; 3],
    pub hit_normal: [f32; 3],
    pub distance: f32,
}

impl SubVoxelStorage {
    /// Raycast через суб-воксели
    /// Возвращает ближайший суб-воксель на пути луча
    pub fn raycast(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_distance: f32,
        level: SubVoxelLevel,
    ) -> Option<SubVoxelHit> {
        let size = level.size();
        let mut closest_hit: Option<SubVoxelHit> = None;
        
        // Проверяем все суб-воксели (можно оптимизировать с spatial hash)
        for (pos, &block_type) in &self.subvoxels {
            // Пропускаем суб-воксели другого уровня
            if pos.level != level {
                continue;
            }
            
            let [min_x, min_y, min_z] = pos.world_min();
            let max_x = min_x + size;
            let max_y = min_y + size;
            let max_z = min_z + size;
            
            // Ray-AABB intersection
            if let Some((t, normal)) = ray_aabb_intersection(
                origin, direction,
                [min_x, min_y, min_z],
                [max_x, max_y, max_z],
            ) {
                if t > 0.0 && t < max_distance {
                    if closest_hit.is_none() || t < closest_hit.as_ref().unwrap().distance {
                        closest_hit = Some(SubVoxelHit {
                            pos: *pos,
                            block_type,
                            hit_point: [
                                origin[0] + direction[0] * t,
                                origin[1] + direction[1] * t,
                                origin[2] + direction[2] * t,
                            ],
                            hit_normal: normal,
                            distance: t,
                        });
                    }
                }
            }
        }
        
        closest_hit
    }
}

/// Ray-AABB intersection test
/// Returns (t, normal) where t is distance along ray and normal is hit face normal
fn ray_aabb_intersection(
    origin: [f32; 3],
    direction: [f32; 3],
    aabb_min: [f32; 3],
    aabb_max: [f32; 3],
) -> Option<(f32, [f32; 3])> {
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;
    let mut normal = [0.0f32; 3];
    
    for i in 0..3 {
        if direction[i].abs() < 1e-8 {
            // Ray is parallel to slab
            if origin[i] < aabb_min[i] || origin[i] > aabb_max[i] {
                return None;
            }
        } else {
            let inv_d = 1.0 / direction[i];
            let mut t1 = (aabb_min[i] - origin[i]) * inv_d;
            let mut t2 = (aabb_max[i] - origin[i]) * inv_d;
            
            let mut n = [0.0f32; 3];
            n[i] = -1.0;
            
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
                n[i] = 1.0;
            }
            
            if t1 > t_min {
                t_min = t1;
                normal = n;
            }
            t_max = t_max.min(t2);
            
            if t_min > t_max {
                return None;
            }
        }
    }
    
    Some((t_min, normal))
}

/// Вычислить позицию для размещения суб-вокселя рядом с hit
pub fn placement_pos_from_hit(hit: &SubVoxelHit, level: SubVoxelLevel) -> SubVoxelPos {
    let size = level.size();
    // Смещаем точку попадания немного в направлении нормали
    let place_x = hit.hit_point[0] + hit.hit_normal[0] * (size * 0.5);
    let place_y = hit.hit_point[1] + hit.hit_normal[1] * (size * 0.5);
    let place_z = hit.hit_point[2] + hit.hit_normal[2] * (size * 0.5);
    
    world_to_subvoxel(place_x, place_y, place_z, level)
}
