use ultraviolet::Vec3;

const CHUNK_SIZE: i32 = 16;
const MIN_Y: f32 = -64.0;
const MAX_Y: f32 = 320.0;

/// Извлекает 6 плоскостей frustum из view-projection матрицы
/// Каждая плоскость: (nx, ny, nz, d) где nx*x + ny*y + nz*z + d >= 0 означает "внутри"
pub fn extract_frustum_planes(vp: &[[f32; 4]; 4]) -> [[f32; 4]; 6] {
    let m = vp;
    [
        // Left:   row3 + row0
        [m[0][3] + m[0][0], m[1][3] + m[1][0], m[2][3] + m[2][0], m[3][3] + m[3][0]],
        // Right:  row3 - row0
        [m[0][3] - m[0][0], m[1][3] - m[1][0], m[2][3] - m[2][0], m[3][3] - m[3][0]],
        // Bottom: row3 + row1
        [m[0][3] + m[0][1], m[1][3] + m[1][1], m[2][3] + m[2][1], m[3][3] + m[3][1]],
        // Top:    row3 - row1
        [m[0][3] - m[0][1], m[1][3] - m[1][1], m[2][3] - m[2][1], m[3][3] - m[3][1]],
        // Near:   row3 + row2
        [m[0][3] + m[0][2], m[1][3] + m[1][2], m[2][3] + m[2][2], m[3][3] + m[3][2]],
        // Far:    row3 - row2
        [m[0][3] - m[0][2], m[1][3] - m[1][2], m[2][3] - m[2][2], m[3][3] - m[3][2]],
    ]
}

/// Проверяет, находится ли AABB полностью снаружи плоскости frustum
fn is_aabb_outside_plane(plane: &[f32; 4], min: Vec3, max: Vec3) -> bool {
    let px = if plane[0] >= 0.0 { max.x } else { min.x };
    let py = if plane[1] >= 0.0 { max.y } else { min.y };
    let pz = if plane[2] >= 0.0 { max.z } else { min.z };
    
    plane[0] * px + plane[1] * py + plane[2] * pz + plane[3] < 0.0
}

/// Frustum culling: проверяет видимость AABB чанка
pub fn is_chunk_visible(view_proj: &[[f32; 4]; 4], chunk_x: i32, chunk_z: i32, scale: i32) -> bool {
    let size = (CHUNK_SIZE * scale.max(1)) as f32;
    let min_x = (chunk_x * CHUNK_SIZE) as f32;
    let min_z = (chunk_z * CHUNK_SIZE) as f32;
    
    let min = Vec3::new(min_x, MIN_Y, min_z);
    let max = Vec3::new(min_x + size, MAX_Y, min_z + size);
    
    let planes = extract_frustum_planes(view_proj);
    
    for plane in &planes {
        if is_aabb_outside_plane(plane, min, max) {
            return false;
        }
    }
    true
}
