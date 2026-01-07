// ============================================
// Cascade Configuration - Настройки каскадов CSM
// ============================================

use ultraviolet::{Vec3, Vec4, Mat4};

/// Конфигурация каскадов теней
#[derive(Clone, Debug)]
pub struct CascadeConfig {
    /// Количество каскадов (обычно 3-4)
    pub num_cascades: usize,
    /// Разрешение shadow map для каждого каскада
    pub resolution: u32,
    /// Дальности каскадов (в блоках)
    pub cascade_distances: Vec<f32>,
    /// Коэффициент перекрытия между каскадами
    pub overlap_factor: f32,
    /// Стабилизация (снижает мерцание теней)
    pub stabilize: bool,
}

impl CascadeConfig {
    /// Конфигурация для больших миров (горы, дальние тени)
    pub fn large_world() -> Self {
        Self {
            num_cascades: 2,
            resolution: 2048,
            cascade_distances: vec![64.0, 256.0, 512.0, 1024.0],
            overlap_factor: 0.1,
            stabilize: true,
        }
    }
    
    /// Конфигурация для средних миров
    pub fn medium_world() -> Self {
        Self {
            num_cascades: 3,
            resolution: 1024,
            cascade_distances: vec![32.0, 128.0, 512.0],
            overlap_factor: 0.1,
            stabilize: true,
        }
    }
    
    /// Быстрая конфигурация (меньше качество, выше FPS)
    pub fn fast() -> Self {
        Self {
            num_cascades: 2,
            resolution: 512,
            cascade_distances: vec![64.0, 256.0],
            overlap_factor: 0.05,
            stabilize: false,
        }
    }
}

impl Default for CascadeConfig {
    fn default() -> Self {
        Self::large_world()
    }
}

/// Один каскад shadow map
#[derive(Clone, Debug)]
pub struct Cascade {
    /// Индекс каскада
    pub index: usize,
    /// Ближняя плоскость
    pub near: f32,
    /// Дальняя плоскость
    pub far: f32,
    /// Матрица вида-проекции для рендеринга теней
    pub light_view_proj: Mat4,
    /// Размер текселя в мировых координатах (для стабилизации)
    pub texel_size: f32,
}

impl Cascade {
    pub fn new(index: usize, near: f32, far: f32) -> Self {
        Self {
            index,
            near,
            far,
            light_view_proj: Mat4::identity(),
            texel_size: 0.0,
        }
    }
    
    /// Вычислить frustum corners камеры для этого каскада
    pub fn compute_frustum_corners(
        &self,
        camera_inv_view_proj: &Mat4,
    ) -> [Vec3; 8] {
        let mut corners = [Vec3::zero(); 8];
        
        // NDC координаты углов frustum
        let ndc_corners = [
            Vec4::new(-1.0, -1.0, 0.0, 1.0), // near bottom-left
            Vec4::new( 1.0, -1.0, 0.0, 1.0), // near bottom-right
            Vec4::new( 1.0,  1.0, 0.0, 1.0), // near top-right
            Vec4::new(-1.0,  1.0, 0.0, 1.0), // near top-left
            Vec4::new(-1.0, -1.0, 1.0, 1.0), // far bottom-left
            Vec4::new( 1.0, -1.0, 1.0, 1.0), // far bottom-right
            Vec4::new( 1.0,  1.0, 1.0, 1.0), // far top-right
            Vec4::new(-1.0,  1.0, 1.0, 1.0), // far top-left
        ];
        
        for (i, ndc) in ndc_corners.iter().enumerate() {
            let world = *camera_inv_view_proj * *ndc;
            corners[i] = Vec3::new(world.x / world.w, world.y / world.w, world.z / world.w);
        }
        
        corners
    }
    
    /// Обновить матрицу света для этого каскада
    pub fn update_light_matrix(
        &mut self,
        light_dir: Vec3,
        frustum_corners: &[Vec3; 8],
        resolution: u32,
        stabilize: bool,
    ) {
        // Центр frustum
        let center = frustum_corners.iter().fold(Vec3::zero(), |acc, c| acc + *c) / 8.0;
        
        // Матрица вида света
        let up = if light_dir.y.abs() > 0.99 {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        
        let light_view = Mat4::look_at(
            center - light_dir * 100.0,
            center,
            up,
        );
        
        // Трансформируем углы в пространство света
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;
        
        for corner in frustum_corners {
            let light_space = light_view * Vec4::new(corner.x, corner.y, corner.z, 1.0);
            min_x = min_x.min(light_space.x);
            max_x = max_x.max(light_space.x);
            min_y = min_y.min(light_space.y);
            max_y = max_y.max(light_space.y);
            min_z = min_z.min(light_space.z);
            max_z = max_z.max(light_space.z);
        }
        
        // Расширяем Z для захвата объектов за frustum
        let z_mult = 2.0;
        let z_range = max_z - min_z;
        min_z -= z_range * (z_mult - 1.0);
        
        // Стабилизация - привязка к текселям
        if stabilize {
            let world_units_per_texel = (max_x - min_x) / resolution as f32;
            self.texel_size = world_units_per_texel;
            
            min_x = (min_x / world_units_per_texel).floor() * world_units_per_texel;
            max_x = (max_x / world_units_per_texel).ceil() * world_units_per_texel;
            min_y = (min_y / world_units_per_texel).floor() * world_units_per_texel;
            max_y = (max_y / world_units_per_texel).ceil() * world_units_per_texel;
        }
        
        // Ортографическая проекция
        let light_proj = Mat4::new(
            Vec4::new(2.0 / (max_x - min_x), 0.0, 0.0, 0.0),
            Vec4::new(0.0, 2.0 / (max_y - min_y), 0.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0 / (max_z - min_z), 0.0),
            Vec4::new(
                -(max_x + min_x) / (max_x - min_x),
                -(max_y + min_y) / (max_y - min_y),
                -min_z / (max_z - min_z),
                1.0,
            ),
        );
        
        self.light_view_proj = light_proj * light_view;
    }
}
