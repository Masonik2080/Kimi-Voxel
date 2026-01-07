// ============================================
// Flight Mode - Режим полёта
// ============================================
// F - включить/выключить полёт
// Space - вверх, Shift/Ctrl - вниз
// Нет гравитации, свободное перемещение

use ultraviolet::Vec3;

/// Режим передвижения игрока
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementMode {
    /// Обычная ходьба с гравитацией
    Walking,
    /// Полёт (креатив)
    Flying,
}

/// Контроллер полёта
pub struct FlightController {
    /// Текущий режим
    pub mode: MovementMode,
    
    /// Скорость полёта
    pub fly_speed: f32,
    
    /// Скорость быстрого полёта (с Ctrl)
    pub fast_fly_speed: f32,
    
    /// Вертикальная скорость полёта
    pub vertical_speed: f32,
    
    /// Клавиша вверх (Space)
    pub up: bool,
    
    /// Клавиша вниз (Shift/Ctrl в полёте)
    pub down: bool,
}

impl FlightController {
    pub fn new() -> Self {
        Self {
            mode: MovementMode::Walking,
            fly_speed: 12.0,
            fast_fly_speed: 480.0, // x40 от базовой скорости полёта
            vertical_speed: 10.0,
            up: false,
            down: false,
        }
    }
    
    /// Переключить режим полёта
    pub fn toggle_flight(&mut self) {
        self.mode = match self.mode {
            MovementMode::Walking => {
                MovementMode::Flying
            }
            MovementMode::Flying => {
                MovementMode::Walking
            }
        };
    }
    
    /// Проверка режима полёта
    pub fn is_flying(&self) -> bool {
        self.mode == MovementMode::Flying
    }
    
    /// Обработка клавиш для полёта
    pub fn process_keyboard(&mut self, key: winit::keyboard::KeyCode, pressed: bool) -> bool {
        use winit::keyboard::KeyCode;
        
        match key {
            KeyCode::KeyF if pressed => {
                self.toggle_flight();
                true // Обработано
            }
            KeyCode::Space => {
                self.up = pressed;
                false // Пусть основной контроллер тоже обработает (для прыжка)
            }
            KeyCode::ShiftLeft | KeyCode::ControlLeft => {
                self.down = pressed;
                false // Пусть основной контроллер тоже обработает (для спринта)
            }
            _ => false,
        }
    }
    
    /// Вычислить вертикальную скорость в режиме полёта
    pub fn get_vertical_velocity(&self) -> f32 {
        if !self.is_flying() {
            return 0.0;
        }
        
        let mut vy = 0.0;
        if self.up {
            vy += self.vertical_speed;
        }
        if self.down {
            vy -= self.vertical_speed;
        }
        vy
    }
    
    /// Получить скорость передвижения (с учётом быстрого полёта)
    pub fn get_fly_speed(&self, is_fast: bool) -> f32 {
        if is_fast {
            self.fast_fly_speed
        } else {
            self.fly_speed
        }
    }
}

impl Default for FlightController {
    fn default() -> Self {
        Self::new()
    }
}
