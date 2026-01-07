// ============================================
// Inventory GPU Renderer - Hi-Tech glassmorphism
// ============================================

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use std::time::Instant;

use super::{
    Inventory,
    INVENTORY_COLS, INV_SLOT_SIZE, INV_SLOT_GAP, INV_PADDING,
    HEADER_HEIGHT, SCROLLBAR_WIDTH,
};
use crate::gpu::blocks::{BlockType, get_face_colors};

/// Uniforms для шейдера инвентаря
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct InventoryUniforms {
    pub screen_size: [f32; 2],
    pub time: f32,
    pub scroll: f32,
    pub panel_pos: [f32; 2],
    pub panel_size: [f32; 2],
}

/// Данные одного слота для GPU
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct InventorySlot {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub slot_type: u32,      // 0=overlay, 1=panel, 2=slot, 3=scrollbar, 4=scrollthumb, 5=header
    pub is_hovered: u32,
    pub has_item: u32,
    pub _padding: u32,
    pub top_color: [f32; 4],
    pub side_color: [f32; 4],
}

/// GPU рендерер инвентаря
pub struct InventoryRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    
    screen_width: f32,
    screen_height: f32,
    start_time: Instant,
    
    // Кэшированные размеры панели
    panel_x: f32,
    panel_y: f32,
    panel_width: f32,
    panel_height: f32,
    content_height: f32,
    visible_rows: usize,
}

