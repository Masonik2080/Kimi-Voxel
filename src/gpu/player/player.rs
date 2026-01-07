
// ============================================
// Player Entity - Физическая сущность игрока
// ============================================
// Хранит позицию ног, обрабатывает физику и ввод.
// Является центром мира для генерации чанков.

use ultraviolet::Vec3;
use super::flight::FlightController;

/// Константы игрока
pub const PLAYER_HEIGHT: f32 = 1.8;      // Полная высота игрока
pub const EYE_HEIGHT: f32 = 1.62;        // Высота глаз от ног
pub const PLAYER_RADIUS: f32 = 0.3;      // Радиус хитбокса (половина ширины)
pub const GRAVITY: f32 = 28.0;           // Ускорение свободного падения
pub const JUMP_VELOCITY: f32 = 9.0;      // Начальная скорость прыжка
pub const TERMINAL_VELOCITY: f32 = 50.0; // Максимальная скорость падения

/// Игрок — физическая сущность в мире
pub struct Player {
    /// Позиция ног (нижняя точка хитбокса)
    pub position: Vec3,
    
    /// Скорость (для физики)
    pub velocity: Vec3,
    
    /// Горизонтальный угол поворота тела (yaw)
    pub yaw: f32,
    
    /// Вертикальный угол головы (pitch)
    pub pitch: f32,
    
    /// На земле ли игрок
    pub on_ground: bool,
    
    /// Скорость передвижения
    pub move_speed: f32,
    
    /// Скорость бега (shift)
    pub sprint_speed: f32,
    
    /// Сейчас бежит
    pub is_sprinting: bool,
}

impl Player {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: Vec3::new(x, y, z),
            velocity: Vec3::zero(),
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            move_speed: 5.0,
            sprint_speed: 8.0,
            is_sprinting: false,
        }
    }
    
    /// Позиция глаз (для камеры от первого лица)
    pub fn eye_position(&self) -> Vec3 {
        Vec3::new(
            self.position.x,
            self.position.y + EYE_HEIGHT,
            self.position.z,
        )
    }
    
    /// Центр тела (для рендеринга модели)
    pub fn body_center(&self) -> Vec3 {
        Vec3::new(
            self.position.x,
            self.position.y + PLAYER_HEIGHT * 0.5,
            self.position.z,
        )
    }
    
    /// Направление взгляда (forward vector)
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        ).normalized()
    }
    
    /// Горизонтальное направление движения (без pitch)
    pub fn forward_horizontal(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos(),
            0.0,
            self.yaw.sin(),
        ).normalized()
    }
    
    /// Вектор вправо
    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::unit_y()).normalized()
    }
    
    /// Горизонтальный вектор вправо
    pub fn right_horizontal(&self) -> Vec3 {
        self.forward_horizontal().cross(Vec3::unit_y()).normalized()
    }
}

/// Тип функции проверки твёрдости блока
/// Принимает (x, y, z) и возвращает true если блок твёрдый
pub type BlockSolidChecker = Box<dyn Fn(i32, i32, i32, &std::collections::HashMap<crate::gpu::terrain::BlockPos, crate::gpu::blocks::BlockType>) -> bool + Send + Sync>;

/// Тип функции проверки коллизии с суб-вокселями
/// Принимает AABB игрока (min_x, min_y, min_z, max_x, max_y, max_z) и возвращает true если есть коллизия
pub type SubVoxelCollisionChecker = Box<dyn Fn(f32, f32, f32, f32, f32, f32) -> bool + Send + Sync>;

/// Контроллер игрока — обрабатывает ввод и физику
pub struct PlayerController {
    // Состояние клавиш движения
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sprint: bool,
    
    // Дельта мыши
    mouse_dx: f32,
    mouse_dy: f32,
    
    // Чувствительность мыши
    pub sensitivity: f32,
    
    // Контроллер полёта
    pub flight: FlightController,
    
    // Функция проверки твёрдости блока
    block_solid_checker: Option<BlockSolidChecker>,
    
    // Функция проверки коллизии с суб-вокселями
    subvoxel_collision_checker: Option<SubVoxelCollisionChecker>,
}

