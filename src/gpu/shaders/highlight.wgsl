// ============================================
// Block Highlight Shader - Выделение блока
// ============================================

struct Uniforms {
    view_proj: mat4x4<f32>,
    block_pos: vec3<f32>,
    block_size: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Масштабируем вершины по размеру блока и смещаем на позицию
    let scaled_pos = in.position * uniforms.block_size;
    let world_pos = scaled_pos + uniforms.block_pos;
    out.clip_position = uniforms.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = in.color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
