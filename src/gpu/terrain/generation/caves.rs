// ============================================
// Cave System - 3D Noise для пещер
// ============================================

use super::noise::noise3d;

/// Параметры генерации пещер
#[derive(Clone, Copy)]
pub struct CaveParams {
    pub scale: f32,
    pub threshold: f32,
    pub surface_offset: i32,
    pub min_height: i32,
    pub vertical_squeeze: f32,
}

impl Default for CaveParams {
    fn default() -> Self {
        Self {
            scale: 0.025,
            threshold: 0.48,
            surface_offset: 8,
            min_height: -64,
            vertical_squeeze: 0.5,
        }
    }
}

/// Проверяет, является ли блок пещерой
#[inline]
pub fn is_cave(x: i32, y: i32, z: i32, params: &CaveParams) -> bool {
    let fx = x as f32 * params.scale;
    let fy = y as f32 * params.scale * params.vertical_squeeze;
    let fz = z as f32 * params.scale;
    
    let cave_noise = noise3d(fx, fy, fz);
    cave_noise > params.threshold
}
