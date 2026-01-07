// ============================================
// Menu UI Shader - Hytale-style modern design
// Glassmorphism + Neon accents + Smooth animations
// ============================================

struct GlobalUniforms {
    view_proj: mat4x4<f32>,
    screen_size: vec2<f32>,
    time: f32,
    menu_state: f32, // 0: main, 1: settings
}

@group(0) @binding(0) var<uniform> global: GlobalUniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct InstanceInput {
    @location(1) pos: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) state: u32,          // 0: Normal, 1: Hover, 2: Primary, 3: Danger, 4: Panel, 5: Slider
    @location(4) extra: f32,          // Slider value (0-1) or animation progress
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) size: vec2<f32>,
    @location(2) @interpolate(flat) state: u32,
    @location(3) @interpolate(flat) extra: f32,
    @location(4) world_pos: vec2<f32>,
}

// Цветовая палитра Hytale
const ACCENT: vec3<f32> = vec3<f32>(0.0, 0.94, 1.0);           // #00f0ff - cyan
const BG_BLUR: vec4<f32> = vec4<f32>(0.039, 0.071, 0.11, 0.85); // rgba(10, 18, 28, 0.85)
const TEXT_MAIN: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
const TEXT_DIM: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
const PANEL_RADIUS: f32 = 24.0;
const ITEM_RADIUS: f32 = 14.0;

@vertex
fn vs_main(in: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    
    let pixel_pos = inst.pos + in.position * inst.size;
    let ndc_x = (pixel_pos.x / global.screen_size.x) * 2.0 - 1.0;
    let ndc_y = (1.0 - pixel_pos.y / global.screen_size.y) * 2.0 - 1.0;
    
    out.clip_pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = in.position;
    out.size = inst.size;
    out.state = inst.state;
    out.extra = inst.extra;
    out.world_pos = pixel_pos;
    
    return out;
}

