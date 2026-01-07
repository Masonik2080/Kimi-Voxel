// ============================================
// Light Sources - Направленный свет (солнце)
// ============================================

use bytemuck::{Pod, Zeroable};
use ultraviolet::{Vec3, Mat4};

/// Направленный свет (солнце/луна)
#[derive(Clone, Copy, Debug)]
pub struct DirectionalLight {
    /// Направление света (нормализованное, от источника)
    pub direction: Vec3,
    /// Цвет света
    pub color: Vec3,
    /// Интенсивность
    pub intensity: f32,
}

impl DirectionalLight {
    pub fn new(direction: Vec3, color: Vec3, intensity: f32) -> Self {
        Self {
            direction: direction.normalized(),
            color,
            intensity,
        }
    }
    
    /// Создать матрицу вида для shadow map
    pub fn view_matrix(&self, center: Vec3) -> Mat4 {
        let up = if self.direction.y.abs() > 0.99 {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        
        Mat4::look_at(
            center - self.direction * 100.0,
            center,
            up,
        )
    }
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(0.4, -0.8, 0.3).normalized(),
            color: Vec3::new(1.0, 0.98, 0.9),
            intensity: 1.0,
        }
    }
}

/// Солнце с циклом дня/ночи
pub struct SunLight {
    /// Базовый угол (0 = полдень)
    pub angle: f32,
    /// Скорость вращения (радиан/сек)
    pub rotation_speed: f32,
    /// Текущий направленный свет
    pub light: DirectionalLight,
}

impl SunLight {
    pub fn new() -> Self {
        Self {
            angle: 0.0,
            rotation_speed: 0.0, // Статичное солнце по умолчанию
            light: DirectionalLight::default(),
        }
    }
    
    /// Обновить позицию солнца
    pub fn update(&mut self, dt: f32) {
        self.angle += self.rotation_speed * dt;
        self.angle %= std::f32::consts::TAU;
        
        // Вычисляем направление солнца
        let cos_a = self.angle.cos();
        let sin_a = self.angle.sin();
        
        self.light.direction = Vec3::new(
            0.3 * cos_a,
            -sin_a.abs().max(0.1), // Солнце всегда сверху
            0.3 * sin_a,
        ).normalized();
        
        // Цвет зависит от высоты солнца
        let height = (-self.light.direction.y).max(0.0);
        
        if height > 0.5 {
            // День - белый свет
            self.light.color = Vec3::new(1.0, 0.98, 0.95);
            self.light.intensity = 1.0;
        } else if height > 0.1 {
            // Закат/рассвет - оранжевый
            let t = (height - 0.1) / 0.4;
            self.light.color = Vec3::new(1.0, 0.6 + 0.38 * t, 0.4 + 0.55 * t);
            self.light.intensity = 0.6 + 0.4 * t;
        } else {
            // Ночь - синеватый лунный свет
            self.light.color = Vec3::new(0.4, 0.5, 0.7);
            self.light.intensity = 0.2;
        }
    }
    
    /// Установить время суток (0.0 = полночь, 0.5 = полдень)
    pub fn set_time_of_day(&mut self, time: f32) {
        self.angle = (time - 0.25) * std::f32::consts::TAU;
        self.update(0.0);
    }
}

impl Default for SunLight {
    fn default() -> Self {
        let mut sun = Self::new();
        sun.set_time_of_day(0.4); // Утро
        sun
    }
}

/// GPU uniform для света
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LightUniform {
    pub direction: [f32; 3],
    pub intensity: f32,
    pub color: [f32; 3],
    pub _padding: f32,
}

impl From<&DirectionalLight> for LightUniform {
    fn from(light: &DirectionalLight) -> Self {
        Self {
            direction: light.direction.into(),
            intensity: light.intensity,
            color: light.color.into(),
            _padding: 0.0,
        }
    }
}
