// ============================================
// Game Menu - Hytale-style GPU rendered menu
// Modern glassmorphism design with neon accents
// ============================================

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use std::time::Instant;

/// Состояние меню
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuState {
    Hidden,
    Main,
    Settings,
}

/// Действие из меню
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    None,
    Resume,
    Settings,
    BackToMain,
    SaveSettings,  // Сохранить настройки и применить LOD
    QuitToDesktop,
}

/// Тип элемента UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ElementType {
    Button = 0,
    ButtonHover = 1,
    ButtonPrimary = 2,
    ButtonDanger = 3,
    Panel = 4,
    Slider = 5,
    Select = 6,
    Title = 7,
    Overlay = 8,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MenuUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub screen_size: [f32; 2],
    pub time: f32,
    pub menu_state: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct MenuInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub state: u32,
    pub extra: f32,  // Для слайдеров - значение 0-1
}

pub struct UIElement {
    pub id: &'static str,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub element_type: ElementType,
    pub hover: bool,
    pub value: f32,  // Для слайдеров
    pub visible: bool,
}

impl UIElement {
    fn new_button(id: &'static str, label: &str, width: f32, height: f32) -> Self {
        Self {
            id,
            label: label.to_string(),
            x: 0.0,
            y: 0.0,
            width,
            height,
            element_type: ElementType::Button,
            hover: false,
            value: 0.0,
            visible: true,
        }
    }
    
    fn new_primary(id: &'static str, label: &str, width: f32, height: f32) -> Self {
        Self {
            id,
            label: label.to_string(),
            x: 0.0,
            y: 0.0,
            width,
            height,
            element_type: ElementType::ButtonPrimary,
            hover: false,
            value: 0.0,
            visible: true,
        }
    }
    
    fn new_danger(id: &'static str, label: &str, width: f32, height: f32) -> Self {
        Self {
            id,
            label: label.to_string(),
            x: 0.0,
            y: 0.0,
            width,
            height,
            element_type: ElementType::ButtonDanger,
            hover: false,
            value: 0.0,
            visible: true,
        }
    }
    
    fn new_slider(id: &'static str, label: &str, width: f32, initial: f32) -> Self {
        Self {
            id,
            label: label.to_string(),
            x: 0.0,
            y: 0.0,
            width,
            height: 20.0,  // Увеличенная высота для лучшей видимости
            element_type: ElementType::Slider,
            hover: false,
            value: initial,
            visible: true,
        }
    }
    
    fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.width && 
        my >= self.y && my <= self.y + self.height
    }
    
    fn get_state(&self) -> u32 {
        if self.hover && self.element_type == ElementType::Button {
            ElementType::ButtonHover as u32
        } else {
            self.element_type as u32
        }
    }
}

/// GPU-рендерер меню в стиле Hytale
pub struct MenuSystem {
    // UI элементы по экранам
    main_elements: Vec<UIElement>,
    settings_elements: Vec<UIElement>,
    
    // GPU ресурсы
    instance_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    
    // Состояние
    current_state: MenuState,
    screen_width: f32,
    screen_height: f32,
    start_time: Instant,
    
    // Панели
    panel_main: UIElement,
    panel_settings: UIElement,
    overlay: UIElement,
}

