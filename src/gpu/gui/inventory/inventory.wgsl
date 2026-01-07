// ============================================
// Inventory Shader - Hi-Tech glassmorphism style
// Cyan neon accents + scrollable grid
// ============================================

struct InventoryUniforms {
    screen_size: vec2<f32>,
    time: f32,
    scroll: f32,
    panel_pos: vec2<f32>,
    panel_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: InventoryUniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct InstanceInput {
    @location(1) pos: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) slot_type: u32,
    @location(4) is_hovered: u32,
    @location(5) has_item: u32,
    @location(6) top_color: vec4<f32>,
    @location(7) side_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) size: vec2<f32>,
    @location(2) @interpolate(flat) slot_type: u32,
    @location(3) @interpolate(flat) is_hovered: u32,
    @location(4) @interpolate(flat) has_item: u32,
    @location(5) @interpolate(flat) top_color: vec4<f32>,
    @location(6) @interpolate(flat) side_color: vec4<f32>,
    @location(7) world_pos: vec2<f32>,
}

// Цветовая палитра Hi-Tech
const ACCENT: vec3<f32> = vec3<f32>(0.0, 0.953, 1.0);
const BG_DARK: vec4<f32> = vec4<f32>(0.02, 0.05, 0.08, 0.95);
const SLOT_BG: vec4<f32> = vec4<f32>(0.0, 0.078, 0.118, 0.7);
const BORDER_COLOR: vec3<f32> = vec3<f32>(0.0, 0.8, 1.0);

@vertex
fn vs_main(in: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    
    let pixel_pos = inst.pos + in.position * inst.size;
    let ndc_x = (pixel_pos.x / uniforms.screen_size.x) * 2.0 - 1.0;
    let ndc_y = (1.0 - pixel_pos.y / uniforms.screen_size.y) * 2.0 - 1.0;
    
    out.clip_pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = in.position;
    out.size = inst.size;
    out.slot_type = inst.slot_type;
    out.is_hovered = inst.is_hovered;
    out.has_item = inst.has_item;
    out.top_color = inst.top_color;
    out.side_color = inst.side_color;
    out.world_pos = pixel_pos;
    
    return out;
}

