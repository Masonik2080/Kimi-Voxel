// ============================================
// Celestial Bodies Shader - Солнце и Луна
// ============================================

struct CelestialUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,        // xyz + pad
    sun_direction: vec4<f32>,     // xyz + visibility
    sun_color: vec4<f32>,         // rgb + size
    moon_direction: vec4<f32>,    // xyz + visibility
    moon_color: vec4<f32>,        // rgb + phase
    time_of_day: vec4<f32>,       // time + pad
}

@group(0) @binding(0)
var<uniform> uniforms: CelestialUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) instance: u32,
}

const SKY_DISTANCE: f32 = 500.0;

@vertex
fn vs_main(in: VertexInput, @builtin(instance_index) instance: u32) -> VertexOutput {
    var out: VertexOutput;
    
    var direction: vec3<f32>;
    var size: f32;
    var visibility: f32;
    
    if (instance == 0u) {
        direction = uniforms.sun_direction.xyz;
        size = uniforms.sun_color.w;
        visibility = uniforms.sun_direction.w;
    } else {
        direction = uniforms.moon_direction.xyz;
        size = uniforms.sun_color.w * 0.8;
        visibility = uniforms.moon_direction.w;
    }
    
    if (visibility < 0.01) {
        out.clip_position = vec4<f32>(0.0, 0.0, -2.0, 1.0);
        out.uv = in.uv;
        out.instance = instance;
        return out;
    }
    
    let center = uniforms.camera_pos.xyz + direction * SKY_DISTANCE;
    let to_camera = normalize(uniforms.camera_pos.xyz - center);
    
    var up = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(dot(to_camera, up)) > 0.99) {
        up = vec3<f32>(0.0, 0.0, 1.0);
    }
    let right = normalize(cross(up, to_camera));
    let billboard_up = normalize(cross(to_camera, right));
    
    let scale = SKY_DISTANCE * size;
    let offset = right * in.position.x * scale + billboard_up * in.position.y * scale;
    let world_pos = center + offset;
    
    out.clip_position = uniforms.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = in.uv;
    out.instance = instance;
    
    return out;
}


fn draw_sun(uv: vec2<f32>) -> vec4<f32> {
    let center = vec2<f32>(0.5, 0.5);
    let dist = length(uv - center);
    
    let core_radius = 0.2;
    let glow_radius = 0.5;
    
    let core = smoothstep(core_radius, core_radius * 0.5, dist);
    let glow = smoothstep(glow_radius, core_radius, dist) * 0.6;
    
    let angle = atan2(uv.y - 0.5, uv.x - 0.5);
    let rays = (sin(angle * 12.0) * 0.5 + 0.5) * 0.3;
    let ray_intensity = rays * smoothstep(glow_radius, core_radius * 1.5, dist) * smoothstep(0.0, core_radius * 2.0, dist);
    
    let sun_core_color = vec3<f32>(1.0, 1.0, 0.95);
    let sun_glow_color = uniforms.sun_color.xyz;
    let sun_ray_color = vec3<f32>(1.0, 0.8, 0.4);
    
    var color = sun_core_color * core;
    color += sun_glow_color * glow;
    color += sun_ray_color * ray_intensity;
    
    let alpha = (core + glow + ray_intensity * 0.5) * uniforms.sun_direction.w;
    
    return vec4<f32>(color, alpha);
}

fn draw_moon(uv: vec2<f32>) -> vec4<f32> {
    let center = vec2<f32>(0.5, 0.5);
    let dist = length(uv - center);
    
    let moon_radius = 0.35;
    let glow_radius = 0.45;
    
    let moon_disk = smoothstep(moon_radius, moon_radius - 0.02, dist);
    let glow = smoothstep(glow_radius, moon_radius, dist) * 0.3;
    
    let phase = uniforms.moon_color.w;
    let phase_offset = (phase - 0.5) * 2.0;
    let shadow_center = vec2<f32>(0.5 + phase_offset * 0.4, 0.5);
    let shadow_dist = length(uv - shadow_center);
    let shadow = smoothstep(moon_radius * 0.9, moon_radius * 0.7, shadow_dist);
    
    var phase_factor = 1.0;
    if (phase < 0.5) {
        phase_factor = 1.0 - shadow * (1.0 - phase * 2.0);
    } else {
        phase_factor = 1.0 - shadow * ((phase - 0.5) * 2.0);
    }
    
    let crater_noise = crater_pattern(uv * 8.0);
    let surface_detail = 1.0 - crater_noise * 0.15;
    
    let moon_color = uniforms.moon_color.xyz * surface_detail;
    
    var color = moon_color * moon_disk * phase_factor;
    color += uniforms.moon_color.xyz * 0.5 * glow;
    
    let alpha = (moon_disk * phase_factor + glow) * uniforms.moon_direction.w;
    
    return vec4<f32>(color, alpha);
}

fn crater_pattern(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    
    var min_dist = 1.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let neighbor = vec2<f32>(f32(x), f32(y));
            let point = hash2d(i + neighbor);
            let diff = neighbor + point - f;
            let dist = length(diff);
            min_dist = min(min_dist, dist);
        }
    }
    return smoothstep(0.0, 0.3, min_dist);
}

fn hash2d(p: vec2<f32>) -> vec2<f32> {
    let k = vec2<f32>(0.3183099, 0.3678794);
    var q = p * k + k.yx;
    return fract(16.0 * k * fract(q.x * q.y * (q.x + q.y))) * 2.0 - 1.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (in.instance == 0u) {
        return draw_sun(in.uv);
    } else {
        return draw_moon(in.uv);
    }
}
