// ============================================
// GUI Module - Minecraft-style menu rendering
// ============================================

mod menu;
mod text;
mod crosshair;
mod fps_counter;
pub mod hotbar;
pub mod inventory;

pub use menu::{GameMenu, MenuState, MenuAction, MenuSystem};
pub use text::{TextRenderer, TextParams, TextAlign};
pub use hotbar::{Hotbar, HotbarItem, HotbarRenderer, HotbarSlot};
pub use crosshair::{Crosshair, BlockHighlight, UiVertex, WireVertex};
pub use fps_counter::FpsCounter;
pub use inventory::{Inventory, InventoryRenderer};

/// GPU рендерер для меню
pub struct GuiRenderer {
    menu_system: MenuSystem,
    text_renderer: TextRenderer,
    hotbar_renderer: hotbar::HotbarRenderer,
    hotbar: Hotbar,
    inventory_renderer: inventory::InventoryRenderer,
    inventory: Inventory,
    screen_width: u32,
    screen_height: u32,
}

impl GuiRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        global_bind_group_layout: &wgpu::BindGroupLayout,
        width: u32,
        height: u32,
    ) -> Self {
        let menu_system = MenuSystem::new(device, format, global_bind_group_layout, width, height);
        let text_renderer = TextRenderer::new(device, queue, format, width, height);
        let hotbar_renderer = hotbar::HotbarRenderer::new(device, format, width, height);
        let hotbar = Hotbar::new();
        let inventory_renderer = inventory::InventoryRenderer::new(device, format, width, height);
        let inventory = Inventory::new();
        
        Self { 
            menu_system,
            text_renderer,
            hotbar_renderer,
            hotbar,
            inventory_renderer,
            inventory,
            screen_width: width,
            screen_height: height,
        }
    }
    
    pub fn resize(&mut self, queue: &wgpu::Queue, width: u32, height: u32) {
        self.menu_system.resize(width, height);
        self.text_renderer.resize(queue, width, height);
        self.hotbar_renderer.resize(width, height);
        self.inventory_renderer.resize(width, height);
        self.screen_width = width;
        self.screen_height = height;
    }
    
    pub fn menu_system(&mut self) -> &mut MenuSystem {
        &mut self.menu_system
    }
    
    pub fn hotbar(&mut self) -> &mut Hotbar {
        &mut self.hotbar
    }
    
    pub fn inventory(&mut self) -> &mut Inventory {
        &mut self.inventory
    }
    
    pub fn inventory_ref(&self) -> &Inventory {
        &self.inventory
    }
    
    pub fn inventory_renderer(&self) -> &inventory::InventoryRenderer {
        &self.inventory_renderer
    }
    
    pub fn inventory_renderer_mut(&mut self) -> &mut inventory::InventoryRenderer {
        &mut self.inventory_renderer
    }
    
    pub fn screen_size(&self) -> (f32, f32) {
        (self.screen_width as f32, self.screen_height as f32)
    }
    
    /// Рендерит меню используя encoder (создаёт свой render pass)
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        mouse_pos: (f32, f32),
    ) {
        // Рендерим хотбар (всегда, если не в меню)
        if !self.menu_system.is_visible() && self.hotbar.is_visible() {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Hotbar Render Pass"),
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
            
            self.hotbar_renderer.render(&mut render_pass, queue, &self.hotbar);
        }
        
        // Рендерим инвентарь
        if self.inventory.is_visible() {
            self.inventory_renderer.update_inventory_scroll(&mut self.inventory);
            
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Inventory Render Pass"),
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
                
                self.inventory_renderer.render(&mut render_pass, queue, &self.inventory);
            }
            
            // Рендерим перетаскиваемый блок поверх всего
            if self.inventory.dragging().is_some() {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Dragging Block Render Pass"),
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
                
                self.inventory_renderer.render_dragging(&mut render_pass, queue, &self.inventory, mouse_pos.0, mouse_pos.1);
            }
            
            // Рендерим текст инвентаря
            let (panel_x, panel_y) = self.inventory_renderer.panel_pos();
            let (panel_w, _) = self.inventory_renderer.panel_size();
            
            let texts = vec![
                TextParams {
                    x: panel_x + panel_w / 2.0,
                    y: panel_y + 18.0,
                    text: "INVENTORY".to_string(),
                    size: 20.0,
                    color: [0.0, 0.94, 1.0, 1.0],
                    align: TextAlign::Center,
                    max_width: None,
                },
            ];
            self.text_renderer.render(device, encoder, view, queue, &texts);
            return;
        }
        
        if !self.menu_system.is_visible() {
            return;
        }
        
        // Рендерим UI элементы меню
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Menu Render Pass"),
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
            
            self.menu_system.render(&mut render_pass, queue);
        }
        
        // Рендерим текст поверх
        let texts = self.menu_system.get_text_params();
        self.text_renderer.render(device, encoder, view, queue, &texts);
    }
}

// Заглушки для layout (для совместимости)
pub mod layout {
    #[derive(Debug, Clone)]
    pub struct LayoutNode {
        pub rect: Rect,
        pub id: Option<String>,
    }
    
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Rect {
        pub x: f32,
        pub y: f32,
        pub width: f32,
        pub height: f32,
    }
    
    impl Rect {
        pub fn contains(&self, px: f32, py: f32) -> bool {
            px >= self.x && px <= self.x + self.width &&
            py >= self.y && py <= self.y + self.height
        }
    }
}
