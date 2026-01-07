// ============================================
// Celestial Bodies - Солнце и Луна
// ============================================
// Система смены дня и ночи

use ultraviolet::Vec3;
use std::f32::consts::{PI, TAU};

/// Время суток (0.0 - 1.0, где 0.0 = полночь, 0.5 = полдень)
#[derive(Clone, Copy, Debug)]
pub struct TimeOfDay {
    /// Нормализованное время (0.0 - 1.0)
    pub time: f32,
    /// Скорость течения времени (1.0 = реальное время, 72.0 = 20 минут на сутки)
    pub speed: f32,
}

impl TimeOfDay {
    pub fn new(time: f32, speed: f32) -> Self {
        Self {
            time: time.rem_euclid(1.0),
            speed,
        }
    }

    /// Обновить время
    pub fn update(&mut self, dt: f32) {
        // 1 игровой день = 24 минуты при speed = 1.0
        self.time += dt * self.speed / (24.0 * 60.0);
        self.time = self.time.rem_euclid(1.0);
    }

    /// Угол солнца в радианах (0 = восход, PI/2 = полдень, PI = закат)
    pub fn sun_angle(&self) -> f32 {
        (self.time - 0.25) * TAU
    }

    /// Угол луны (противоположен солнцу)
    pub fn moon_angle(&self) -> f32 {
        self.sun_angle() + PI
    }

    /// Это день? (солнце над горизонтом)
    pub fn is_day(&self) -> bool {
        self.time > 0.25 && self.time < 0.75
    }

    /// Высота солнца над горизонтом (-1 до 1)
    pub fn sun_height(&self) -> f32 {
        self.sun_angle().sin()
    }
}

impl Default for TimeOfDay {
    fn default() -> Self {
        Self::new(0.35, 1.0) // Утро, нормальная скорость
    }
}

/// Небесное тело (солнце или луна)
#[derive(Clone, Copy, Debug)]
pub struct CelestialBody {
    /// Направление к телу (нормализованное)
    pub direction: Vec3,
    /// Цвет света
    pub color: Vec3,
    /// Интенсивность (0.0 - 1.0)
    pub intensity: f32,
    /// Видимость (0.0 - 1.0, для плавных переходов)
    pub visibility: f32,
}

impl CelestialBody {
    pub fn new() -> Self {
        Self {
            direction: Vec3::new(0.0, 1.0, 0.0),
            color: Vec3::one(),
            intensity: 1.0,
            visibility: 1.0,
        }
    }

    /// Направление света (от тела к поверхности)
    pub fn light_direction(&self) -> Vec3 {
        -self.direction
    }
}

/// Солнце
pub struct Sun {
    pub body: CelestialBody,
}

impl Sun {
    pub fn new() -> Self {
        Self {
            body: CelestialBody::new(),
        }
    }

    /// Обновить позицию и цвет солнца
    pub fn update(&mut self, time: &TimeOfDay) {
        let angle = time.sun_angle();
        
        // Позиция солнца на небосводе
        let height = angle.sin();
        let horizontal = angle.cos();
        
        self.body.direction = Vec3::new(
            horizontal * 0.3,
            height,
            horizontal * 0.95,
        ).normalized();

        // Видимость (плавный переход на горизонте)
        self.body.visibility = smoothstep(-0.1, 0.1, height);

        // Интенсивность зависит от высоты
        self.body.intensity = smoothstep(-0.05, 0.3, height);

        // Цвет солнца
        if height > 0.2 {
            // День - тёплый белый
            self.body.color = Vec3::new(1.0, 0.98, 0.95);
        } else if height > 0.0 {
            // Закат/рассвет - оранжевый/красный
            let t = height / 0.2;
            self.body.color = Vec3::new(
                1.0,
                0.5 + 0.48 * t,
                0.3 + 0.65 * t,
            );
        } else {
            // Под горизонтом
            self.body.color = Vec3::new(1.0, 0.4, 0.2);
        }
    }
}

/// Луна
pub struct Moon {
    pub body: CelestialBody,
    /// Фаза луны (0.0 - 1.0)
    pub phase: f32,
}

impl Moon {
    pub fn new() -> Self {
        Self {
            body: CelestialBody {
                color: Vec3::new(0.6, 0.7, 0.9), // Холодный голубоватый
                intensity: 0.15,
                ..CelestialBody::new()
            },
            phase: 0.5, // Полнолуние
        }
    }

    /// Обновить позицию луны
    pub fn update(&mut self, time: &TimeOfDay) {
        let angle = time.moon_angle();
        
        let height = angle.sin();
        let horizontal = angle.cos();
        
        self.body.direction = Vec3::new(
            horizontal * 0.3,
            height,
            horizontal * 0.95,
        ).normalized();

        // Видимость
        self.body.visibility = smoothstep(-0.1, 0.1, height);

        // Интенсивность зависит от фазы и высоты
        let phase_intensity = 0.5 + 0.5 * (self.phase * TAU).cos().abs();
        self.body.intensity = 0.15 * phase_intensity * smoothstep(-0.05, 0.2, height);
    }

