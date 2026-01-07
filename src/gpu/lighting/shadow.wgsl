// ============================================
// Shadow Pass Shader - Рендеринг в shadow map
// ============================================
// Минимальный шейдер только для записи глубины

struct ShadowUniforms {
    light_view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> shadow: ShadowUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = shadow.light_view_proj * vec4<f32>(in.position, 1.0);
    return out;
}

// Fragment shader не нужен - только depth write
