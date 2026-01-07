// ============================================
// Block Breaker - Логика ломания блоков
// ============================================
// Обрабатывает:
// - Raycast от камеры к блоку
// - Прогресс ломания (анимация трещин)
// - Удаление блока из мира
// - Выделение целевого блока

use ultraviolet::Vec3;
use std::sync::Arc;
use std::sync::RwLock;
use crate::gpu::blocks::BlockType;
use crate::gpu::player::Player;
use crate::gpu::terrain::get_height;
use crate::gpu::terrain::WorldChanges;

/// Максимальная дистанция ломания блоков
pub const MAX_BREAK_DISTANCE: f32 = 5.0;

/// Скорость ломания (базовая, без инструментов)
pub const BASE_BREAK_SPEED: f32 = 1.0;

/// Результат raycast — информация о блоке под прицелом
#[derive(Debug, Clone, Copy)]
pub struct BlockHit {
    /// Позиция блока (целые координаты)
    pub block_pos: [i32; 3],
    
    /// Точка попадания луча (мировые координаты)
    pub hit_point: Vec3,
    
    /// Нормаль грани, в которую попал луч
    pub hit_normal: Vec3,
    
    /// Дистанция от глаз до точки попадания
    pub distance: f32,
    
    /// Тип блока
    pub block_type: BlockType,
}

/// Состояние процесса ломания
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BreakState {
    /// Не ломаем
    Idle,
    /// В процессе ломания
    Breaking {
        /// Позиция ломаемого блока
        block_pos: [i32; 3],
        /// Прогресс (0.0 - 1.0)
        progress: f32,
    },
    /// Блок сломан (на один кадр)
    Broken {
        block_pos: [i32; 3],
        block_type: BlockType,
    },
}

/// Система ломания блоков
pub struct BlockBreaker {
    /// Текущее состояние
    state: BreakState,
    
    /// Блок под прицелом (если есть)
    target_block: Option<BlockHit>,
    
    /// Зажата ли кнопка ломания (ЛКМ)
    is_breaking: bool,
    
    /// Зажата ли кнопка установки (ПКМ)
    is_placing: bool,
    
    /// Максимальная дистанция
    max_distance: f32,
    
    /// Множитель скорости ломания (от инструмента)
    break_speed_multiplier: f32,
    
    /// Ссылка на изменения мира
    world_changes: Arc<RwLock<WorldChanges>>,
}

impl BlockBreaker {
    pub fn new(world_changes: Arc<RwLock<WorldChanges>>) -> Self {
        Self {
            state: BreakState::Idle,
            target_block: None,
            is_breaking: false,
            is_placing: false,
            max_distance: MAX_BREAK_DISTANCE,
            break_speed_multiplier: 1.0,
            world_changes,
        }
    }
    
    /// Обработка нажатия кнопки мыши
    pub fn process_mouse_button(&mut self, button: MouseButton, pressed: bool) -> Option<BlockHit> {
        match button {
            MouseButton::Left => {
                // Мгновенное ломание по клику
                if pressed {
                    if let Some(hit) = &self.target_block {
                        // Сразу ломаем блок
                        let broken_block = *hit;
                        
                        {
                            let mut changes = self.world_changes.write().unwrap();
                            changes.break_block(
                                broken_block.block_pos[0],
                                broken_block.block_pos[1],
                                broken_block.block_pos[2],
                            );
                        }
                        
                        return Some(broken_block);
                    }
                }
            }
            MouseButton::Right => {
                self.is_placing = pressed;
            }
            MouseButton::Middle => {
                // Средняя кнопка — пока не используется (можно для pick block)
            }
        }
        None
    }
    
    /// Установить множитель скорости (от инструмента)
    pub fn set_break_speed(&mut self, multiplier: f32) {
        self.break_speed_multiplier = multiplier;
    }
    
    /// Обновление каждый кадр — только raycast для выделения
    pub fn update(&mut self, player: &Player, _dt: f32) {
        // Raycast для поиска блока под прицелом
        self.target_block = self.raycast_block(player);
    }
    
    /// Raycast от глаз игрока в направлении взгляда
    fn raycast_block(&self, player: &Player) -> Option<BlockHit> {
        let origin = player.eye_position();
        let direction = player.forward();
        
        // DDA (Digital Differential Analyzer) алгоритм для воксельного raycast
        self.dda_raycast(origin, direction, self.max_distance)
    }
    