    /// Обновить фазу луны (медленный цикл ~29.5 дней)
    pub fn update_phase(&mut self, dt: f32) {
        self.phase += dt / (29.5 * 24.0 * 60.0 * 60.0);
        self.phase = self.phase.rem_euclid(1.0);
    }
}

/// Система дня и ночи
pub struct DayNightCycle {
    pub time: TimeOfDay,
    pub sun: Sun,
    pub moon: Moon,
    /// Ambient освещение
    pub ambient_color: Vec3,
    pub ambient_intensity: f32,
    /// Цвет неба
    pub sky_color: Vec3,
    /// Цвет тумана
    pub fog_color: Vec3,
}

impl DayNightCycle {
    pub fn new() -> Self {
        let mut cycle = Self {
            time: TimeOfDay::default(),
            sun: Sun::new(),
            moon: Moon::new(),
            ambient_color: Vec3::one(),
            ambient_intensity: 0.3,
            sky_color: Vec3::new(0.5, 0.7, 1.0),
            fog_color: Vec3::new(0.7, 0.8, 0.9),
        };
        cycle.update(0.0);
        cycle
    }

    /// Установить время суток
    pub fn set_time(&mut self, time: f32) {
        self.time.time = time.rem_euclid(1.0);
        self.update(0.0);
    }

    /// Установить скорость времени
    pub fn set_speed(&mut self, speed: f32) {
        self.time.speed = speed;
    }

    /// Обновить всю систему
    pub fn update(&mut self, dt: f32) {
        self.time.update(dt);
        self.sun.update(&self.time);
        self.moon.update(&self.time);
        self.moon.update_phase(dt);
        
        self.update_ambient();
        self.update_sky();
    }

    fn update_ambient(&mut self) {
        let sun_h = self.time.sun_height();
        
        if sun_h > 0.1 {
            // День
            self.ambient_color = Vec3::new(0.9, 0.95, 1.0);
            self.ambient_intensity = 0.3 + 0.1 * sun_h;
        } else if sun_h > -0.1 {
            // Сумерки
            let t = (sun_h + 0.1) / 0.2;
            self.ambient_color = Vec3::new(
                0.4 + 0.5 * t,
                0.4 + 0.55 * t,
                0.6 + 0.4 * t,
            );
            self.ambient_intensity = 0.15 + 0.15 * t;
        } else {
            // Ночь
            self.ambient_color = Vec3::new(0.3, 0.35, 0.5);
            self.ambient_intensity = 0.1 + 0.05 * self.moon.body.intensity;
        }
    }

    fn update_sky(&mut self) {
        let sun_h = self.time.sun_height();
        
        if sun_h > 0.2 {
            // День - голубое небо
            self.sky_color = Vec3::new(0.5, 0.7, 1.0);
            self.fog_color = Vec3::new(0.7, 0.8, 0.95);
        } else if sun_h > 0.0 {
            // Закат/рассвет
            let t = sun_h / 0.2;
            self.sky_color = Vec3::new(
                0.9 - 0.4 * t,
                0.4 + 0.3 * t,
                0.3 + 0.7 * t,
            );
            self.fog_color = Vec3::new(
                0.9 - 0.2 * t,
                0.6 + 0.2 * t,
                0.5 + 0.45 * t,
            );
        } else if sun_h > -0.2 {
            // Сумерки
            let t = (sun_h + 0.2) / 0.2;
            self.sky_color = Vec3::new(
                0.1 + 0.8 * t,
                0.1 + 0.3 * t,
                0.2 + 0.1 * t,
            );
            self.fog_color = Vec3::new(
                0.15 + 0.75 * t,
                0.15 + 0.45 * t,
                0.25 + 0.25 * t,
            );
        } else {
            // Ночь - тёмно-синее небо
            self.sky_color = Vec3::new(0.05, 0.07, 0.15);
            self.fog_color = Vec3::new(0.1, 0.12, 0.2);
        }
    }

    /// Получить основной источник света (солнце днём, луна ночью)
    pub fn primary_light(&self) -> &CelestialBody {
        if self.time.is_day() {
            &self.sun.body
        } else {
            &self.moon.body
        }
    }

    /// Получить направление света для теней
    pub fn shadow_light_direction(&self) -> Vec3 {
        // Используем солнце для теней днём, луну ночью
        // Но ночью тени слабее
        if self.sun.body.visibility > 0.1 {
            self.sun.body.light_direction()
        } else {
            self.moon.body.light_direction()
        }
    }

    /// Получить интенсивность теней
    pub fn shadow_intensity(&self) -> f32 {
        if self.time.is_day() {
            self.sun.body.intensity
        } else {
            self.moon.body.intensity * 0.5
        }
    }
}

impl Default for DayNightCycle {
    fn default() -> Self {
        Self::new()
    }
}

/// Плавная интерполяция
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