impl MenuSystem {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        _layout: &wgpu::BindGroupLayout,
        width: u32,
        height: u32,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Menu Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        
        let uniforms = MenuUniforms {
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            screen_size: [width as f32, height as f32],
            time: 0.0,
            menu_state: 0.0,
        };
        
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Menu Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Menu Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        
        // Квадрат (2 треугольника)
        let vertices: Vec<[f32; 2]> = vec![
            [0.0, 0.0], [1.0, 0.0], [1.0, 1.0],
            [0.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        ];
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Menu Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Menu Instance Buffer"),
            size: (std::mem::size_of::<MenuInstance>() * 50) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Menu Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("menu.wgsl").into()),
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Menu Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Menu Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: 8,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        }],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<MenuInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 1,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 8,
                                shader_location: 2,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Uint32,
                                offset: 16,
                                shader_location: 3,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32,
                                offset: 20,
                                shader_location: 4,
                            },
                        ],
                    },
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        
        // ========== Главное меню ==========
        let main_elements = vec![
            UIElement::new_primary("resume", "Back to Game", 380.0, 56.0),
            UIElement::new_button("settings", "Settings", 380.0, 56.0),
            UIElement::new_button("stats", "Statistics", 380.0, 56.0),
            UIElement::new_danger("quit", "Quit to Menu", 380.0, 56.0),
        ];
        
        // ========== Меню настроек ==========
        let settings_elements = vec![
            UIElement::new_slider("lod0", "LOD0", 160.0, 0.5),
            UIElement::new_slider("lod1", "LOD1", 160.0, 0.5),
            UIElement::new_slider("lod2", "LOD2", 160.0, 0.5),
            UIElement::new_slider("lod3", "LOD3", 160.0, 0.5),
            UIElement::new_primary("save", "Save", 380.0, 56.0),
            UIElement::new_button("back", "Back", 380.0, 56.0),
        ];
        
        // Панели
        let panel_main = UIElement {
            id: "panel_main",
            label: String::new(),
            x: 0.0,
            y: 0.0,
            width: 420.0,
            height: 380.0,
            element_type: ElementType::Panel,
            hover: false,
            value: 0.0,
            visible: true,
        };
        
        let panel_settings = UIElement {
            id: "panel_settings",
            label: String::new(),
            x: 0.0,
            y: 0.0,
            width: 420.0,
            height: 480.0,
            element_type: ElementType::Panel,
            hover: false,
            value: 0.0,
            visible: true,
        };
        
        let overlay = UIElement {
            id: "overlay",
            label: String::new(),
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
            element_type: ElementType::Overlay,
            hover: false,
            value: 0.0,
            visible: true,
        };
        
        let mut menu = Self {
            main_elements,
            settings_elements,
            instance_buffer,
            uniform_buffer,
            bind_group,
            pipeline,
            vertex_buffer,
            current_state: MenuState::Hidden,
            screen_width: width as f32,
            screen_height: height as f32,
            start_time: Instant::now(),
            panel_main,
            panel_settings,
            overlay,
        };
        
        menu.update_layout();
        menu
    }
    
    pub fn update_layout(&mut self) {
        let cx = self.screen_width / 2.0;
        let cy = self.screen_height / 2.0;
        
        // Overlay на весь экран
        self.overlay.width = self.screen_width;
        self.overlay.height = self.screen_height;
        
        // ========== Main Menu Layout ==========
        let panel_w = 420.0;
        let panel_h = 380.0;
        self.panel_main.x = cx - panel_w / 2.0;
        self.panel_main.y = cy - panel_h / 2.0;
        self.panel_main.width = panel_w;
        self.panel_main.height = panel_h;
        
        let start_y = self.panel_main.y + 80.0;
        let btn_spacing = 66.0;
        
        for (i, elem) in self.main_elements.iter_mut().enumerate() {
            elem.x = cx - elem.width / 2.0;
            elem.y = start_y + i as f32 * btn_spacing;
            
            // Отступ перед кнопкой выхода
            if elem.id == "quit" {
                elem.y += 20.0;
            }
        }
        
        // ========== Settings Menu Layout ==========
        let settings_h = 480.0;
        self.panel_settings.x = cx - panel_w / 2.0;
        self.panel_settings.y = cy - settings_h / 2.0;
        self.panel_settings.width = panel_w;
        self.panel_settings.height = settings_h;
        
        let settings_start_y = self.panel_settings.y + 100.0;
        let slider_spacing = 50.0;
        
        // LOD слайдеры в сетке 2x2
        let grid_left = self.panel_settings.x + 30.0;
        let grid_right = cx + 15.0;
        
        if self.settings_elements.len() >= 4 {
            // LOD0
            self.settings_elements[0].x = grid_left;
            self.settings_elements[0].y = settings_start_y;
            // LOD1
            self.settings_elements[1].x = grid_right;
            self.settings_elements[1].y = settings_start_y;
            // LOD2
            self.settings_elements[2].x = grid_left;
            self.settings_elements[2].y = settings_start_y + slider_spacing;
            // LOD3
            self.settings_elements[3].x = grid_right;
            self.settings_elements[3].y = settings_start_y + slider_spacing;
        }
        
        // Кнопки внизу
        let buttons_y = self.panel_settings.y + settings_h - 140.0;
        if self.settings_elements.len() >= 6 {
            self.settings_elements[4].x = cx - self.settings_elements[4].width / 2.0;
            self.settings_elements[4].y = buttons_y;
            
            self.settings_elements[5].x = cx - self.settings_elements[5].width / 2.0;
            self.settings_elements[5].y = buttons_y + 60.0;
        }
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.screen_width = width as f32;
        self.screen_height = height as f32;
        self.update_layout();
    }
    
    pub fn handle_mouse_move(&mut self, mx: f32, my: f32) {
        if self.current_state == MenuState::Hidden {
            return;
        }
        
        let elements = match self.current_state {
            MenuState::Main => &mut self.main_elements,
            MenuState::Settings => &mut self.settings_elements,
            MenuState::Hidden => return,
        };
        
        for elem in elements.iter_mut() {
            elem.hover = elem.contains(mx, my);
        }
    }
    
    pub fn handle_click(&mut self, mx: f32, my: f32) -> MenuAction {
        if self.current_state == MenuState::Hidden {
            return MenuAction::None;
        }
        
        match self.current_state {
            MenuState::Main => {
                for elem in &self.main_elements {
                    if elem.contains(mx, my) {
                        match elem.id {
                            "resume" => {
                                self.current_state = MenuState::Hidden;
                                return MenuAction::Resume;
                            }
                            "settings" => {
                                self.current_state = MenuState::Settings;
                                return MenuAction::Settings;
                            }
                            "quit" => {
                                return MenuAction::QuitToDesktop;
                            }
                            _ => {}
                        }
                    }
                }
            }
            MenuState::Settings => {
                for elem in &self.settings_elements {
                    if elem.contains(mx, my) {
                        match elem.id {
                            "save" => {
                                self.current_state = MenuState::Main;
                                return MenuAction::SaveSettings;
                            }
                            "back" => {
                                self.current_state = MenuState::Main;
                                return MenuAction::BackToMain;
                            }
                            _ => {}
                        }
                    }
                }
            }
            MenuState::Hidden => {}
        }
        
        MenuAction::None
    }
    
    /// Обработка перетаскивания слайдера
    pub fn handle_drag(&mut self, mx: f32, my: f32, pressed: bool) {
        if self.current_state != MenuState::Settings || !pressed {
            return;
        }
        
        for elem in &mut self.settings_elements {
            if elem.element_type == ElementType::Slider {
                // Расширенная зона для слайдера (по высоте)
                let slider_hit_height = 20.0;
                let in_y = my >= elem.y - slider_hit_height / 2.0 
                        && my <= elem.y + slider_hit_height / 2.0;
                let in_x = mx >= elem.x && mx <= elem.x + elem.width;
                
                if in_x && in_y {
                    elem.value = ((mx - elem.x) / elem.width).clamp(0.0, 1.0);
                }
            }
        }
    }
    
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, queue: &wgpu::Queue) {
        if self.current_state == MenuState::Hidden {
            return;
        }
        
        let time = self.start_time.elapsed().as_secs_f32();
        
        let uniforms = MenuUniforms {
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            screen_size: [self.screen_width, self.screen_height],
            time,
            menu_state: match self.current_state {
                MenuState::Main => 0.0,
                MenuState::Settings => 1.0,
                MenuState::Hidden => 0.0,
            },
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        
        // Собираем все instances
        let mut instances: Vec<MenuInstance> = Vec::new();
        
        // 1. Overlay (затемнение фона)
        instances.push(MenuInstance {
            pos: [self.overlay.x, self.overlay.y],
            size: [self.overlay.width, self.overlay.height],
            state: ElementType::Overlay as u32,
            extra: 0.0,
        });
        
        // 2. Панель
        let panel = match self.current_state {
            MenuState::Main => &self.panel_main,
            MenuState::Settings => &self.panel_settings,
            MenuState::Hidden => &self.panel_main,
        };
        instances.push(MenuInstance {
            pos: [panel.x, panel.y],
            size: [panel.width, panel.height],
            state: ElementType::Panel as u32,
            extra: 0.0,
        });
        
        // 3. Элементы UI
        let elements = match self.current_state {
            MenuState::Main => &self.main_elements,
            MenuState::Settings => &self.settings_elements,
            MenuState::Hidden => &self.main_elements,
        };
        
        for elem in elements {
            if !elem.visible {
                continue;
            }
            instances.push(MenuInstance {
                pos: [elem.x, elem.y],
                size: [elem.width, elem.height],
                state: elem.get_state(),
                extra: elem.value,
            });
        }
        
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw(0..6, 0..instances.len() as u32);
    }
    
    pub fn toggle(&mut self) {
        self.current_state = match self.current_state {
            MenuState::Hidden => MenuState::Main,
            _ => MenuState::Hidden,
        };
    }
    
    pub fn is_visible(&self) -> bool {
        self.current_state != MenuState::Hidden
    }
    
    pub fn show(&mut self) {
        self.current_state = MenuState::Main;
    }
    
    pub fn hide(&mut self) {
        self.current_state = MenuState::Hidden;
    }
    
    pub fn state(&self) -> MenuState {
        self.current_state
    }
    
    /// Получить значения LOD слайдеров
    pub fn get_lod_values(&self) -> [f32; 4] {
        let mut values = [0.5; 4];
        for (i, elem) in self.settings_elements.iter().take(4).enumerate() {
            values[i] = elem.value;
        }
        values
    }
    
    /// Получить параметры текста для рендеринга
    pub fn get_text_params(&self) -> Vec<super::TextParams> {
        use super::{TextParams, TextAlign};
        
        let mut texts = Vec::new();
        let cx = self.screen_width / 2.0;
        
        match self.current_state {
            MenuState::Main => {
                // Заголовок "HYTALE"
                texts.push(TextParams {
                    x: cx,
                    y: self.panel_main.y + 25.0,
                    text: "HYTALE".to_string(),
                    size: 28.0,
                    color: [0.0, 0.94, 1.0, 1.0], // Cyan accent
                    align: TextAlign::Center,
                    max_width: None,
                });
                
                // Подзаголовок
                texts.push(TextParams {
                    x: cx,
                    y: self.panel_main.y + 55.0,
                    text: "World: Creative_Zone_01".to_string(),
                    size: 12.0,
                    color: [1.0, 1.0, 1.0, 0.5],
                    align: TextAlign::Center,
                    max_width: None,
                });
                
                // Текст кнопок
                for elem in &self.main_elements {
                    println!("Button text: '{}' at ({}, {})", elem.label, elem.x + elem.width / 2.0, elem.y + elem.height / 2.0 - 8.0);
                    texts.push(TextParams {
                        x: elem.x + elem.width / 2.0,
                        y: elem.y + elem.height / 2.0 - 8.0,
                        text: elem.label.clone(),
                        size: 16.0,
                        color: [1.0, 1.0, 1.0, 1.0], // Белый для всех
                        align: TextAlign::Center,
                        max_width: None,
                    });
                }
            }
            MenuState::Settings => {
                // Заголовок
                texts.push(TextParams {
                    x: cx,
                    y: self.panel_settings.y + 30.0,
                    text: "Settings".to_string(),
                    size: 22.0,
                    color: [0.0, 0.94, 1.0, 1.0],
                    align: TextAlign::Center,
                    max_width: None,
                });
                
                // Секция LOD
                texts.push(TextParams {
                    x: self.panel_settings.x + 30.0,
                    y: self.panel_settings.y + 75.0,
                    text: "LOD Distances".to_string(),
                    size: 11.0,
                    color: [1.0, 1.0, 1.0, 0.5],
                    align: TextAlign::Left,
                    max_width: None,
                });
                
                // Лейблы и значения слайдеров
                for (i, elem) in self.settings_elements.iter().take(4).enumerate() {
                    let lod_name = format!("LOD{}", i);
                    let lod_value = format!("{}", (elem.value * 512.0) as i32);
                    
                    // Название слайдера
                    texts.push(TextParams {
                        x: elem.x,
                        y: elem.y - 18.0,
                        text: lod_name,
                        size: 14.0,
                        color: [1.0, 1.0, 1.0, 1.0],
                        align: TextAlign::Left,
                        max_width: None,
                    });
                    
                    // Значение
                    texts.push(TextParams {
                        x: elem.x + elem.width,
                        y: elem.y - 18.0,
                        text: lod_value,
                        size: 14.0,
                        color: [0.0, 0.94, 1.0, 1.0],
                        align: TextAlign::Right,
                        max_width: None,
                    });
                }
                
                // Текст кнопок
                for elem in self.settings_elements.iter().skip(4) {
                    texts.push(TextParams {
                        x: elem.x + elem.width / 2.0,
                        y: elem.y + elem.height / 2.0 - 8.0,
                        text: elem.label.clone(),
                        size: 16.0,
                        color: if elem.element_type == ElementType::ButtonPrimary {
                            [0.0, 0.0, 0.0, 1.0]
                        } else {
                            [1.0, 1.0, 1.0, 1.0]
                        },
                        align: TextAlign::Center,
                        max_width: None,
                    });
                }
            }
            MenuState::Hidden => {}
        }
        
        texts
    }
}

