// ============================================
// Hotbar Shader - Hi-Tech glassmorphism style
// Cyan neon accents + blur effect + animations
// ============================================

struct HotbarUniforms {
    screen_size: vec2<f32>,
    time: f32,
    selected_slot: f32,
}

@group(0) @binding(0) var<uniform> uniforms: HotbarUniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct InstanceInput {
    @location(1) pos: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) slot_index: u32,
    @location(4) is_selected: u32,
    @location(5) has_item: u32,
    @location(6) top_color: vec4<f32>,
    @location(7) side_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) size: vec2<f32>,
    @location(2) @interpolate(flat) slot_index: u32,
    @location(3) @interpolate(flat) is_selected: u32,
    @location(4) @interpolate(flat) has_item: u32,
    @location(5) @interpolate(flat) top_color: vec4<f32>,
    @location(6) @interpolate(flat) side_color: vec4<f32>,
    @location(7) world_pos: vec2<f32>,
}

// Цветовая палитра Hi-Tech
const ACCENT: vec3<f32> = vec3<f32>(0.0, 0.953, 1.0);           // #00f3ff - cyan
const BG_DARK: vec4<f32> = vec4<f32>(0.0, 0.039, 0.078, 0.4);   // rgba(0, 10, 20, 0.4)
const SLOT_BG: vec4<f32> = vec4<f32>(0.0, 0.078, 0.118, 0.6);   // rgba(0, 20, 30, 0.6)
const BORDER_COLOR: vec3<f32> = vec3<f32>(0.0, 1.0, 1.0);       // Cyan border

@vertex
fn vs_main(in: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    
    let pixel_pos = inst.pos + in.position * inst.size;
    let ndc_x = (pixel_pos.x / uniforms.screen_size.x) * 2.0 - 1.0;
    let ndc_y = (1.0 - pixel_pos.y / uniforms.screen_size.y) * 2.0 - 1.0;
    
    out.clip_pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = in.position;
    out.size = inst.size;
    out.slot_index = inst.slot_index;
    out.is_selected = inst.is_selected;
    out.has_item = inst.has_item;
    out.top_color = inst.top_color;
    out.side_color = inst.side_color;
    out.world_pos = pixel_pos;
    
    return out;
}

// SDF для скруглённого прямоугольника
fn sdf_rounded_rect(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p - size * 0.5) - size * 0.5 + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// SDF для скошенного угла (clip-path эффект)
fn sdf_clipped_rect(p: vec2<f32>, size: vec2<f32>, clip_size: f32) -> f32 {
    // Основной прямоугольник
    let rect_d = sdf_rounded_rect(p, size, 0.0);
    
    // Скошенный угол в правом нижнем углу
    let corner = vec2<f32>(size.x - clip_size, size.y);
    let to_corner = p - corner;
    let clip_d = to_corner.x + to_corner.y - clip_size;
    
    return max(rect_d, clip_d);
}

// Простой шум
fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// Процедурная текстура блока (точки, вариации)
fn block_texture(uv: vec2<f32>, base_color: vec3<f32>, seed: u32) -> f32 {
    var variation = 0.0;
    
    // Пиксельная сетка 8x8
    let pixel_size = 0.125;
    let pixel_uv = floor(uv / pixel_size);
    let pixel_seed = pixel_uv + vec2<f32>(f32(seed) * 7.3, f32(seed) * 3.7);
    
    let noise_val = hash(pixel_seed * 0.17);
    
    // Светлые и тёмные точки
    if (noise_val > 0.82) {
        variation = 0.08;
    } else if (noise_val < 0.18) {
        variation = -0.06;
    }
    
    // Дополнительный мелкий шум
    let fine_noise = hash(uv * 23.0 + vec2<f32>(f32(seed))) * 0.04 - 0.02;
    variation += fine_noise;
    
    return variation;
}

