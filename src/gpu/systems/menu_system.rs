// ============================================
// Menu System - Обработка игрового меню
// ============================================

use winit::event_loop::ActiveEventLoop;

use crate::gpu::core::GameResources;
use crate::gpu::gui::MenuAction;
use crate::gpu::systems::input_system::InputSystem;
use crate::gpu::systems::save_system::SaveSystem;

/// Система обработки меню
pub struct MenuSystem;

impl MenuSystem {
    /// Обработка нажатия кнопки мыши (начало drag)
    pub fn handle_mouse_down(
        resources: &mut GameResources,
    ) {
        if let Some(gui) = &mut resources.gui_renderer {
            if gui.inventory_ref().is_visible() {
                let mx = resources.mouse_pos.0;
                let my = resources.mouse_pos.1;
                
                // Проверяем клик по слоту инвентаря
                let slot_at = gui.inventory_renderer().get_slot_at(mx, my, gui.inventory_ref());
                
                if let Some(slot_index) = slot_at {
                    // Начинаем перетаскивание
                    gui.inventory().handle_click(slot_index);
                }
            }
        }
    }
    
    /// Обработка отпускания кнопки мыши (drop)
    pub fn handle_mouse_up(
        resources: &mut GameResources,
    ) -> bool {
        let mut should_grab_cursor = false;
        
        if let Some(gui) = &mut resources.gui_renderer {
            if gui.inventory_ref().is_visible() {
                // Проверяем есть ли перетаскиваемый блок
                if let Some(block_type) = gui.inventory().dragging() {
                    let mx = resources.mouse_pos.0;
                    let my = resources.mouse_pos.1;
                    
                    // Проверяем drop на хотбар
                    let (screen_w, screen_h) = gui.screen_size();
                    
                    if gui.hotbar().handle_click(mx, my, screen_w, screen_h) {
                        // Кликнули на слот хотбара - добавляем туда блок
                        let selected_slot = gui.hotbar().selected();
                        gui.hotbar().set_item(selected_slot, Some(crate::gpu::gui::hotbar::HotbarItem::from_block(block_type)));
                    }
                    
                    // Завершаем перетаскивание
                    gui.inventory().end_drag();
                }
            }
        }
        
        should_grab_cursor
    }
    
    /// Обработка клика по меню или инвентарю (legacy - для совместимости)
    pub fn handle_click(
        resources: &mut GameResources,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        // Сначала проверяем инвентарь
        let inventory_action = if let Some(gui) = &mut resources.gui_renderer {
            if gui.inventory_ref().is_visible() {
                let mx = resources.mouse_pos.0;
                let my = resources.mouse_pos.1;
                
                // Получаем данные для проверки
                let slot_at = gui.inventory_renderer().get_slot_at(mx, my, gui.inventory_ref());
                
                // Проверяем клик по слоту
                if let Some(slot_index) = slot_at {
                    let block_type = gui.inventory().handle_click(slot_index);
                    if let Some(bt) = block_type {
                        gui.hotbar().pick_block(bt);
                        Some(true) // Нужно grab cursor
                    } else {
                        Some(false)
                    }
                } else {
                    // Проверяем клик по скроллбару
                    let is_scrollbar = gui.inventory_renderer().is_scrollbar_click(mx, my);
                    if is_scrollbar {
                        let max_scroll = gui.inventory_ref().max_scroll();
                        let scroll = gui.inventory_renderer().get_scroll_from_mouse_raw(my, max_scroll);
                        gui.inventory().set_scroll(scroll);
                    }
                    Some(false)
                }
            } else {
                None
            }
        } else {
            None
        };
        
        if let Some(need_grab) = inventory_action {
            if need_grab {
                InputSystem::grab_cursor(resources, true);
            }
            return false;
        }
        
        let action = if let Some(gui) = &mut resources.gui_renderer {
            gui.menu_system().handle_click(resources.mouse_pos.0, resources.mouse_pos.1)
        } else {
            resources.menu.process_click(resources.mouse_pos.0, resources.mouse_pos.1)
        };
        
        match action {
            MenuAction::Resume => {
                resources.menu.hide();
                if let Some(gui) = &mut resources.gui_renderer {
                    gui.menu_system().hide();
                }
                InputSystem::grab_cursor(resources, true);
                false
            }
            MenuAction::SaveSettings => {
                Self::apply_lod_settings(resources);
                false
            }
            MenuAction::QuitToDesktop => {
                SaveSystem::save_world(resources);
                event_loop.exit();
                true
            }
            _ => false
        }
    }
    
    /// Обновление hover состояния меню и инвентаря
    pub fn update_hover(resources: &mut GameResources) {
        // Обновляем инвентарь
        if let Some(gui) = &mut resources.gui_renderer {
            if gui.inventory_ref().is_visible() {
                let mx = resources.mouse_pos.0;
                let my = resources.mouse_pos.1;
                
                let hovered = gui.inventory_renderer().get_slot_at(mx, my, gui.inventory_ref());
                gui.inventory().set_hovered(hovered);
                return;
            }
        }
        
        // Обновляем меню
        if resources.menu.is_visible() {
            if let Some(gui) = &mut resources.gui_renderer {
                gui.menu_system().handle_mouse_move(resources.mouse_pos.0, resources.mouse_pos.1);
                gui.menu_system().handle_drag(resources.mouse_pos.0, resources.mouse_pos.1, resources.menu_mouse_pressed);
            }
        }
    }
    
    /// Обработка скролла в инвентаре
    pub fn handle_inventory_scroll(resources: &mut GameResources, delta: f32) {
        if let Some(gui) = &mut resources.gui_renderer {
            if gui.inventory_ref().is_visible() {
                gui.inventory().scroll_by(delta);
            }
        }
    }
    
    /// Применение настроек LOD
    fn apply_lod_settings(resources: &mut GameResources) {
        let distances = if let Some(gui) = &mut resources.gui_renderer {
            let lod_values = gui.menu_system().get_lod_values();
            // Конвертируем 0-1 в дистанции чанков (4-64)
            Some([
                (lod_values[0] * 60.0 + 4.0) as i32,
                (lod_values[1] * 60.0 + 4.0) as i32,
                (lod_values[2] * 60.0 + 4.0) as i32,
                (lod_values[3] * 60.0 + 4.0) as i32,
            ])
        } else {
            None
        };
        
        if let (Some(distances), Some(renderer)) = (distances, &mut resources.renderer) {
            renderer.set_lod_distances(distances);
            println!("[LOD] Applied distances: {:?}", distances);
        }
    }
}