    /// DDA Raycast через воксельную сетку
    fn dda_raycast(&self, origin: Vec3, direction: Vec3, max_dist: f32) -> Option<BlockHit> {
        // Текущая позиция в блоках
        let mut block_x = origin.x.floor() as i32;
        let mut block_y = origin.y.floor() as i32;
        let mut block_z = origin.z.floor() as i32;
        
        // Направление шага (+1 или -1)
        let step_x = if direction.x >= 0.0 { 1 } else { -1 };
        let step_y = if direction.y >= 0.0 { 1 } else { -1 };
        let step_z = if direction.z >= 0.0 { 1 } else { -1 };
        
        // Дельта t для пересечения одного блока
        let t_delta_x = if direction.x.abs() < 1e-10 { f32::MAX } else { (1.0 / direction.x).abs() };
        let t_delta_y = if direction.y.abs() < 1e-10 { f32::MAX } else { (1.0 / direction.y).abs() };
        let t_delta_z = if direction.z.abs() < 1e-10 { f32::MAX } else { (1.0 / direction.z).abs() };
        
        // Начальные t до первой границы блока
        let mut t_max_x = if direction.x >= 0.0 {
            ((block_x + 1) as f32 - origin.x) / direction.x
        } else {
            (block_x as f32 - origin.x) / direction.x
        };
        let mut t_max_y = if direction.y >= 0.0 {
            ((block_y + 1) as f32 - origin.y) / direction.y
        } else {
            (block_y as f32 - origin.y) / direction.y
        };
        let mut t_max_z = if direction.z >= 0.0 {
            ((block_z + 1) as f32 - origin.z) / direction.z
        } else {
            (block_z as f32 - origin.z) / direction.z
        };
        
        // Нормаль последней пересечённой грани
        let mut hit_normal = Vec3::zero();
        let mut distance = 0.0_f32;
        
        // Итерируем пока не превысим дистанцию
        let max_steps = (max_dist * 2.0) as i32 + 1;
        
        for _ in 0..max_steps {
            // Проверяем текущий блок
            if let Some(block_type) = self.get_block_at(block_x, block_y, block_z) {
                if block_type != super::AIR {
                    // Нашли твёрдый блок!
                    let hit_point = origin + direction * distance;
                    
                    return Some(BlockHit {
                        block_pos: [block_x, block_y, block_z],
                        hit_point,
                        hit_normal,
                        distance,
                        block_type,
                    });
                }
            }
            
            // Переходим к следующему блоку (выбираем ближайшую границу)
            if t_max_x < t_max_y {
                if t_max_x < t_max_z {
                    distance = t_max_x;
                    t_max_x += t_delta_x;
                    block_x += step_x;
                    hit_normal = Vec3::new(-step_x as f32, 0.0, 0.0);
                } else {
                    distance = t_max_z;
                    t_max_z += t_delta_z;
                    block_z += step_z;
                    hit_normal = Vec3::new(0.0, 0.0, -step_z as f32);
                }
            } else {
                if t_max_y < t_max_z {
                    distance = t_max_y;
                    t_max_y += t_delta_y;
                    block_y += step_y;
                    hit_normal = Vec3::new(0.0, -step_y as f32, 0.0);
                } else {
                    distance = t_max_z;
                    t_max_z += t_delta_z;
                    block_z += step_z;
                    hit_normal = Vec3::new(0.0, 0.0, -step_z as f32);
                }
            }
            
            // Проверка дистанции
            if distance > max_dist {
                break;
            }
        }
        
        None
    }
    
    /// Получить тип блока в координатах
    fn get_block_at(&self, x: i32, y: i32, z: i32) -> Option<BlockType> {
        use crate::gpu::terrain::{CaveParams, is_cave};
        use crate::gpu::biomes::biome_selector;
        
        // Сначала проверяем изменения мира
        {
            let changes = self.world_changes.read().unwrap();
            if let Some(block_type) = changes.get_block(x, y, z) {
                return Some(block_type);
            }
        }
        
        // Иначе используем процедурную генерацию с биомами
        let terrain_height = get_height(x as f32, z as f32) as i32;
        
        // Над поверхностью — воздух
        if y > terrain_height {
            return Some(super::AIR);
        }
        
        // Проверяем пещеры
        let cave_params = CaveParams::default();
        let cave_ceiling = terrain_height - cave_params.surface_offset;
        
        if y >= cave_params.min_height && y < cave_ceiling {
            if is_cave(x, y, z, &cave_params) {
                return Some(super::AIR);
            }
        }
        
        // Получаем биом и используем его блоки
        let biome = biome_selector().get_biome_def(x, z);
        
        if y < -29 {
            Some(super::DEEPSLATE)
        } else if y < terrain_height - 4 {
            Some(biome.deep_block)
        } else if y < terrain_height {
            Some(biome.subsurface_block)
        } else {
            Some(biome.surface_block)
        }
    }
    
    // === Getters ===
    
    /// Блок под прицелом
    pub fn target_block(&self) -> Option<&BlockHit> {
        self.target_block.as_ref()
    }
    
    /// Текущее состояние ломания
    pub fn state(&self) -> &BreakState {
        &self.state
    }
    
    /// Прогресс ломания (0.0 - 1.0)
    pub fn break_progress(&self) -> f32 {
        match &self.state {
            BreakState::Breaking { progress, .. } => *progress,
            _ => 0.0,
        }
    }
    
    /// Позиция блока для выделения (если есть)
    pub fn highlight_block_pos(&self) -> Option<[i32; 3]> {
        self.target_block.as_ref().map(|hit| hit.block_pos)
    }
    
    /// Позиция для установки нового блока (рядом с целевым)
    pub fn placement_pos(&self) -> Option<[i32; 3]> {
        self.target_block.as_ref().map(|hit| {
            [
                hit.block_pos[0] + hit.hit_normal.x as i32,
                hit.block_pos[1] + hit.hit_normal.y as i32,
                hit.block_pos[2] + hit.hit_normal.z as i32,
            ]
        })
    }
    
    /// Мировые координаты точки для размещения суб-вокселя
    pub fn placement_world_pos(&self) -> Option<[f32; 3]> {
        self.target_block.as_ref().map(|hit| {
            // Смещаем точку попадания немного в направлении нормали
            let offset = 0.01;
            [
                hit.hit_point.x + hit.hit_normal.x * offset,
                hit.hit_point.y + hit.hit_normal.y * offset,
                hit.hit_point.z + hit.hit_normal.z * offset,
            ]
        })
    }
    
    /// Нужно ли установить блок (ПКМ нажата и есть цель)
    pub fn should_place(&self) -> bool {
        self.is_placing && self.target_block.is_some()
    }
}

/// Кнопки мыши
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

// Убран Default так как требуется world_changes
