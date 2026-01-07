// ============================================
// Text Renderer - Рендеринг текста через wgpu_text
// ============================================

use wgpu_text::glyph_brush::{
    ab_glyph::FontRef, Section, Text,
};
use wgpu_text::BrushBuilder;

/// Выравнивание текста
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// Параметры текста для рендеринга
#[derive(Clone)]
pub struct TextParams {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub size: f32,
    pub color: [f32; 4],
    pub align: TextAlign,
    pub max_width: Option<f32>,
}

impl TextParams {
    pub fn new(text: &str, x: f32, y: f32, size: f32) -> Self {
        Self {
            x,
            y,
            text: text.to_string(),
            size,
            color: [1.0, 1.0, 1.0, 1.0],
            align: TextAlign::Left,
            max_width: None,
        }
    }
    
    pub fn centered(text: &str, x: f32, y: f32, size: f32) -> Self {
        Self {
            x,
            y,
            text: text.to_string(),
            size,
            color: [1.0, 1.0, 1.0, 1.0],
            align: TextAlign::Center,
            max_width: None,
        }
    }
    
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

/// GPU рендерер текста
pub struct TextRenderer {
    brush: wgpu_text::TextBrush<FontRef<'static>>,
    screen_width: u32,
    screen_height: u32,
}

impl TextRenderer {
    pub fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        // Загружаем шрифт
        let font_data: &'static [u8] = include_bytes!("../../../assets/fonts/Roboto-Regular.ttf");
        let font = FontRef::try_from_slice(font_data).expect("Failed to load font");
        
        let brush = BrushBuilder::using_font(font)
            .build(device, width, height, format);
        
        Self {
            brush,
            screen_width: width,
            screen_height: height,
        }
    }
    
    pub fn resize(&mut self, queue: &wgpu::Queue, width: u32, height: u32) {
        self.screen_width = width;
        self.screen_height = height;
        self.brush.resize_view(width as f32, height as f32, queue);
    }
    
    /// Подготовить и отрендерить текст
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        texts: &[TextParams],
    ) {
        if texts.is_empty() {
            return;
        }
        
        // Собираем все секции
        let sections: Vec<Section> = texts.iter().map(|params| {
            // Вычисляем ширину текста приблизительно
            let approx_width = params.text.chars().count() as f32 * params.size * 0.5;
            
            // Корректируем позицию в зависимости от выравнивания
            let pos_x = match params.align {
                TextAlign::Center => params.x - approx_width / 2.0,
                TextAlign::Right => params.x - approx_width,
                TextAlign::Left => params.x,
            };
            
            Section::default()
                .add_text(
                    Text::new(&params.text)
                        .with_scale(params.size)
                        .with_color(params.color),
                )
                .with_screen_position((pos_x, params.y))
        }).collect();
        
        // Передаём все секции одним вызовом
        self.brush.queue(device, queue, sections).unwrap();
        
        // Рендерим
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Text Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            self.brush.draw(&mut render_pass);
        }
    }
}
