// ============================================
// Noise Functions - Шумовые функции для генерации
// ============================================

/// Hash3D возвращает значение в диапазоне 0.0..1.0
#[inline(always)]
pub fn hash3d(x: i32, y: i32, z: i32) -> f32 {
    let n = x.wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(z.wrapping_mul(1274126177));
    let n = (n ^ (n >> 13)).wrapping_mul(1911520717);
    ((n as u32) as f32) / (u32::MAX as f32)
}

#[inline(always)]
fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// 3D Value Noise - быстрее Simplex, достаточно для пещер
#[inline]
pub fn noise3d(x: f32, y: f32, z: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let zi = z.floor() as i32;
    
    let xf = smoothstep(x - x.floor());
    let yf = smoothstep(y - y.floor());
    let zf = smoothstep(z - z.floor());
    
    let n000 = hash3d(xi, yi, zi);
    let n100 = hash3d(xi + 1, yi, zi);
    let n010 = hash3d(xi, yi + 1, zi);
    let n110 = hash3d(xi + 1, yi + 1, zi);
    let n001 = hash3d(xi, yi, zi + 1);
    let n101 = hash3d(xi + 1, yi, zi + 1);
    let n011 = hash3d(xi, yi + 1, zi + 1);
    let n111 = hash3d(xi + 1, yi + 1, zi + 1);
    
    let nx00 = n000 + xf * (n100 - n000);
    let nx10 = n010 + xf * (n110 - n010);
    let nx01 = n001 + xf * (n101 - n001);
    let nx11 = n011 + xf * (n111 - n011);
    
    let nxy0 = nx00 + yf * (nx10 - nx00);
    let nxy1 = nx01 + yf * (nx11 - nx01);
    
    nxy0 + zf * (nxy1 - nxy0)
}

// 2D noise functions (from original noise.rs)
#[inline(always)]
pub fn hash2d(x: i32, y: i32) -> f32 {
    let n = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
    let n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    ((n as u32) as f32) / (u32::MAX as f32)
}

#[inline]
pub fn noise2d(x: f32, y: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = smoothstep(x - x.floor());
    let yf = smoothstep(y - y.floor());
    
    let n00 = hash2d(xi, yi);
    let n10 = hash2d(xi + 1, yi);
    let n01 = hash2d(xi, yi + 1);
    let n11 = hash2d(xi + 1, yi + 1);
    
    let nx0 = n00 + xf * (n10 - n00);
    let nx1 = n01 + xf * (n11 - n01);
    
    nx0 + yf * (nx1 - nx0)
}

/// FBM 2D - несколько октав шума
#[inline]
pub fn fbm2d(x: f32, y: f32, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_value = 0.0;
    
    for _ in 0..octaves {
        value += amplitude * noise2d(x * frequency, y * frequency);
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    
    value / max_value
}
