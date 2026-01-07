// ============================================
// Terrain Shader with CSM Shadows + Texture Atlas
// ============================================
// Полная версия с каскадными тенями, динамическим небом и текстурным атласом

struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
    sky_color: vec3<f32>,
    time_of_day: f32,
    fog_color: vec3<f32>,
    _pad: f32,
}

struct LightData {
    direction: vec3<f32>,
    intensity: f32,
    color: vec3<f32>,
    _padding: f32,
}

struct ShadowData {
    light_vp_0: mat4x4<f32>,
    light_vp_1: mat4x4<f32>,
    light_vp_2: mat4x4<f32>,
    light_vp_3: mat4x4<f32>,
    cascade_splits: vec4<f32>,
    num_cascades: u32,
    texel_size: f32,
    bias: f32,
    _pad: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var<uniform> light: LightData;

@group(2) @binding(0)
var shadow_map: texture_depth_2d_array;
@group(2) @binding(1)
var shadow_sampler: sampler_comparison;
@group(2) @binding(2)
var<uniform> shadow_data: ShadowData;

// Текстурный атлас для кастомных блоков (ID >= 100)
@group(3) @binding(0)
var atlas_texture: texture_2d<f32>;
@group(3) @binding(1)
var atlas_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) block_id: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) view_depth: f32,
    @location(4) block_id: u32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    let world_pos = vec4<f32>(in.position, 1.0);
    out.clip_position = uniforms.view_proj * world_pos;
    out.world_pos = in.position;
    out.normal = in.normal;
    out.color = in.color;
    out.block_id = in.block_id;
    
    // Расстояние от камеры для выбора каскада
    out.view_depth = length(in.position - uniforms.camera_pos);
    
    return out;
}

// === Shadow Sampling Functions ===

fn select_cascade(view_depth: f32) -> u32 {
    if (view_depth < shadow_data.cascade_splits.x) { return 0u; }
    if (view_depth < shadow_data.cascade_splits.y) { return 1u; }
    if (view_depth < shadow_data.cascade_splits.z) { return 2u; }
    return 3u;
}

fn get_light_matrix(cascade: u32) -> mat4x4<f32> {
    switch (cascade) {
        case 0u: { return shadow_data.light_vp_0; }
        case 1u: { return shadow_data.light_vp_1; }
        case 2u: { return shadow_data.light_vp_2; }
        default: { return shadow_data.light_vp_3; }
    }
}

fn sample_shadow_pcf(world_pos: vec3<f32>, normal: vec3<f32>, cascade: u32) -> f32 {
    let light_matrix = get_light_matrix(cascade);
    
    // Normal offset bias - сдвигаем позицию вдоль нормали
    let normal_offset = normal * 0.1 * (1.0 + f32(cascade) * 0.3);
    let biased_pos = world_pos + normal_offset;
    
    let light_space = light_matrix * vec4<f32>(biased_pos, 1.0);
    
    let ndc = light_space.xyz / light_space.w;
    
    let uv = vec2<f32>(
        ndc.x * 0.5 + 0.5,
        -ndc.y * 0.5 + 0.5
    );
    
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z < 0.0 || ndc.z > 1.0) {
        return 1.0;
    }
    
    let depth = ndc.z - 0.001;
    let texel_size = 1.0 / 2048.0;
    
    // Poisson disk - фиксированный паттерн для мягких теней без ряби
    let poisson = array<vec2<f32>, 16>(
        vec2<f32>(-0.94201624, -0.39906216),
        vec2<f32>(0.94558609, -0.76890725),
        vec2<f32>(-0.09418410, -0.92938870),
        vec2<f32>(0.34495938, 0.29387760),
        vec2<f32>(-0.91588581, 0.45771432),
        vec2<f32>(-0.81544232, -0.87912464),
        vec2<f32>(-0.38277543, 0.27676845),
        vec2<f32>(0.97484398, 0.75648379),
        vec2<f32>(0.44323325, -0.97511554),
        vec2<f32>(0.53742981, -0.47373420),
        vec2<f32>(-0.26496911, -0.41893023),
        vec2<f32>(0.79197514, 0.19090188),
        vec2<f32>(-0.24188840, 0.99706507),
        vec2<f32>(-0.81409955, 0.91437590),
        vec2<f32>(0.19984126, 0.78641367),
        vec2<f32>(0.14383161, -0.14100790)
    );
    
    var shadow = 0.0;
    let spread = 2.5 * texel_size;
    
    for (var i = 0; i < 16; i++) {
        let offset = poisson[i] * spread;
        shadow += textureSampleCompareLevel(
            shadow_map, shadow_sampler,
            uv + offset, i32(cascade), depth
        );
    }
    
    return shadow / 16.0;
}