// SDF для скруглённого прямоугольника
fn sdf_rounded_rect(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p - size * 0.5) - size * 0.5 + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// Плавный шум для текстуры
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    
    let a = hash2d(i);
    let b = hash2d(i + vec2<f32>(1.0, 0.0));
    let c = hash2d(i + vec2<f32>(0.0, 1.0));
    let d = hash2d(i + vec2<f32>(1.0, 1.0));
    
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn hash2d(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
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
    let time = global.time;
    
    // Определяем радиус скругления в зависимости от типа элемента
    var radius = ITEM_RADIUS;
    if (in.state == 4u) { radius = PANEL_RADIUS; } // Panel
    
    // SDF расстояние до края
    let d = sdf_rounded_rect(px, in.size, radius);
    
    // Отсекаем пиксели за пределами скруглённого прямоугольника
    if (d > 0.5) {
        discard;
    }
    
    // ========== PANEL (state 4) - Основная панель меню ==========
    if (in.state == 4u) {
        // Glassmorphism фон
        var bg = BG_BLUR;
        
        // Тонкая светлая рамка
        let border_dist = abs(d + 1.0);
        if (border_dist < 1.5) {
            let border_alpha = 1.0 - border_dist / 1.5;
            bg = mix(bg, vec4<f32>(1.0, 1.0, 1.0, 0.15), border_alpha * 0.3);
        }
        
        // Лёгкий градиент сверху вниз
        let gradient = 1.0 - in.uv.y * 0.1;
        bg.r *= gradient;
        bg.g *= gradient;
        bg.b *= gradient;
        
        // Тонкий шум для текстуры стекла
        let n = noise(px * 0.1 + time * 0.5) * 0.02;
        bg.r += n;
        bg.g += n;
        bg.b += n;
        
        return bg;
    }
    
    // ========== PRIMARY BUTTON (state 2) - Акцентная кнопка ==========
    if (in.state == 2u) {
        var color = vec4<f32>(ACCENT, 1.0);
        
        // Hover эффект - подсветка
        let hover_glow = glow(d, 0.3, 0.1);
        color.r += hover_glow * 0.2;
        color.g += hover_glow * 0.2;
        color.b += hover_glow * 0.2;
        
        // Пульсация
        let pulse = sin(time * 3.0) * 0.05 + 0.95;
        color.r *= pulse;
        color.g *= pulse;
        color.b *= pulse;
        
        // Внутренний градиент
        let inner_gradient = 1.0 - in.uv.y * 0.2;
        color.r *= inner_gradient;
        color.g *= inner_gradient;
        color.b *= inner_gradient;
        
        // Glow по краям
        if (d > -3.0) {
            let edge_glow = 1.0 - (-d / 3.0);
            color = mix(color, vec4<f32>(1.0, 1.0, 1.0, 1.0), edge_glow * 0.3);
        }
        
        return color;
    }
    
    // ========== NORMAL BUTTON (state 0) ==========
    if (in.state == 0u) {
        // Полупрозрачный фон
        var color = vec4<f32>(1.0, 1.0, 1.0, 0.05);
        
        // Рамка
        if (d > -1.5) {
            let border_alpha = 1.0 - (-d / 1.5);
            color = mix(color, vec4<f32>(1.0, 1.0, 1.0, 0.15), border_alpha);
        }
        
        return color;
    }
    
    // ========== HOVER BUTTON (state 1) ==========
    if (in.state == 1u) {
        // Подсвеченный фон при наведении
        var color = vec4<f32>(1.0, 1.0, 1.0, 0.12);
        
        // Акцентная рамка
        if (d > -2.0) {
            let border_alpha = 1.0 - (-d / 2.0);
            color = mix(color, vec4<f32>(ACCENT, 0.8), border_alpha);
        }
        
        // Glow эффект
        let g = glow(d, 0.15, 0.08);
        color.r += ACCENT.r * g;
        color.g += ACCENT.g * g;
        color.b += ACCENT.b * g;
        
        // Сдвиг вправо (анимация) - имитируем через градиент
        let shift_gradient = smoothstep(0.0, 0.1, in.uv.x);
        color.a *= shift_gradient * 0.5 + 0.5;
        
        return color;
    }
    
    // ========== DANGER BUTTON (state 3) ==========
    if (in.state == 3u) {
        var color = vec4<f32>(1.0, 1.0, 1.0, 0.05);
        
        // Красноватая рамка
        let danger_color = vec3<f32>(1.0, 0.2, 0.2);
        if (d > -1.5) {
            let border_alpha = 1.0 - (-d / 1.5);
            color = mix(color, vec4<f32>(danger_color * 0.3, 0.3), border_alpha);
        }
        
        return color;
    }
    
    // ========== SLIDER (state 5) ==========
    if (in.state == 5u) {
        let slider_value = in.extra;
        
        // Трек слайдера (тонкая полоска по центру)
        let track_height = 4.0;
        let track_y = h * 0.5;
        let in_track = abs(px.y - track_y) < track_height * 0.5;
        
        // Фон трека
        var color = vec4<f32>(1.0, 1.0, 1.0, 0.1);
        
        if (in_track) {
            // Заполненная часть
            let fill_width = w * slider_value;
            if (px.x < fill_width) {
                let fill_progress = px.x / max(fill_width, 1.0);
                color = vec4<f32>(ACCENT * (0.7 + fill_progress * 0.3), 0.9);
            }
        }
        
        // Ползунок (thumb) - круглый
        let thumb_x = w * slider_value;
        let thumb_radius = 10.0;
        let thumb_center = vec2<f32>(clamp(thumb_x, thumb_radius, w - thumb_radius), h * 0.5);
        let thumb_dist = length(px - thumb_center);
        
        if (thumb_dist < thumb_radius + 2.0) {
            // Glow вокруг ползунка
            if (thumb_dist >= thumb_radius) {
                let glow_alpha = 1.0 - (thumb_dist - thumb_radius) / 2.0;
                color = mix(color, vec4<f32>(ACCENT, 0.5), glow_alpha * 0.5);
            } else {
                // Сам ползунок
                let inner_alpha = 1.0 - thumb_dist / thumb_radius;
                color = vec4<f32>(ACCENT, 0.9 + inner_alpha * 0.1);
                
                // Блик сверху
                if (px.y < thumb_center.y - thumb_radius * 0.3) {
                    color = mix(color, vec4<f32>(1.0, 1.0, 1.0, 1.0), 0.2);
                }
            }
        }
        
        return color;
    }
    
    // ========== SELECT/DROPDOWN (state 6) ==========
    if (in.state == 6u) {
        var color = vec4<f32>(1.0, 1.0, 1.0, 0.05);
        
        // Рамка
        if (d > -1.5) {
            let border_alpha = 1.0 - (-d / 1.5);
            color = mix(color, vec4<f32>(1.0, 1.0, 1.0, 0.15), border_alpha);
        }
        
        // Стрелка вниз (простой треугольник справа)
        let arrow_center = vec2<f32>(w - 20.0, h * 0.5);
        let arrow_size = 6.0;
        let ap = px - arrow_center;
        
        // Проверка попадания в треугольник
        if (ap.y > -arrow_size * 0.5 && ap.y < arrow_size * 0.5) {
            let expected_x = (arrow_size * 0.5 - abs(ap.y)) * 1.5;
            if (abs(ap.x) < expected_x) {
                color = vec4<f32>(TEXT_DIM * 0.5, 1.0);
            }
        }
        
        return color;
    }
    
    // ========== TITLE/HEADER (state 7) ==========
    if (in.state == 7u) {
        // Прозрачный фон, только для позиционирования текста
        // Акцентный цвет для заголовка
        var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        
        // Подчёркивание
        if (in.uv.y > 0.9) {
            let line_alpha = smoothstep(0.9, 1.0, in.uv.y);
            color = vec4<f32>(ACCENT * 0.5, line_alpha * 0.3);
        }
        
        return color;
    }
    
    // ========== OVERLAY (state 8) - Затемнение фона ==========
    if (in.state == 8u) {
        // Радиальный градиент затемнения
        let center = global.screen_size * 0.5;
        let dist_from_center = length(in.world_pos - center) / length(global.screen_size * 0.5);
        
        // Центр светлее, края темнее
        let darkness = mix(0.2, 0.8, dist_from_center);
        
        return vec4<f32>(0.0, 0.0, 0.0, darkness);
    }
    
    // Fallback
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}