fn sdf_rounded_rect(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p - size * 0.5) - size * 0.5 + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn sdf_clipped_rect(p: vec2<f32>, size: vec2<f32>, clip_size: f32) -> f32 {
    let rect_d = sdf_rounded_rect(p, size, 0.0);
    let corner = vec2<f32>(size.x - clip_size, size.y);
    let to_corner = p - corner;
    let clip_d = to_corner.x + to_corner.y - clip_size;
    return max(rect_d, clip_d);
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn block_texture(uv: vec2<f32>, seed: u32) -> f32 {
    var variation = 0.0;
    let pixel_size = 0.125;
    let pixel_uv = floor(uv / pixel_size);
    let pixel_seed = pixel_uv + vec2<f32>(f32(seed) * 7.3, f32(seed) * 3.7);
    let noise_val = hash(pixel_seed * 0.17);
    
    if (noise_val > 0.82) {
        variation = 0.08;
    } else if (noise_val < 0.18) {
        variation = -0.06;
    }
    
    let fine_noise = hash(uv * 23.0 + vec2<f32>(f32(seed))) * 0.04 - 0.02;
    variation += fine_noise;
    
    return variation;
}

fn glow(d: f32, intensity: f32, spread: f32) -> f32 {
    return intensity / (1.0 + abs(d) * spread);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let px = in.uv * in.size;
    let w = in.size.x;
    let h = in.size.y;
    let time = uniforms.time;
    
    // ========== OVERLAY (slot_type == 0) ==========
    if (in.slot_type == 0u) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.7);
    }
    
    // ========== PANEL (slot_type == 1) ==========
    if (in.slot_type == 1u) {
        let radius = 12.0;
        let d = sdf_rounded_rect(px, in.size, radius);
        
        if (d > 0.5) {
            discard;
        }
        
        var color = BG_DARK;
        
        // Рамка
        if (d > -2.0) {
            let border_alpha = 1.0 - (-d / 2.0);
            color = mix(color, vec4<f32>(BORDER_COLOR * 0.4, 0.6), border_alpha);
        }
        
        // Градиент сверху
        let top_gradient = 1.0 - px.y / h;
        let accent_add = ACCENT * top_gradient * 0.02;
        color = vec4<f32>(color.rgb + accent_add, color.a);
        
        return color;
    }
    
    // ========== HEADER (slot_type == 5) ==========
    if (in.slot_type == 5u) {
        let radius = 12.0;
        let d = sdf_rounded_rect(px, vec2<f32>(w, h + radius), radius);
        
        if (d > 0.5 || px.y > h) {
            discard;
        }
        
        var color = vec4<f32>(0.0, 0.06, 0.1, 0.9);
        
        // Нижняя граница заголовка
        if (px.y > h - 2.0) {
            let line_alpha = (px.y - (h - 2.0)) / 2.0;
            color = mix(color, vec4<f32>(ACCENT, 0.8), line_alpha);
        }
        
        // Декоративная полоска сверху
        let center_x = w * 0.5;
        let stripe_width = w * 0.4;
        let dist_from_center = abs(px.x - center_x);
        if (px.y < 3.0 && dist_from_center < stripe_width * 0.5) {
            let stripe_alpha = 1.0 - dist_from_center / (stripe_width * 0.5);
            let stripe_y_alpha = 1.0 - px.y / 3.0;
            color = mix(color, vec4<f32>(ACCENT, 0.9), stripe_alpha * stripe_y_alpha * 0.8);
        }
        
        return color;
    }
    
    // ========== SCROLLBAR TRACK (slot_type == 3) ==========
    if (in.slot_type == 3u) {
        let radius = 6.0;
        let d = sdf_rounded_rect(px, in.size, radius);
        
        if (d > 0.5) {
            discard;
        }
        
        return vec4<f32>(0.0, 0.04, 0.06, 0.5);
    }
    
    // ========== SCROLLBAR THUMB (slot_type == 4) ==========
    if (in.slot_type == 4u) {
        let radius = 5.0;
        let d = sdf_rounded_rect(px, in.size, radius);
        
        if (d > 0.5) {
            discard;
        }
        
        var color = vec4<f32>(ACCENT * 0.6, 0.8);
        
        // Свечение
        if (d > -2.0) {
            let border_alpha = 1.0 - (-d / 2.0);
            color = mix(color, vec4<f32>(ACCENT, 1.0), border_alpha * 0.5);
        }
        
        return color;
    }
    
    // ========== SLOT (slot_type == 2) или DRAGGING (slot_type == 6) ==========
    let is_dragging = in.slot_type == 6u;
    let clip_size = h * 0.12;
    let d = sdf_clipped_rect(px, in.size, clip_size);
    
    if (d > 0.5) {
        discard;
    }
    
    var color: vec4<f32>;
    
    if (in.is_hovered == 1u || is_dragging) {
        // Подсвеченный слот
        color = vec4<f32>(ACCENT * 0.2, 0.3);
        
        if (d > -2.5) {
            let border_alpha = 1.0 - (-d / 2.5);
            color = mix(color, vec4<f32>(ACCENT, 0.9), border_alpha);
        }
        
        let inner_glow = glow(d, 0.3, 0.04);
        color = vec4<f32>(color.rgb + ACCENT * inner_glow * 0.4, color.a);
        
        let pulse = sin(time * 4.0) * 0.1 + 0.95;
        color = vec4<f32>(color.rgb * pulse, color.a);
    } else {
        // Обычный слот
        color = SLOT_BG;
        
        if (d > -1.5) {
            let border_alpha = 1.0 - (-d / 1.5);
            color = mix(color, vec4<f32>(BORDER_COLOR * 0.3, 0.4), border_alpha);
        }
    }
    
    // ========== ITEM RENDERING - 3D ISOMETRIC CUBE ==========
    if (in.has_item == 1u) {
        let center = in.size * 0.5;
        let p = px - center;
        
        let top_col = in.top_color.rgb;
        let side_col = in.side_color.rgb;
        
        let cube_w = min(w, h) * 0.32;
        let cube_h = cube_w * 0.5;
        let cube_d = cube_w * 1.1;
        
        let total_h = cube_h + cube_d;
        let offset_y = -total_h * 0.5 + cube_h * 0.5;
        let p_off = vec2<f32>(p.x, p.y - offset_y);
        
        var drawn = false;
        
        // Верхняя грань (ромб)
        let rhombus_check = abs(p_off.x) / cube_w + abs(p_off.y) / cube_h;
        
        if (rhombus_check < 1.0 && p_off.y < cube_h) {
            let uv = vec2<f32>(
                (p_off.x / cube_w + 1.0) * 0.5,
                (p_off.y / cube_h + 1.0) * 0.5
            );
            
            var tex_var = block_texture(uv, u32(in.world_pos.x * 0.1));
            
            let edge_dist = 1.0 - rhombus_check;
            if (edge_dist < 0.15) {
                tex_var -= 0.12 * (1.0 - edge_dist / 0.15);
            }
            
            let lit_top = top_col * (1.1 + tex_var);
            color = vec4<f32>(min(lit_top, vec3<f32>(1.0)), 1.0);
            drawn = true;
        }
        
        // Левая грань
        if (p_off.x <= 0.0 && !drawn) {
            let t = -p_off.x / cube_w;
            let y_top_line = cube_h * (1.0 - t);
            let y_bottom_line = cube_h + cube_d - cube_h * t;
            
            if (p_off.y >= y_top_line && p_off.y <= y_bottom_line && p_off.x >= -cube_w) {
                let uv = vec2<f32>(
                    t,
                    (p_off.y - y_top_line) / (y_bottom_line - y_top_line)
                );
                
                var tex_var = block_texture(uv, u32(in.world_pos.x * 0.1) + 10u);
                
                let edge_x = min(t, 1.0 - t);
                let edge_y = min(uv.y, 1.0 - uv.y);
                let edge = min(edge_x, edge_y);
                if (edge < 0.1) {
                    tex_var -= 0.1 * (1.0 - edge / 0.1);
                }
                
                let left_col = side_col * (0.8 + tex_var);
                color = vec4<f32>(left_col, 1.0);
                drawn = true;
            }
        }
        
        // Правая грань
        if (p_off.x >= 0.0 && !drawn) {
            let t = p_off.x / cube_w;
            let y_top_line = cube_h * (1.0 - t);
            let y_bottom_line = cube_h + cube_d - cube_h * t;
            
            if (p_off.y >= y_top_line && p_off.y <= y_bottom_line && p_off.x <= cube_w) {
                let uv = vec2<f32>(
                    t,
                    (p_off.y - y_top_line) / (y_bottom_line - y_top_line)
                );
                
                var tex_var = block_texture(uv, u32(in.world_pos.x * 0.1) + 20u);
                
                let edge_x = min(t, 1.0 - t);
                let edge_y = min(uv.y, 1.0 - uv.y);
                let edge = min(edge_x, edge_y);
                if (edge < 0.1) {
                    tex_var -= 0.1 * (1.0 - edge / 0.1);
                }
                
                let right_col = side_col * (0.55 + tex_var);
                color = vec4<f32>(right_col, 1.0);
                drawn = true;
            }
        }
    }
    
    return color;
}