// ============================================
// Legacy GameMenu wrapper для совместимости
// ============================================

use super::layout::{LayoutNode, Rect};

pub struct GameMenu {
    state: MenuState,
    width: u32,
    height: u32,
}

impl GameMenu {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            state: MenuState::Hidden,
            width,
            height,
        }
    }
    
    pub fn toggle(&mut self) {
        self.state = match self.state {
            MenuState::Hidden => MenuState::Main,
            _ => MenuState::Hidden,
        };
    }
    
    pub fn hide(&mut self) {
        self.state = MenuState::Hidden;
    }
    
    pub fn is_visible(&self) -> bool {
        self.state != MenuState::Hidden
    }
    
    pub fn layout(&self) -> Vec<LayoutNode> {
        if !self.is_visible() {
            return Vec::new();
        }
        
        let center_x = self.width as f32 / 2.0;
        let start_y = self.height as f32 / 4.0;
        
        vec![
            LayoutNode {
                rect: Rect { x: center_x - 200.0, y: start_y, width: 400.0, height: 40.0 },
                id: Some("resume".to_string()),
            },
            LayoutNode {
                rect: Rect { x: center_x - 200.0, y: start_y + 50.0, width: 195.0, height: 40.0 },
                id: Some("settings".to_string()),
            },
            LayoutNode {
                rect: Rect { x: center_x + 5.0, y: start_y + 50.0, width: 195.0, height: 40.0 },
                id: Some("lan".to_string()),
            },
            LayoutNode {
                rect: Rect { x: center_x - 200.0, y: start_y + 100.0, width: 400.0, height: 40.0 },
                id: Some("quit".to_string()),
            },
        ]
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
    
    pub fn process_click(&mut self, x: f32, y: f32) -> MenuAction {
        if self.state == MenuState::Hidden {
            return MenuAction::None;
        }
        
        for node in self.layout() {
            if node.rect.contains(x, y) {
                if let Some(id) = &node.id {
                    match id.as_str() {
                        "resume" => {
                            self.hide();
                            return MenuAction::Resume;
                        }
                        "settings" => {
                            self.state = MenuState::Settings;
                            return MenuAction::Settings;
                        }
                        "quit" => {
                            return MenuAction::QuitToDesktop;
                        }
                        _ => {}
                    }
                }
            }
        }
        
        MenuAction::None
    }
}
