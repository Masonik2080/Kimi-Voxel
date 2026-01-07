// ============================================
// Camera - Система камеры с режимами
// ============================================
// Камера следует за игроком и поддерживает:
// - Первое лицо (FirstPerson)
// - Третье лицо сзади (ThirdPersonBack)
// - Третье лицо спереди (ThirdPersonFront)

use ultraviolet::{Mat4, Vec3};
use super::player::Player;

/// Режим камеры
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMode {
    /// Камера в глазах игрока
    FirstPerson,
    /// Камера сзади игрока
    ThirdPersonBack,
    /// Камера спереди (смотрит в лицо)
    ThirdPersonFront,
}

impl CameraMode {
    /// Переключить на следующий режим
    pub fn next(self) -> Self {
        match self {
            CameraMode::FirstPerson => CameraMode::ThirdPersonBack,
            CameraMode::ThirdPersonBack => CameraMode::ThirdPersonFront,
            CameraMode::ThirdPersonFront => CameraMode::FirstPerson,
        }
    }
}

/// Камера — "глупый" объект, следующий за игроком
pub struct Camera {
    /// Текущая позиция камеры (вычисляется)
    pub position: Vec3,
    
    /// Направление взгляда (вычисляется)
    forward: Vec3,
    
    /// Режим камеры
    pub mode: CameraMode,
    
    /// Дистанция от игрока в режиме 3-го лица
    pub third_person_distance: f32,
    
    /// Минимальная дистанция (при коллизии со стеной)
    pub min_distance: f32,
    
    /// Текущая реальная дистанция (после raycast)
    current_distance: f32,
    
    /// Параметры проекции
    pub aspect: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 50.0, 0.0),
            forward: Vec3::new(0.0, 0.0, 1.0),
            mode: CameraMode::FirstPerson,
            third_person_distance: 5.0,
            min_distance: 1.0,
            current_distance: 5.0,
            aspect,
            fov: 70.0_f32.to_radians(),
            near: 0.1,
            far: 2000.0,
        }
    }
    
    /// Направление взгляда камеры
    pub fn forward(&self) -> Vec3 {
        self.forward
    }
    
    /// Вектор вправо
    pub fn right(&self) -> Vec3 {
        self.forward.cross(Vec3::unit_y()).normalized()
    }
    
    /// Обновить камеру на основе позиции игрока
    pub fn update_from_player(&mut self, player: &Player) {
        let eye_pos = player.eye_position();
        let player_forward = player.forward();
        
        match self.mode {
            CameraMode::FirstPerson => {
                // Камера точно в глазах
                self.position = eye_pos;
                self.forward = player_forward;
            }
            
            CameraMode::ThirdPersonBack => {
                // Камера сзади игрока
                // CameraPos = EyePos - Forward * Distance
                let target_pos = eye_pos - player_forward * self.third_person_distance;
                
                // Raycast для коллизии со стенами
                self.current_distance = self.raycast_distance(
                    eye_pos,
                    -player_forward,
                    self.third_person_distance,
                );
                
                self.position = eye_pos - player_forward * self.current_distance;
                self.forward = player_forward;
            }
            
            CameraMode::ThirdPersonFront => {
                // Камера спереди, смотрит на игрока
                let target_pos = eye_pos + player_forward * self.third_person_distance;
                
                // Raycast для коллизии
                self.current_distance = self.raycast_distance(
                    eye_pos,
                    player_forward,
                    self.third_person_distance,
                );
                
                self.position = eye_pos + player_forward * self.current_distance;
                // Смотрим на игрока (инвертированный forward)
                self.forward = -player_forward;
            }
        }
    }
    
    /// Raycast от головы игрока к желаемой позиции камеры
    /// Возвращает безопасную дистанцию (не проходящую сквозь стены)
    fn raycast_distance(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> f32 {
        // Простой raycast через сэмплирование
        // В реальном проекте здесь был бы полноценный raycast по воксельной сетке
        
        let step_size = 0.25;
        let steps = (max_distance / step_size) as i32;
        
        for i in 1..=steps {
            let dist = i as f32 * step_size;
            let check_pos = origin + direction * dist;
            
            // Проверяем коллизию с террейном
            // Используем простую проверку: если точка ниже поверхности — коллизия
            let terrain_height = self.sample_terrain_height(check_pos.x, check_pos.z);
            
            if check_pos.y < terrain_height + 0.5 {
                // Нашли коллизию — возвращаем предыдущую безопасную дистанцию
                return ((i - 1) as f32 * step_size).max(self.min_distance);
            }
        }
        
        // Нет коллизий — полная дистанция
        max_distance
    }
    
    /// Сэмплирование высоты террейна (упрощённое)
    /// В реальном проекте это должно использовать ту же функцию что и генератор
    fn sample_terrain_height(&self, x: f32, z: f32) -> f32 {
        // Используем ту же функцию высоты что и террейн
        use crate::gpu::terrain::get_height;
        get_height(x, z)
    }
    
    /// Матрица вида (View Matrix)
    pub fn view_matrix(&self) -> Mat4 {
        let target = self.position + self.forward;
        Mat4::look_at(self.position, target, Vec3::unit_y())
    }
    
    /// Матрица проекции (Perspective с Reversed-Z для лучшей точности вдали)
    pub fn projection_matrix(&self) -> Mat4 {
        // Reversed-Z: меняем near и far местами
        let mut proj = ultraviolet::projection::perspective_wgpu_dx(
            self.fov,
            self.aspect,
            self.far,  // far вместо near
            self.near, // near вместо far
        );
        proj
    }
    
    /// Комбинированная матрица View-Projection
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }
    
    /// Переключить режим камеры
    pub fn toggle_mode(&mut self) {
        self.mode = self.mode.next();
    }
    
    /// Нужно ли рендерить модель игрока
    pub fn should_render_player(&self) -> bool {
        match self.mode {
            CameraMode::FirstPerson => false,
            CameraMode::ThirdPersonBack | CameraMode::ThirdPersonFront => true,
        }
    }
    
    /// Текущая дистанция до игрока (для отладки)
    pub fn current_distance(&self) -> f32 {
        self.current_distance
    }
}

// ============================================
// Старый CameraController (для совместимости)
// ============================================

/// Контроллер камеры (WASD + мышь) — DEPRECATED
/// Используйте PlayerController вместо этого
pub struct CameraController {
    pub speed: f32,
    pub sensitivity: f32,
    
    // Состояние клавиш
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    
    // Дельта мыши
    mouse_dx: f32,
    mouse_dy: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
            mouse_dx: 0.0,
            mouse_dy: 0.0,
        }
    }

    pub fn process_keyboard(&mut self, key: winit::keyboard::KeyCode, pressed: bool) {
        use winit::keyboard::KeyCode;
        match key {
            KeyCode::KeyW => self.forward = pressed,
            KeyCode::KeyS => self.backward = pressed,
            KeyCode::KeyA => self.left = pressed,
            KeyCode::KeyD => self.right = pressed,
            KeyCode::Space => self.up = pressed,
            KeyCode::ShiftLeft => self.down = pressed,
            _ => {}
        }
    }

    pub fn process_mouse(&mut self, dx: f64, dy: f64) {
        self.mouse_dx = dx as f32;
        self.mouse_dy = dy as f32;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: f32) {
        // Этот метод больше не используется напрямую
        // Оставлен для совместимости
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
    }
}