// Glow эффект
fn glow(d: f32, intensity: f32, spread: f32) -> f32 {
    return intensity / (1.0 + abs(d) * spread);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let px = in.uv * in.size;
    let w = in.size.x;
    let h = in.size.y;
    let time = uniforms.time;
    
    // ========== BACKGROUND (slot_index == 99) ==========
    if (in.slot_index == 99u) {
        let radius = 8.0;
        let d = sdf_rounded_rect(px, in.size, radius);
        
        // Отсекаем за пределами
        if (d > 0.5) {
            discard;
        }
        
        // Glassmorphism фон
        var color = BG_DARK;
        
        // Тонкая рамка сверху (декоративная полоска)
        if (d > -2.0 && d <= 0.0) {
            let border_alpha = 1.0 - (-d / 2.0);
            color = mix(color, vec4<f32>(BORDER_COLOR * 0.2, 0.2), border_alpha * 0.5);
        }
        
        // Градиентная полоска сверху по центру
        let center_x = w * 0.5;
        let stripe_width = w * 0.6;
        let dist_from_center = abs(px.x - center_x);
        if (px.y < 2.0 && dist_from_center < stripe_width * 0.5) {
            let stripe_alpha = 1.0 - dist_from_center / (stripe_width * 0.5);
            let stripe_y_alpha = 1.0 - px.y / 2.0;
            color = mix(color, vec4<f32>(ACCENT, 0.8), stripe_alpha * stripe_y_alpha * 0.7);
        }
        
        return color;
    }
    
    // ========== SLOT ==========
    let clip_size = h * 0.15; // 15% скос угла
    let d = sdf_clipped_rect(px, in.size, clip_size);
    
    // Отсекаем за пределами формы
    if (d > 0.5) {
        discard;
    }
    
    var color: vec4<f32>;
    
    // ========== SELECTED SLOT ==========
    if (in.is_selected == 1u) {
        // Яркий фон для выбранного слота
        color = vec4<f32>(ACCENT * 0.15, 0.15);
        
        // Яркая рамка
        if (d > -2.5) {
            let border_alpha = 1.0 - (-d / 2.5);
            color = mix(color, vec4<f32>(ACCENT, 0.8), border_alpha);
        }
        
        // Внутреннее свечение
        let inner_glow = glow(d, 0.2, 0.05);
        color.r += ACCENT.r * inner_glow * 0.3;
        color.g += ACCENT.g * inner_glow * 0.3;
        color.b += ACCENT.b * inner_glow * 0.3;
        
        // Внешнее свечение (glow)
        let outer_glow = glow(d + 5.0, 0.6, 0.1);
        color.r += ACCENT.r * outer_glow * 0.2;
        color.g += ACCENT.g * outer_glow * 0.2;
        color.b += ACCENT.b * outer_glow * 0.2;
        
        // Пульсация
        let pulse = sin(time * 3.0) * 0.1 + 0.9;
        color.r *= pulse;
        color.g *= pulse;
        color.b *= pulse;
        
    } else {
        // ========== NORMAL SLOT ==========
        color = SLOT_BG;
        
        // Тонкая рамка
        if (d > -1.5) {
            let border_alpha = 1.0 - (-d / 1.5);
            color = mix(color, vec4<f32>(BORDER_COLOR * 0.3, 0.3), border_alpha);
        }
    }
    
    // ========== ITEM RENDERING - 3D ISOMETRIC CUBE WITH TEXTURE ==========
    if (in.has_item == 1u) {
        let center = in.size * 0.5;
        
        // Локальные координаты относительно центра
        let p = px - center;
        
        let top_col = in.top_color.rgb;
        let side_col = in.side_color.rgb;
        
        // Размеры куба
        let cube_w = min(w, h) * 0.30;
        let cube_h = cube_w * 0.5;
        let cube_d = cube_w * 1.1;  // Высота боковых граней
        
        // Центрирование: куб идёт от -cube_h (верх ромба) до cube_h + cube_d (низ)
        // Общая высота = cube_h + cube_h + cube_d = 2*cube_h + cube_d
        // Центр должен быть на 0, значит смещаем на половину высоты минус cube_h
        let total_h = cube_h + cube_d;
        let offset_y = -total_h * 0.5 + cube_h * 0.5;
        let p_off = vec2<f32>(p.x, p.y - offset_y);
        
        var drawn = false;
        
        // === ВЕРХНЯЯ ГРАНЬ (ромб) ===
        let rhombus_check = abs(p_off.x) / cube_w + abs(p_off.y) / cube_h;
        
        if (rhombus_check < 1.0 && p_off.y < cube_h) {
            // UV координаты для верхней грани
            let uv = vec2<f32>(
                (p_off.x / cube_w + 1.0) * 0.5,
                (p_off.y / cube_h + 1.0) * 0.5
            );
            
            // Текстурные детали
            var tex_var = block_texture(uv, top_col, in.slot_index);
            
            // Обводка по краям ромба
            let edge_dist = 1.0 - rhombus_check;
            if (edge_dist < 0.15) {
                tex_var -= 0.12 * (1.0 - edge_dist / 0.15);
            }
            
            let lit_top = top_col * (1.1 + tex_var);
            color = vec4<f32>(min(lit_top, vec3<f32>(1.0)), 1.0);
            drawn = true;
        }
        
        // === ЛЕВАЯ ГРАНЬ ===
        if (p_off.x <= 0.0 && !drawn) {
            let t = -p_off.x / cube_w;
            let y_top_line = cube_h * (1.0 - t);
            let y_bottom_line = cube_h + cube_d - cube_h * t;
            
            if (p_off.y >= y_top_line && p_off.y <= y_bottom_line && p_off.x >= -cube_w) {
                // UV для левой грани
                let uv = vec2<f32>(
                    t,
                    (p_off.y - y_top_line) / (y_bottom_line - y_top_line)
                );
                
                var tex_var = block_texture(uv, side_col, in.slot_index + 10u);
                
                // Обводка
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
        
        // === ПРАВАЯ ГРАНЬ ===
        if (p_off.x >= 0.0 && !drawn) {
            let t = p_off.x / cube_w;
            let y_top_line = cube_h * (1.0 - t);
            let y_bottom_line = cube_h + cube_d - cube_h * t;
            
            if (p_off.y >= y_top_line && p_off.y <= y_bottom_line && p_off.x <= cube_w) {
                // UV для правой грани
                let uv = vec2<f32>(
                    t,
                    (p_off.y - y_top_line) / (y_bottom_line - y_top_line)
                );
                
                var tex_var = block_texture(uv, side_col, in.slot_index + 20u);
                
                // Обводка
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
    
    // ========== KEY BIND NUMBER ==========
    // Рисуем цифру в левом верхнем углу
    let key_num = in.slot_index + 1u;
    let digit_pos = vec2<f32>(8.0, 8.0);
    let digit_size = 10.0;
    
    // Простое отображение цифры через SDF (упрощённо - точка)
    let digit_center = digit_pos + vec2<f32>(digit_size * 0.5, digit_size * 0.5);
    let digit_dist = length(px - digit_center);
    
    // Подсветка области цифры (текст будет рендериться отдельно)
    if (digit_dist < digit_size) {
        let digit_alpha = 0.1 * (1.0 - digit_dist / digit_size);
        color = mix(color, vec4<f32>(ACCENT, 0.3), digit_alpha);
    }
    
    return color;
}