fn calculate_shadow(world_pos: vec3<f32>, normal: vec3<f32>, view_depth: f32) -> f32 {
    if (shadow_data.num_cascades == 0u) {
        return 1.0;
    }
    
    // За пределами последнего каскада - нет теней
    let last_split = shadow_data.cascade_splits[shadow_data.num_cascades - 1u];
    if (view_depth > last_split) {
        return 1.0;
    }
    
    let cascade = select_cascade(view_depth);
    return sample_shadow_pcf(world_pos, normal, cascade);
}

// === Texture Functions ===

fn hash2(p: vec2<f32>) -> f32 {
    let k = vec2<f32>(0.3183099, 0.3678794);
    let q = p * k + k.yx;
    return fract(16.0 * k.x * fract(q.x * q.y * (q.x + q.y)));
}

fn get_block_uv(world_pos: vec3<f32>, normal: vec3<f32>) -> vec2<f32> {
    if (abs(normal.y) > 0.5) {
        return fract(world_pos.xz);
    } else if (abs(normal.x) > 0.5) {
        return fract(vec2<f32>(world_pos.z, world_pos.y));
    } else {
        return fract(vec2<f32>(world_pos.x, world_pos.y));
    }
}

fn get_texture_variation(base_color: vec3<f32>, uv: vec2<f32>, world_pos: vec3<f32>) -> f32 {
    let edge_width = 0.05;
    let edge_x = min(uv.x, 1.0 - uv.x);
    let edge_y = min(uv.y, 1.0 - uv.y);
    let edge_dist = min(edge_x, edge_y);
    
    if (edge_dist < edge_width) {
        return -0.15;
    }
    
    let pixel_size = 0.0625;
    let pixel_uv = floor(uv / pixel_size);
    let seed = pixel_uv + floor(world_pos.xz);
    let noise = hash2(seed * 0.1);
    
    if (noise > 0.85) { return 0.1; }
    if (noise < 0.15) { return -0.08; }
    return 0.0;
}

// === Texture Atlas Functions ===

const ATLAS_SIZE: f32 = 16.0;  // 16x16 блоков в атласе

// Получить UV в атласе для блока
fn get_atlas_uv(block_id: u32, local_uv: vec2<f32>) -> vec2<f32> {
    let atlas_x = f32(block_id % u32(ATLAS_SIZE));
    let atlas_y = f32(block_id / u32(ATLAS_SIZE));
    
    // Небольшой отступ от краёв чтобы избежать bleeding
    let padding = 0.001;
    let clamped_uv = clamp(local_uv, vec2<f32>(padding), vec2<f32>(1.0 - padding));
    
    let u = (atlas_x + clamped_uv.x) / ATLAS_SIZE;
    let v = (atlas_y + clamped_uv.y) / ATLAS_SIZE;
    
    return vec2<f32>(u, v);
}

// Проверить, кастомный ли блок (ID >= 100)
fn is_custom_block(block_id: u32) -> bool {
    return block_id >= 100u;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Направленное освещение
    let ndotl = max(dot(in.normal, -light.direction), 0.0);
    
    // Освещение граней (ambient occlusion стиль)
    var face_light = 1.0;
    if (in.normal.y > 0.5) { face_light = 1.0; }
    else if (abs(in.normal.x) > 0.5) { face_light = 0.75; }
    else { face_light = 0.6; }
    
    // Тени с normal offset bias
    let shadow = calculate_shadow(in.world_pos, in.normal, in.view_depth);
    
    // Финальное освещение
    let ambient = 0.3;
    let diffuse = ndotl * light.intensity * shadow;
    let lighting = (ambient + diffuse * 0.7) * face_light;
    
    // UV координаты на грани блока
    let uv = get_block_uv(in.world_pos, in.normal);
    
    var color: vec3<f32>;
    
    // Кастомные блоки (ID >= 100) используют текстурный атлас
    if (is_custom_block(in.block_id)) {
        let atlas_uv = get_atlas_uv(in.block_id, uv);
        let tex_color = textureSample(atlas_texture, atlas_sampler, atlas_uv);
        color = tex_color.rgb * lighting;
    } else {
        // Стандартные блоки - процедурные текстуры
        let tex_var = get_texture_variation(in.color, uv, in.world_pos);
        color = in.color * light.color * (1.0 + tex_var) * lighting;
    }
    
    // Туман с динамическим цветом
    let dist = length(in.world_pos.xz - uniforms.camera_pos.xz);
    let fog = smoothstep(800.0, 1000.0, dist);
    color = mix(color, uniforms.fog_color, fog);
    
    return vec4<f32>(color, 1.0);
}
