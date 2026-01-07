// ============================================
// Terrain Shader with Texture Atlas Support
// ============================================

struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Текстурный атлас (опционально)
@group(1) @binding(0)
var atlas_texture: texture_2d<f32>;
@group(1) @binding(1)
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
    @location(3) block_id: u32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.world_pos = in.position;
    out.normal = in.normal;
    out.color = in.color;
    out.block_id = in.block_id;
    
    return out;
}

// Константы атласа
const ATLAS_SIZE: f32 = 16.0;  // 16x16 блоков в атласе
const TEXTURE_SIZE: f32 = 16.0; // 16x16 пикселей на текстуру

// Хеш для процедурного шума
fn hash2(p: vec2<f32>) -> f32 {
    let k = vec2<f32>(0.3183099, 0.3678794);
    let q = p * k + k.yx;
    return fract(16.0 * k.x * fract(q.x * q.y * (q.x + q.y)));
}

// Получить UV координаты на грани блока
fn get_block_uv(world_pos: vec3<f32>, normal: vec3<f32>) -> vec2<f32> {
    if (abs(normal.y) > 0.5) {
        return fract(world_pos.xz);
    } else if (abs(normal.x) > 0.5) {
        return fract(vec2<f32>(world_pos.z, world_pos.y));
    } else {
        return fract(vec2<f32>(world_pos.x, world_pos.y));
    }
}

// Получить UV в атласе для блока
fn get_atlas_uv(block_id: u32, local_uv: vec2<f32>) -> vec2<f32> {
    let atlas_x = f32(block_id % u32(ATLAS_SIZE));
    let atlas_y = f32(block_id / u32(ATLAS_SIZE));
    
    let u = (atlas_x + local_uv.x) / ATLAS_SIZE;
    let v = (atlas_y + local_uv.y) / ATLAS_SIZE;
    
    return vec2<f32>(u, v);
}

// Проверить, кастомный ли блок (ID >= 100)
fn is_custom_block(block_id: u32) -> bool {
    return block_id >= 100u;
}

// Анти-муар функции
fn get_detail_fade(dist: f32) -> f32 {
    return 1.0 - smoothstep(15.0, 40.0, dist);
}

fn get_side_face_fade(dist: f32) -> f32 {
    return smoothstep(20.0, 60.0, dist);
}

fn get_edge_fade(dist: f32) -> f32 {
    return 1.0 - smoothstep(10.0, 35.0, dist);
}

// Процедурные текстуры для стандартных блоков
fn grass_texture(uv: vec2<f32>, world_pos: vec3<f32>, dist: f32) -> f32 {
    var variation = 0.0;
    let edge_fade = get_edge_fade(dist);
    let detail_fade = get_detail_fade(dist);
    
    let edge_width = 0.06;
    let edge_x = min(uv.x, 1.0 - uv.x);
    let edge_y = min(uv.y, 1.0 - uv.y);
    let edge_dist = min(edge_x, edge_y);
    let edge_factor = smoothstep(0.0, edge_width, edge_dist);
    variation = mix(-0.15, 0.0, edge_factor) * edge_fade;
    
    if (detail_fade > 0.01) {
        let pixel_size = 0.0625;
        let pixel_uv = floor(uv / pixel_size);
        let seed = pixel_uv + floor(world_pos.xz);
        let noise_val = hash2(seed * 0.1);
        
        if (noise_val > 0.85) {
            variation = variation + 0.08 * detail_fade;
        } else if (noise_val < 0.15) {
            variation = variation - 0.06 * detail_fade;
        }
    }
    
    return variation;
}

