// ============================================
// Player Model Shader
// ============================================
// Шейдер для рендеринга модели игрока в режиме 3-го лица

struct Uniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
}

struct ModelMatrix {
    model: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var<uniform> model: ModelMatrix;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Применяем матрицу модели (позиция + поворот игрока)
    let world_pos = model.model * vec4<f32>(in.position, 1.0);
    
    out.clip_position = uniforms.view_proj * world_pos;
    out.world_pos = world_pos.xyz;
    
    // Трансформируем нормаль (только поворот, без масштаба)
    let normal_matrix = mat3x3<f32>(
        model.model[0].xyz,
        model.model[1].xyz,
        model.model[2].xyz
    );
    out.normal = normalize(normal_matrix * in.normal);
    out.color = in.color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Направление света (солнце)
    let light_dir = normalize(vec3<f32>(0.4, 0.8, 0.3));
    let ndotl = max(dot(in.normal, light_dir), 0.0);
    
    // Освещение граней (как у блоков)
    var face_light = 1.0;
    if (in.normal.y > 0.5) { 
        face_light = 1.0; 
    } else if (abs(in.normal.x) > 0.5) { 
        face_light = 0.8; 
    } else { 
        face_light = 0.7; 
    }
    
    // Ambient + diffuse
    let ambient = 0.4;
    let diffuse = ndotl * 0.6;
    let lighting = (ambient + diffuse) * face_light;
    
    var color = in.color * lighting;
    
    // Лёгкий rim light для выделения силуэта
    let view_dir = normalize(uniforms.camera_pos - in.world_pos);
    let rim = 1.0 - max(dot(view_dir, in.normal), 0.0);
    let rim_intensity = pow(rim, 3.0) * 0.15;
    color += vec3<f32>(rim_intensity);
    
    // Туман (как у террейна)
    let dist = length(in.world_pos.xz - uniforms.camera_pos.xz);
    let fog_color = vec3<f32>(0.7, 0.8, 0.9);
    let fog_factor = smoothstep(800.0, 1000.0, dist);
    
    color = mix(color, fog_color, fog_factor);
    
    return vec4<f32>(color, 1.0);
}
