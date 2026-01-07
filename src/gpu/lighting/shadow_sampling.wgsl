// ============================================
// Shadow Sampling Functions - PCF фильтрация
// ============================================
// Функции для сэмплирования CSM теней
// Включается в основной terrain shader

// Структура для shadow uniforms
struct ShadowData {
    light_view_proj_0: mat4x4<f32>,
    light_view_proj_1: mat4x4<f32>,
    light_view_proj_2: mat4x4<f32>,
    light_view_proj_3: mat4x4<f32>,
    cascade_distances: vec4<f32>,
    num_cascades: u32,
    texel_size: f32,
    bias: f32,
    normal_bias: f32,
}

// Выбор каскада по расстоянию от камеры
fn select_cascade(view_depth: f32, cascade_distances: vec4<f32>, num_cascades: u32) -> u32 {
    for (var i: u32 = 0u; i < num_cascades; i = i + 1u) {
        if (view_depth < cascade_distances[i]) {
            return i;
        }
    }
    return num_cascades - 1u;
}

// Получить матрицу света для каскада
fn get_light_matrix(cascade: u32, shadow_data: ShadowData) -> mat4x4<f32> {
    switch (cascade) {
        case 0u: { return shadow_data.light_view_proj_0; }
        case 1u: { return shadow_data.light_view_proj_1; }
        case 2u: { return shadow_data.light_view_proj_2; }
        default: { return shadow_data.light_view_proj_3; }
    }
}

// PCF 3x3 фильтрация для мягких теней
fn sample_shadow_pcf(
    shadow_map: texture_depth_2d_array,
    shadow_sampler: sampler_comparison,
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    light_matrix: mat4x4<f32>,
    cascade: u32,
    bias: f32,
    texel_size: f32,
) -> f32 {
    // Трансформируем в пространство света
    let light_space = light_matrix * vec4<f32>(world_pos, 1.0);
    var proj_coords = light_space.xyz / light_space.w;
    
    // Преобразуем из [-1,1] в [0,1]
    proj_coords.x = proj_coords.x * 0.5 + 0.5;
    proj_coords.y = proj_coords.y * -0.5 + 0.5; // Y инвертирован
    
    // Проверка границ
    if (proj_coords.x < 0.0 || proj_coords.x > 1.0 ||
        proj_coords.y < 0.0 || proj_coords.y > 1.0 ||
        proj_coords.z < 0.0 || proj_coords.z > 1.0) {
        return 1.0; // Вне shadow map - нет тени
    }
    
    let current_depth = proj_coords.z - bias;
    
    // PCF 3x3
    var shadow = 0.0;
    let offsets = array<vec2<f32>, 9>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 0.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  0.0),
        vec2<f32>( 0.0,  0.0),
        vec2<f32>( 1.0,  0.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 0.0,  1.0),
        vec2<f32>( 1.0,  1.0),
    );
    
    for (var i = 0; i < 9; i = i + 1) {
        let offset = offsets[i] * texel_size;
        let sample_pos = vec2<f32>(proj_coords.x + offset.x, proj_coords.y + offset.y);
        shadow += textureSampleCompareLevel(
            shadow_map,
            shadow_sampler,
            sample_pos,
            i32(cascade),
            current_depth,
        );
    }
    
    return shadow / 9.0;
}

// Быстрое сэмплирование без PCF (для дальних каскадов)
fn sample_shadow_simple(
    shadow_map: texture_depth_2d_array,
    shadow_sampler: sampler_comparison,
    world_pos: vec3<f32>,
    light_matrix: mat4x4<f32>,
    cascade: u32,
    bias: f32,
) -> f32 {
    let light_space = light_matrix * vec4<f32>(world_pos, 1.0);
    var proj_coords = light_space.xyz / light_space.w;
    
    proj_coords.x = proj_coords.x * 0.5 + 0.5;
    proj_coords.y = proj_coords.y * -0.5 + 0.5;
    
    if (proj_coords.x < 0.0 || proj_coords.x > 1.0 ||
        proj_coords.y < 0.0 || proj_coords.y > 1.0 ||
        proj_coords.z < 0.0 || proj_coords.z > 1.0) {
        return 1.0;
    }
    
    return textureSampleCompareLevel(
        shadow_map,
        shadow_sampler,
        vec2<f32>(proj_coords.x, proj_coords.y),
        i32(cascade),
        proj_coords.z - bias,
    );
}

// Главная функция расчёта тени с выбором каскада
fn calculate_shadow(
    shadow_map: texture_depth_2d_array,
    shadow_sampler: sampler_comparison,
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    view_depth: f32,
    shadow_data: ShadowData,
) -> f32 {
    let cascade = select_cascade(view_depth, shadow_data.cascade_distances, shadow_data.num_cascades);
    let light_matrix = get_light_matrix(cascade, shadow_data);
    
    // PCF для ближних каскадов, простое сэмплирование для дальних
    if (cascade < 2u) {
        return sample_shadow_pcf(
            shadow_map,
            shadow_sampler,
            world_pos,
            normal,
            light_matrix,
            cascade,
            shadow_data.bias,
            shadow_data.texel_size,
        );
    } else {
        return sample_shadow_simple(
            shadow_map,
            shadow_sampler,
            world_pos,
            light_matrix,
            cascade,
            shadow_data.bias,
        );
    }
}

// Визуализация каскадов для отладки
fn debug_cascade_color(cascade: u32) -> vec3<f32> {
    switch (cascade) {
        case 0u: { return vec3<f32>(1.0, 0.0, 0.0); } // Красный
        case 1u: { return vec3<f32>(0.0, 1.0, 0.0); } // Зелёный
        case 2u: { return vec3<f32>(0.0, 0.0, 1.0); } // Синий
        default: { return vec3<f32>(1.0, 1.0, 0.0); } // Жёлтый
    }
}