fn stone_texture(uv: vec2<f32>, world_pos: vec3<f32>, dist: f32) -> f32 {
    var variation = 0.0;
    let edge_fade = get_edge_fade(dist);
    let detail_fade = get_detail_fade(dist);
    
    let edge_width = 0.04;
    let edge_x = min(uv.x, 1.0 - uv.x);
    let edge_y = min(uv.y, 1.0 - uv.y);
    let edge_dist = min(edge_x, edge_y);
    let edge_factor = smoothstep(0.0, edge_width, edge_dist);
    variation = mix(-0.1, 0.0, edge_factor) * edge_fade;
    
    if (detail_fade > 0.01) {
        let pixel_size = 0.0625;
        let pixel_uv = floor(uv / pixel_size);
        let seed = pixel_uv + floor(world_pos.xz) * 2.3;
        let noise_val = hash2(seed * 0.17);
        
        if (noise_val > 0.75) {
            variation = variation + 0.06 * detail_fade;
        } else if (noise_val < 0.25) {
            variation = variation - 0.06 * detail_fade;
        }
    }
    
    return variation;
}

fn get_procedural_variation(base_color: vec3<f32>, uv: vec2<f32>, world_pos: vec3<f32>, dist: f32) -> f32 {
    // Трава
    if (base_color.g > 0.5 && base_color.r < 0.5) {
        return grass_texture(uv, world_pos, dist);
    }
    // Камень
    if (abs(base_color.r - base_color.g) < 0.1 && base_color.r > 0.4 && base_color.r < 0.7) {
        return stone_texture(uv, world_pos, dist);
    }
    // Остальные - минимальная обводка
    let edge_fade = get_edge_fade(dist);
    let edge_width = 0.04;
    let edge_x = min(uv.x, 1.0 - uv.x);
    let edge_y = min(uv.y, 1.0 - uv.y);
    let edge_dist = min(edge_x, edge_y);
    let edge_factor = smoothstep(0.0, edge_width, edge_dist);
    return mix(-0.06, 0.0, edge_factor) * edge_fade;
}

fn get_average_surface_color(base_color: vec3<f32>) -> vec3<f32> {
    if (base_color.r > 0.4 && base_color.g > 0.25 && base_color.g < 0.5) {
        return vec3<f32>(0.35, 0.55, 0.25);
    }
    if (abs(base_color.r - base_color.g) < 0.15 && base_color.r > 0.35) {
        return vec3<f32>(0.5, 0.5, 0.5);
    }
    return base_color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.4, 0.8, 0.3));
    let ndotl = max(dot(in.normal, light_dir), 0.0);
    
    let dist = length(in.world_pos - uniforms.camera_pos);
    let is_side_face = abs(in.normal.y) < 0.5;
    let side_fade = get_side_face_fade(dist);
    
    var face_light = 1.0;
    if (in.normal.y > 0.5) { 
        face_light = 1.0; 
    } else if (abs(in.normal.x) > 0.5) { 
        face_light = mix(0.85, 1.0, side_fade); 
    } else { 
        face_light = mix(0.75, 1.0, side_fade); 
    }
    
    let lighting = (0.4 + ndotl * 0.6) * face_light;
    
    let uv = get_block_uv(in.world_pos, in.normal);
    var color: vec3<f32>;
    
    // Кастомные блоки (ID >= 100) используют текстурный атлас
    if (is_custom_block(in.block_id) && in.block_id > 0u) {
        let atlas_uv = get_atlas_uv(in.block_id, uv);
        let tex_color = textureSample(atlas_texture, atlas_sampler, atlas_uv);
        color = tex_color.rgb * lighting;
    } else {
        // Стандартные блоки - процедурные текстуры
        var base_color = in.color;
        if (is_side_face) {
            let avg_color = get_average_surface_color(in.color);
            base_color = mix(in.color, avg_color, side_fade);
        }
        
        let tex_variation = get_procedural_variation(in.color, uv, in.world_pos, dist);
        color = base_color * (1.0 + tex_variation) * lighting;
    }
    
    // Туман
    let fog_color = vec3<f32>(0.7, 0.8, 0.9);
    let fog_factor = smoothstep(300.0, 600.0, dist);
    color = mix(color, fog_color, fog_factor);
    
    return vec4<f32>(color, 1.0);
}