impl InventoryRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Inventory Bind Group Layout"),
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
        
        let uniforms = InventoryUniforms {
            screen_size: [width as f32, height as f32],
            time: 0.0,
            scroll: 0.0,
            panel_pos: [0.0, 0.0],
            panel_size: [0.0, 0.0],
        };
        
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Inventory Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Inventory Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        
        let vertices: Vec<[f32; 2]> = vec![
            [0.0, 0.0], [1.0, 0.0], [1.0, 1.0],
            [0.0, 0.0], [1.0, 1.0], [0.0, 1.0],
        ];
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Inventory Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        // Буфер для ~100 слотов + UI элементы
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Inventory Instance Buffer"),
            size: (std::mem::size_of::<InventorySlot>() * 120) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Inventory Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("inventory.wgsl").into()),
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Inventory Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Inventory Pipeline"),
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
                        array_stride: std::mem::size_of::<InventorySlot>() as u64,
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
                                format: wgpu::VertexFormat::Uint32,
                                offset: 20,
                                shader_location: 4,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Uint32,
                                offset: 24,
                                shader_location: 5,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 32,
                                shader_location: 6,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x4,
                                offset: 48,
                                shader_location: 7,
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
        
        let mut renderer = Self {
            pipeline,
            vertex_buffer,
            instance_buffer,
            uniform_buffer,
            bind_group,
            screen_width: width as f32,
            screen_height: height as f32,
            start_time: Instant::now(),
            panel_x: 0.0,
            panel_y: 0.0,
            panel_width: 0.0,
            panel_height: 0.0,
            content_height: 0.0,
            visible_rows: 0,
        };
        
        renderer.update_layout();
        renderer
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.screen_width = width as f32;
        self.screen_height = height as f32;
        self.update_layout();
    }
    
    fn update_layout(&mut self) {
        // Размер панели - 70% экрана по ширине, 60% по высоте (чтобы хотбар был виден)
        self.panel_width = (self.screen_width * 0.7).min(
            INVENTORY_COLS as f32 * (INV_SLOT_SIZE + INV_SLOT_GAP) + INV_PADDING * 2.0 + SCROLLBAR_WIDTH + 20.0
        );
        self.panel_height = (self.screen_height * 0.6).min(self.screen_height - 150.0); // Оставляем место для хотбара
        
        // Центрируем панель, но немного выше чтобы не перекрывать хотбар
        self.panel_x = (self.screen_width - self.panel_width) / 2.0;
        self.panel_y = (self.screen_height - self.panel_height - 120.0) / 2.0; // Смещаем вверх
        
        // Высота контента (без заголовка)
        self.content_height = self.panel_height - HEADER_HEIGHT - INV_PADDING * 2.0;
        
        // Количество видимых рядов
        self.visible_rows = ((self.content_height) / (INV_SLOT_SIZE + INV_SLOT_GAP)) as usize;
    }
    
    /// Получить индекс слота под курсором
    pub fn get_slot_at(&self, mx: f32, my: f32, inventory: &Inventory) -> Option<usize> {
        if !inventory.is_visible() {
            return None;
        }
        
        let content_x = self.panel_x + INV_PADDING;
        let content_y = self.panel_y + HEADER_HEIGHT + INV_PADDING;
        
        // Проверяем что курсор в области контента
        if mx < content_x || mx > content_x + INVENTORY_COLS as f32 * (INV_SLOT_SIZE + INV_SLOT_GAP) {
            return None;
        }
        if my < content_y || my > content_y + self.content_height {
            return None;
        }
        
        let rel_x = mx - content_x;
        let rel_y = my - content_y + inventory.scroll() * (INV_SLOT_SIZE + INV_SLOT_GAP);
        
        let col = (rel_x / (INV_SLOT_SIZE + INV_SLOT_GAP)) as usize;
        let row = (rel_y / (INV_SLOT_SIZE + INV_SLOT_GAP)) as usize;
        
        if col >= INVENTORY_COLS {
            return None;
        }
        
        let index = row * INVENTORY_COLS + col;
        let items = inventory.filtered_items();
        
        if index < items.len() {
            Some(index)
        } else {
            None
        }
    }
    
    /// Проверить клик по скроллбару
    pub fn is_scrollbar_click(&self, mx: f32, my: f32) -> bool {
        let sb_x = self.panel_x + self.panel_width - SCROLLBAR_WIDTH - INV_PADDING;
        let sb_y = self.panel_y + HEADER_HEIGHT + INV_PADDING;
        let sb_height = self.content_height;
        
        mx >= sb_x && mx <= sb_x + SCROLLBAR_WIDTH &&
        my >= sb_y && my <= sb_y + sb_height
    }
    
    /// Получить scroll из позиции мыши на скроллбаре
    pub fn get_scroll_from_mouse(&self, my: f32, inventory: &Inventory) -> f32 {
        self.get_scroll_from_mouse_raw(my, inventory.max_scroll())
    }
    
    /// Получить scroll из позиции мыши (без ссылки на инвентарь)
    pub fn get_scroll_from_mouse_raw(&self, my: f32, max_scroll: f32) -> f32 {
        let sb_y = self.panel_y + HEADER_HEIGHT + INV_PADDING;
        let sb_height = self.content_height;
        
        let rel_y = (my - sb_y).clamp(0.0, sb_height);
        let ratio = rel_y / sb_height;
        
        ratio * max_scroll
    }
    
    /// Обновить max_scroll в инвентаре
    pub fn update_inventory_scroll(&self, inventory: &mut Inventory) {
        let items = inventory.filtered_items();
        let total_rows = (items.len() + INVENTORY_COLS - 1) / INVENTORY_COLS;
        inventory.update_max_scroll(self.visible_rows, total_rows);
    }
    
    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        inventory: &Inventory,
    ) {
        if !inventory.is_visible() {
            return;
        }
        
        let time = self.start_time.elapsed().as_secs_f32();
        
        let uniforms = InventoryUniforms {
            screen_size: [self.screen_width, self.screen_height],
            time,
            scroll: inventory.scroll(),
            panel_pos: [self.panel_x, self.panel_y],
            panel_size: [self.panel_width, self.panel_height],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        
        let mut instances: Vec<InventorySlot> = Vec::with_capacity(100);
        
        // 1. Overlay (затемнение)
        instances.push(InventorySlot {
            pos: [0.0, 0.0],
            size: [self.screen_width, self.screen_height],
            slot_type: 0, // overlay
            is_hovered: 0,
            has_item: 0,
            _padding: 0,
            top_color: [0.0, 0.0, 0.0, 0.0],
            side_color: [0.0, 0.0, 0.0, 0.0],
        });
        
        // 2. Панель
        instances.push(InventorySlot {
            pos: [self.panel_x, self.panel_y],
            size: [self.panel_width, self.panel_height],
            slot_type: 1, // panel
            is_hovered: 0,
            has_item: 0,
            _padding: 0,
            top_color: [0.0, 0.0, 0.0, 0.0],
            side_color: [0.0, 0.0, 0.0, 0.0],
        });
        
        // 3. Header
        instances.push(InventorySlot {
            pos: [self.panel_x, self.panel_y],
            size: [self.panel_width, HEADER_HEIGHT],
            slot_type: 5, // header
            is_hovered: 0,
            has_item: 0,
            _padding: 0,
            top_color: [0.0, 0.0, 0.0, 0.0],
            side_color: [0.0, 0.0, 0.0, 0.0],
        });
        
        // 4. Слоты с блоками
        let items = inventory.filtered_items();
        let total_rows = (items.len() + INVENTORY_COLS - 1) / INVENTORY_COLS;
        let scroll_offset = inventory.scroll();
        let start_row = scroll_offset as usize;
        
        let content_x = self.panel_x + INV_PADDING;
        let content_y = self.panel_y + HEADER_HEIGHT + INV_PADDING;
        
        for row in start_row..(start_row + self.visible_rows + 1).min(total_rows) {
            for col in 0..INVENTORY_COLS {
                let index = row * INVENTORY_COLS + col;
                if index >= items.len() {
                    break;
                }
                
                let item = items[index];
                let slot_x = content_x + col as f32 * (INV_SLOT_SIZE + INV_SLOT_GAP);
                let slot_y = content_y + (row as f32 - scroll_offset) * (INV_SLOT_SIZE + INV_SLOT_GAP);
                
                // Пропускаем слоты за пределами видимой области
                if slot_y + INV_SLOT_SIZE < content_y || slot_y > content_y + self.content_height {
                    continue;
                }
                
                let is_hovered = inventory.hovered() == Some(index);
                
                instances.push(InventorySlot {
                    pos: [slot_x, slot_y],
                    size: [INV_SLOT_SIZE, INV_SLOT_SIZE],
                    slot_type: 2, // slot
                    is_hovered: if is_hovered { 1 } else { 0 },
                    has_item: 1,
                    _padding: 0,
                    top_color: [item.top_color[0], item.top_color[1], item.top_color[2], 1.0],
                    side_color: [item.side_color[0], item.side_color[1], item.side_color[2], 1.0],
                });
            }
        }
        
        // 5. Scrollbar track
        let sb_x = self.panel_x + self.panel_width - SCROLLBAR_WIDTH - INV_PADDING;
        let sb_y = self.panel_y + HEADER_HEIGHT + INV_PADDING;
        let sb_height = self.content_height;
        
        instances.push(InventorySlot {
            pos: [sb_x, sb_y],
            size: [SCROLLBAR_WIDTH, sb_height],
            slot_type: 3, // scrollbar track
            is_hovered: 0,
            has_item: 0,
            _padding: 0,
            top_color: [0.0, 0.0, 0.0, 0.0],
            side_color: [0.0, 0.0, 0.0, 0.0],
        });
        
        // 6. Scrollbar thumb
        let max_scroll = inventory.max_scroll();
        if max_scroll > 0.0 {
            let thumb_height = (self.visible_rows as f32 / total_rows as f32 * sb_height).max(30.0);
            let thumb_y = sb_y + (scroll_offset / max_scroll) * (sb_height - thumb_height);
            
            instances.push(InventorySlot {
                pos: [sb_x, thumb_y],
                size: [SCROLLBAR_WIDTH, thumb_height],
                slot_type: 4, // scrollbar thumb
                is_hovered: 0,
                has_item: 0,
                _padding: 0,
                top_color: [0.0, 0.0, 0.0, 0.0],
                side_color: [0.0, 0.0, 0.0, 0.0],
            });
        }
        
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw(0..6, 0..instances.len() as u32);
    }
    
    /// Рендер перетаскиваемого блока (вызывается отдельно с позицией мыши)
    pub fn render_dragging<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        inventory: &Inventory,
        mouse_x: f32,
        mouse_y: f32,
    ) {
        if let Some(block_type) = inventory.dragging() {
            let (top, side) = get_face_colors(block_type);
            
            let drag_size = 56.0; // Немного меньше слота
            
            let instances = vec![InventorySlot {
                pos: [mouse_x - drag_size / 2.0, mouse_y - drag_size / 2.0],
                size: [drag_size, drag_size],
                slot_type: 6, // dragging item
                is_hovered: 1,
                has_item: 1,
                _padding: 0,
                top_color: [top[0], top[1], top[2], 1.0],
                side_color: [side[0], side[1], side[2], 1.0],
            }];
            
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
            
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.draw(0..6, 0..1);
        }
    }
    
    /// Получить позицию панели для текста
    pub fn panel_pos(&self) -> (f32, f32) {
        (self.panel_x, self.panel_y)
    }
    
    /// Получить размер панели
    pub fn panel_size(&self) -> (f32, f32) {
        (self.panel_width, self.panel_height)
    }
}
