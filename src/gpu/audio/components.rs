// ============================================
// Audio Components - Чистые данные (ECS)
// ============================================

use ultraviolet::Vec3;

/// Состояние системы шагов
pub struct FootstepState {
    pub last_position: Vec3,
    pub distance_traveled: f32,
    pub time_since_last_step: f32,
    pub first_frame: bool,
}

impl FootstepState {
    pub fn new() -> Self {
        Self {
            last_position: Vec3::zero(),
            distance_traveled: 0.0,
            time_since_last_step: 0.0,
            first_frame: true,
        }
    }
}

impl Default for FootstepState {
    fn default() -> Self {
        Self::new()
    }
}

/// Состояние системы прыжков
pub struct JumpState {
    pub was_on_ground: bool,
    pub cooldown: f32,
}

impl JumpState {
    pub fn new() -> Self {
        Self {
            was_on_ground: true,
            cooldown: 0.0,
        }
    }
}

impl Default for JumpState {
    fn default() -> Self {
        Self::new()
    }
}

/// Тип окружения для звука
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum EnvironmentType {
    #[default]
    OpenField,
    Forest,
    Cave,
    TightSpace,
    DeepUnderground,
}

/// Параметры окружения для звука
#[derive(Clone, Copy, Debug)]
pub struct EnvironmentParams {
    pub env_type: EnvironmentType,
    pub avg_wall_distance: f32,
    pub enclosure: f32,
    pub ceiling_height: f32,
    pub depth_underground: f32,
}

impl Default for EnvironmentParams {
    fn default() -> Self {
        Self {
            env_type: EnvironmentType::OpenField,
            avg_wall_distance: 20.0,
            enclosure: 0.0,
            ceiling_height: 100.0,
            depth_underground: 0.0,
        }
    }
}

/// Модификаторы звука на основе окружения
#[derive(Clone, Copy, Debug)]
pub struct SoundModifiers {
    pub volume_mult: f32,
    pub pitch_mult: f32,
    pub reverb_amount: f32,
    pub muffling: f32,
}

impl Default for SoundModifiers {
    fn default() -> Self {
        Self::from_environment(&EnvironmentParams::default())
    }
}

impl SoundModifiers {
    /// Рассчитать модификаторы на основе окружения
    pub fn from_environment(env: &EnvironmentParams) -> Self {
        match env.env_type {
            EnvironmentType::OpenField => Self {
                volume_mult: 1.0,
                pitch_mult: 1.0,
                reverb_amount: 0.05,
                muffling: 0.0,
            },
            EnvironmentType::Forest => Self {
                volume_mult: 0.95,
                pitch_mult: 1.0,
                reverb_amount: 0.15,
                muffling: 0.05,
            },
            EnvironmentType::Cave => Self {
                volume_mult: 1.1,
                pitch_mult: 0.98,
                reverb_amount: 0.4 + env.enclosure * 0.3,
                muffling: 0.1,
            },
            EnvironmentType::TightSpace => Self {
                volume_mult: 1.2,
                pitch_mult: 0.95,
                reverb_amount: 0.6,
                muffling: 0.15,
            },
            EnvironmentType::DeepUnderground => Self {
                volume_mult: 1.15,
                pitch_mult: 0.92,
                reverb_amount: 0.5,
                muffling: 0.2,
            },
        }
    }
    
    /// Применить модификаторы к базовым настройкам звука
    pub fn apply(&self, base_volume: f32, base_pitch: f32) -> (f64, f64) {
        let pitch_with_reverb = base_pitch * self.pitch_mult * (1.0 - self.muffling * 0.1);
        let volume = base_volume * self.volume_mult;
        (volume as f64, pitch_with_reverb as f64)
    }
}

/// Тип функции проверки твёрдости блока
pub type BlockSolidChecker = Box<dyn Fn(i32, i32, i32) -> bool + Send + Sync>;