impl PlayerController {
    pub fn new(sensitivity: f32) -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            jump: false,
            sprint: false,
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            sensitivity,
            flight: FlightController::new(),
            block_solid_checker: None,
            subvoxel_collision_checker: None,
        }
    }
    
    /// Установить функцию проверки твёрдости блока
    pub fn set_block_solid_checker<F>(&mut self, f: F)
    where
        F: Fn(i32, i32, i32, &std::collections::HashMap<crate::gpu::terrain::BlockPos, crate::gpu::blocks::BlockType>) -> bool + Send + Sync + 'static,
    {
        self.block_solid_checker = Some(Box::new(f));
    }
    
    /// Установить функцию проверки коллизии с суб-вокселями
    pub fn set_subvoxel_collision_checker<F>(&mut self, f: F)
    where
        F: Fn(f32, f32, f32, f32, f32, f32) -> bool + Send + Sync + 'static,
    {
        self.subvoxel_collision_checker = Some(Box::new(f));
    }
    
    /// Проверить твёрдость блока
    fn is_block_solid(&self, x: i32, y: i32, z: i32, world_changes: &std::collections::HashMap<crate::gpu::terrain::BlockPos, crate::gpu::blocks::BlockType>) -> bool {
        if let Some(ref checker) = self.block_solid_checker {
            checker(x, y, z, world_changes)
        } else {
            false
        }
    }
    
    /// Проверить коллизию с суб-вокселями
    fn check_subvoxel_collision(&self, min_x: f32, min_y: f32, min_z: f32, max_x: f32, max_y: f32, max_z: f32) -> bool {
        if let Some(ref checker) = self.subvoxel_collision_checker {
            checker(min_x, min_y, min_z, max_x, max_y, max_z)
        } else {
            false
        }
    }
    
    /// Проверить коллизию хитбокса игрока с миром
    fn check_collision(&self, pos: Vec3, world_changes: &std::collections::HashMap<crate::gpu::terrain::BlockPos, crate::gpu::blocks::BlockType>) -> bool {
        // AABB игрока
        let p_min_x = pos.x - PLAYER_RADIUS;
        let p_max_x = pos.x + PLAYER_RADIUS;
        let p_min_y = pos.y;
        let p_max_y = pos.y + PLAYER_HEIGHT - 0.01;
        let p_min_z = pos.z - PLAYER_RADIUS;
        let p_max_z = pos.z + PLAYER_RADIUS;
        
        // Проверяем коллизию с суб-вокселями
        if self.check_subvoxel_collision(p_min_x, p_min_y, p_min_z, p_max_x, p_max_y, p_max_z) {
            return true;
        }
        
        // Проверяем все блоки, которые пересекает хитбокс игрока
        let min_x = p_min_x.floor() as i32;
        let max_x = p_max_x.floor() as i32;
        let min_y = p_min_y.floor() as i32;
        let max_y = p_max_y.floor() as i32;
        let min_z = p_min_z.floor() as i32;
        let max_z = p_max_z.floor() as i32;
        
        for bx in min_x..=max_x {
            for by in min_y..=max_y {
                for bz in min_z..=max_z {
                    if self.is_block_solid(bx, by, bz, world_changes) {
                        return true;
                    }
                }
            }
        }
        false
    }
    
    /// Обработка клавиатуры
    pub fn process_keyboard(&mut self, key: winit::keyboard::KeyCode, pressed: bool) {
        use winit::keyboard::KeyCode;
        
        // Сначала проверяем контроллер полёта
        self.flight.process_keyboard(key, pressed);
        
        match key {
            KeyCode::KeyW => self.forward = pressed,
            KeyCode::KeyS => self.backward = pressed,
            KeyCode::KeyA => self.left = pressed,
            KeyCode::KeyD => self.right = pressed,
            KeyCode::Space => self.jump = pressed,
            KeyCode::ControlLeft => self.sprint = pressed,
            KeyCode::ShiftLeft => self.sprint = pressed, // Shift тоже для спринта
            _ => {}
        }
    }
    
    /// Обработка мыши
    pub fn process_mouse(&mut self, dx: f64, dy: f64) {
        self.mouse_dx = dx as f32;
        self.mouse_dy = dy as f32;
    }
    
    /// Обновление игрока
    pub fn update(&mut self, player: &mut Player, dt: f32, world_changes: &std::collections::HashMap<crate::gpu::terrain::BlockPos, crate::gpu::blocks::BlockType>) {
        // === Вращение от мыши ===
        player.yaw += self.mouse_dx * self.sensitivity * dt;
        player.pitch -= self.mouse_dy * self.sensitivity * dt;
        
        // Ограничение pitch (не даём перевернуться)
        player.pitch = player.pitch.clamp(-1.5, 1.5);
        
        // Сброс дельты мыши
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
        
        // === Движение ===
        let forward = player.forward_horizontal();
        let right = player.right_horizontal();
        
        let mut move_dir = Vec3::zero();
        
        if self.forward { move_dir += forward; }
        if self.backward { move_dir -= forward; }
        if self.right { move_dir += right; }
        if self.left { move_dir -= right; }
        
        // Нормализуем если есть движение
        if move_dir.mag_sq() > 0.0 {
            move_dir = move_dir.normalized();
        }
        
        // === Режим полёта ===
        if self.flight.is_flying() {
            // Скорость полёта
            let speed = self.flight.get_fly_speed(self.sprint);
            
            // Горизонтальная скорость
            player.velocity.x = move_dir.x * speed;
            player.velocity.z = move_dir.z * speed;
            
            // Вертикальная скорость (Space вверх, Shift/Ctrl вниз)
            player.velocity.y = self.flight.get_vertical_velocity();
            
            // Применяем скорость
            player.position += player.velocity * dt;
            
            // В полёте всегда "на земле" для анимаций
            player.on_ground = false;
        } else {
            // === Обычная ходьба с гравитацией ===
            
            // Скорость (бег или ходьба)
            player.is_sprinting = self.sprint && self.forward;
            let speed = if player.is_sprinting {
                player.sprint_speed
            } else {
                player.move_speed
            };
            
            // Горизонтальная скорость
            player.velocity.x = move_dir.x * speed;
            player.velocity.z = move_dir.z * speed;
            
            // === Гравитация и прыжок ===
            if player.on_ground {
                if self.jump {
                    player.velocity.y = JUMP_VELOCITY;
                    player.on_ground = false;
                } else {
                    player.velocity.y = 0.0;
                }
            } else {
                // Применяем гравитацию
                player.velocity.y -= GRAVITY * dt;
                player.velocity.y = player.velocity.y.max(-TERMINAL_VELOCITY);
            }
            
            // === Применяем движение с коллизиями ===
            self.move_with_collision(player, dt, world_changes);
        }
    }
    
    /// Движение с проверкой коллизий (раздельно по осям)
    fn move_with_collision(&self, player: &mut Player, dt: f32, world_changes: &std::collections::HashMap<crate::gpu::terrain::BlockPos, crate::gpu::blocks::BlockType>) {
        let old_pos = player.position;
        
        // === Движение по X ===
        let new_x = old_pos.x + player.velocity.x * dt;
        let test_pos_x = Vec3::new(new_x, old_pos.y, old_pos.z);
        
        if !self.check_collision(test_pos_x, world_changes) {
            player.position.x = new_x;
        } else {
            player.velocity.x = 0.0;
        }
        
        // === Движение по Z ===
        let new_z = old_pos.z + player.velocity.z * dt;
        let test_pos_z = Vec3::new(player.position.x, old_pos.y, new_z);
        
        if !self.check_collision(test_pos_z, world_changes) {
            player.position.z = new_z;
        } else {
            player.velocity.z = 0.0;
        }
        
        // === Движение по Y ===
        let new_y = old_pos.y + player.velocity.y * dt;
        let test_pos_y = Vec3::new(player.position.x, new_y, player.position.z);
        
        if !self.check_collision(test_pos_y, world_changes) {
            player.position.y = new_y;
            player.on_ground = false;
        } else {
            // Столкнулись с чем-то
            if player.velocity.y < 0.0 {
                // Падали вниз - приземлились
                player.on_ground = true;
                // Выравниваем на верх блока
                player.position.y = (old_pos.y.floor() as i32) as f32;
                // Проверяем, не застряли ли
                if self.check_collision(player.position, world_changes) {
                    player.position.y = old_pos.y;
                }
            }
            player.velocity.y = 0.0;
        }
        
        // Дополнительная проверка on_ground (стоим ли на блоке)
        if !player.on_ground {
            let below = Vec3::new(player.position.x, player.position.y - 0.05, player.position.z);
            player.on_ground = self.check_collision(below, world_changes);
        }
    }
}
