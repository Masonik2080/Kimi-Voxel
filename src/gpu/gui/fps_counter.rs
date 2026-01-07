// ============================================
// FPS Counter - Счётчик кадров в секунду
// ============================================
// Отображает FPS в левом верхнем углу экрана
// Использует 7-сегментный дисплей для цифр

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// Вершина для UI (2D позиция + цвет)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FpsVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

impl FpsVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<FpsVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// FPS Counter
pub struct FpsCounter {
    vertex_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    
    // FPS tracking
    frame_count: u32,
    last_fps_update: std::time::Instant,
    current_fps: u32,
    
    // Максимальное количество вершин (для 4 цифр + "FPS:" текст)
    max_vertices: u32,
    current_vertex_count: u32,
    
    queue: std::sync::Arc<wgpu::Queue>,
}

impl FpsCounter {
    pub fn new(device: &wgpu::Device, queue: std::sync::Arc<wgpu::Queue>, surface_format: wgpu::TextureFormat) -> Self {
        // Создаём буфер с запасом для 4 цифр (каждая цифра до 7 сегментов * 6 вершин)
        let max_vertices = 4 * 7 * 6 + 100; // Запас для "FPS:" текста
        
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FPS Counter Vertex Buffer"),
            size: (max_vertices as usize * std::mem::size_of::<FpsVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Шейдер для UI (тот же что и для crosshair)
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("FPS UI Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ui.wgsl").into()),
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("FPS Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("FPS Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[FpsVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        
        Self {
            vertex_buffer,
            pipeline,
            frame_count: 0,
            last_fps_update: std::time::Instant::now(),
            current_fps: 0,
            max_vertices,
            current_vertex_count: 0,
            queue,
        }
    }
    
    /// Вызывать каждый кадр для обновления счётчика
    pub fn update(&mut self) {
        self.frame_count += 1;
        
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_fps_update).as_secs_f32();
        
        // Обновляем FPS раз в секунду
        if elapsed >= 1.0 {
            self.current_fps = (self.frame_count as f32 / elapsed) as u32;
            self.frame_count = 0;
            self.last_fps_update = now;
            
            // Перестраиваем геометрию
            self.rebuild_geometry();
        }
    }
    
    fn rebuild_geometry(&mut self) {
        let mut vertices = Vec::new();
        
        // Позиция в левом верхнем углу (NDC: -1 to 1)
        let start_x = -0.95;
        let start_y = 0.90;
        let digit_width = 0.04;
        let digit_height = 0.07;
        let digit_spacing = 0.05;
        let segment_thickness = 0.008;
        
        let color = [1.0, 1.0, 0.0, 0.9]; // Жёлтый
        
        // Отображаем FPS число
        let fps_str = format!("{}", self.current_fps);
        let mut x = start_x;
        
        for ch in fps_str.chars() {
            if let Some(digit) = ch.to_digit(10) {
                self.add_digit(&mut vertices, x, start_y, digit_width, digit_height, segment_thickness, digit as u8, color);
            }
            x += digit_spacing;
        }
        
        self.current_vertex_count = vertices.len() as u32;
        
        if !vertices.is_empty() {
            self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }
    }
    
    /// Добавляет цифру в виде 7-сегментного дисплея
    fn add_digit(&self, vertices: &mut Vec<FpsVertex>, x: f32, y: f32, w: f32, h: f32, t: f32, digit: u8, color: [f32; 4]) {
        // Сегменты 7-сегментного дисплея:
        //  AAA
        // F   B
        //  GGG
        // E   C
        //  DDD
        
        let segments = match digit {
            0 => [true, true, true, true, true, true, false],    // ABCDEF
            1 => [false, true, true, false, false, false, false], // BC
            2 => [true, true, false, true, true, false, true],   // ABDEG
            3 => [true, true, true, true, false, false, true],   // ABCDG
            4 => [false, true, true, false, false, true, true],  // BCFG
            5 => [true, false, true, true, false, true, true],   // ACDFG
            6 => [true, false, true, true, true, true, true],    // ACDEFG
            7 => [true, true, true, false, false, false, false], // ABC
            8 => [true, true, true, true, true, true, true],     // ABCDEFG
            9 => [true, true, true, true, false, true, true],    // ABCDFG
            _ => [false; 7],
        };
        
        let half_h = h / 2.0;
        
        // A - верхний горизонтальный
        if segments[0] {
            self.add_horizontal_segment(vertices, x, y, w, t, color);
        }
        // B - правый верхний вертикальный
        if segments[1] {
            self.add_vertical_segment(vertices, x + w - t, y - t, half_h - t, t, color);
        }
        // C - правый нижний вертикальный
        if segments[2] {
            self.add_vertical_segment(vertices, x + w - t, y - half_h, half_h - t, t, color);
        }
        // D - нижний горизонтальный
        if segments[3] {
            self.add_horizontal_segment(vertices, x, y - h + t, w, t, color);
        }
        // E - левый нижний вертикальный
        if segments[4] {
            self.add_vertical_segment(vertices, x, y - half_h, half_h - t, t, color);
        }
        // F - левый верхний вертикальный
        if segments[5] {
            self.add_vertical_segment(vertices, x, y - t, half_h - t, t, color);
        }
        // G - средний горизонтальный
        if segments[6] {
            self.add_horizontal_segment(vertices, x, y - half_h + t / 2.0, w, t, color);
        }
    }
    
    fn add_horizontal_segment(&self, vertices: &mut Vec<FpsVertex>, x: f32, y: f32, w: f32, t: f32, color: [f32; 4]) {
        // Два треугольника для прямоугольника
        vertices.push(FpsVertex { position: [x, y], color });
        vertices.push(FpsVertex { position: [x + w, y], color });
        vertices.push(FpsVertex { position: [x + w, y - t], color });
        
        vertices.push(FpsVertex { position: [x, y], color });
        vertices.push(FpsVertex { position: [x + w, y - t], color });
        vertices.push(FpsVertex { position: [x, y - t], color });
    }
    
    fn add_vertical_segment(&self, vertices: &mut Vec<FpsVertex>, x: f32, y: f32, h: f32, t: f32, color: [f32; 4]) {
        vertices.push(FpsVertex { position: [x, y], color });
        vertices.push(FpsVertex { position: [x + t, y], color });
        vertices.push(FpsVertex { position: [x + t, y - h], color });
        
        vertices.push(FpsVertex { position: [x, y], color });
        vertices.push(FpsVertex { position: [x + t, y - h], color });
        vertices.push(FpsVertex { position: [x, y - h], color });
    }
    
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.current_vertex_count > 0 {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.current_vertex_count, 0..1);
        }
    }
}
