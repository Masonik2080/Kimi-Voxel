// ============================================
// Input System - Обработка ввода
// ============================================

use std::sync::Arc;
use winit::{
    event::{ElementState, KeyEvent, DeviceEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

use crate::gpu::core::GameResources;
use crate::gpu::gui::MenuAction;

/// Система обработки клавиатуры
pub struct InputSystem;

impl InputSystem {
    /// Обработка клавиатурного ввода
    pub fn process_keyboard(
        resources: &mut GameResources,
        keycode: KeyCode,
        state: ElementState,
    ) -> Option<InputAction> {
        let pressed = state == ElementState::Pressed;
        
        match keycode {
            // Escape - открыть/закрыть меню
            KeyCode::Escape if pressed => {
                // Если открыт инвентарь - закрываем его
                if let Some(gui) = &mut resources.gui_renderer {
                    if gui.inventory().is_visible() {
                        gui.inventory().hide();
                        Self::grab_cursor(resources, true);
                        return Some(InputAction::InventoryToggle);
                    }
                }
                
                resources.menu.toggle();
                if let Some(gui) = &mut resources.gui_renderer {
                    gui.menu_system().toggle();
                }
                
                if resources.menu.is_visible() {
                    Self::grab_cursor(resources, false);
                } else {
                    Self::grab_cursor(resources, true);
                }
                Some(InputAction::MenuToggle)
            }
            
            // E - открыть/закрыть инвентарь
            KeyCode::KeyE if pressed => {
                if !resources.menu.is_visible() {
                    if let Some(gui) = &mut resources.gui_renderer {
                        gui.inventory().toggle();
                        
                        if gui.inventory().is_visible() {
                            Self::grab_cursor(resources, false);
                        } else {
                            Self::grab_cursor(resources, true);
                        }
                        return Some(InputAction::InventoryToggle);
                    }
                }
                None
            }
            
            // Q - переключить размер суб-вокселя
            KeyCode::KeyQ if pressed => {
                resources.current_subvoxel_level = resources.current_subvoxel_level.next();
                println!("[SUBVOXEL] Размер блока: {}", resources.current_subvoxel_level.name());
                Some(InputAction::SubvoxelLevelChange)
            }
            
            // F5 - переключить режим камеры
            KeyCode::F5 if pressed => {
                resources.camera.toggle_mode();
                Some(InputAction::CameraToggle)
            }
            
            // F6 - сохранить мир
            KeyCode::F6 if pressed => {
                Some(InputAction::SaveWorld)
            }
            
            // +/- для дистанции камеры
            KeyCode::Equal | KeyCode::NumpadAdd if pressed => {
                resources.camera.third_person_distance = 
                    (resources.camera.third_person_distance + 1.0).min(20.0);
                None
            }
            KeyCode::Minus | KeyCode::NumpadSubtract if pressed => {
                resources.camera.third_person_distance = 
                    (resources.camera.third_person_distance - 1.0).max(2.0);
                None
            }
            
            // T - переключить время
            KeyCode::KeyT if pressed => {
                Some(InputAction::CycleTime)
            }
            
            // [ и ] - скорость времени
            KeyCode::BracketLeft if pressed => {
                Some(InputAction::SlowTime)
            }
            KeyCode::BracketRight if pressed => {
                Some(InputAction::FastTime)
            }
            
            // Клавиши 1-9 для хотбара
            _ => {
                if !resources.menu.is_visible() {
                    let slot_key = match keycode {
                        KeyCode::Digit1 => Some(1),
                        KeyCode::Digit2 => Some(2),
                        KeyCode::Digit3 => Some(3),
                        KeyCode::Digit4 => Some(4),
                        KeyCode::Digit5 => Some(5),
                        KeyCode::Digit6 => Some(6),
                        KeyCode::Digit7 => Some(7),
                        KeyCode::Digit8 => Some(8),
                        KeyCode::Digit9 => Some(9),
                        _ => None,
                    };
                    
                    if let Some(key) = slot_key {
                        if pressed {
                            if let Some(gui) = &mut resources.gui_renderer {
                                gui.hotbar().select_by_key(key);
                            }
                        }
                    } else {
                        resources.player_controller.process_keyboard(keycode, pressed);
                    }
                }
                None
            }
        }
    }
    
    /// Обработка движения мыши
    pub fn process_mouse_motion(resources: &mut GameResources, delta: (f64, f64)) {
        if resources.cursor_grabbed && !resources.menu.is_visible() {
            resources.player_controller.process_mouse(delta.0, delta.1);
        }
    }
    
    /// Обработка колеса мыши
    pub fn process_mouse_wheel(resources: &mut GameResources, delta: winit::event::MouseScrollDelta) {
        let scroll = match delta {
            winit::event::MouseScrollDelta::LineDelta(_, y) => y as i32,
            winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.y / 100.0) as i32,
        };
        
        if scroll != 0 {
            // Если открыт инвентарь - скроллим его
            // scroll > 0 когда крутим вверх, < 0 когда вниз
            if let Some(gui) = &mut resources.gui_renderer {
                if gui.inventory().is_visible() {
                    gui.inventory().scroll_by(scroll as f32 * 0.5);
                    return;
                }
            }
            
            // Иначе скроллим хотбар
            if resources.cursor_grabbed && !resources.menu.is_visible() {
                if let Some(gui) = &mut resources.gui_renderer {
                    gui.hotbar().scroll(-scroll);
                }
            }
        }
    }
    
    /// Захват/освобождение курсора
    pub fn grab_cursor(resources: &mut GameResources, grab: bool) {
        if let Some(window) = &resources.window {
            resources.cursor_grabbed = grab;
            if grab {
                let _ = window.set_cursor_grab(CursorGrabMode::Confined)
                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked));
                window.set_cursor_visible(false);
            } else {
                let _ = window.set_cursor_grab(CursorGrabMode::None);
                window.set_cursor_visible(true);
            }
        }
    }
}

/// Действия, которые могут быть вызваны вводом
#[derive(Debug, Clone, Copy)]
pub enum InputAction {
    MenuToggle,
    InventoryToggle,
    SubvoxelLevelChange,
    CameraToggle,
    SaveWorld,
    CycleTime,
    SlowTime,
    FastTime,
}
