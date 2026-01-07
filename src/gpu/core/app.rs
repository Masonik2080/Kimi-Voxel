// ============================================
// App - Главный обработчик приложения
// ============================================

use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::gpu::core::GameResources;
use crate::gpu::systems::{
    InitSystem, InputSystem, InputAction, BlockInteractionSystem,
    MenuSystem, SaveSystem, UpdateSystem, RenderSystem,
};
use crate::gpu::blocks::MouseButton;

/// Главное приложение
pub struct App {
    resources: GameResources,
}

impl App {
    pub fn new() -> Self {
        Self {
            resources: InitSystem::create_resources(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.resources.window.is_none() {
            let window_attrs = Window::default_attributes()
                .with_title("GPU Infinite Terrain - Press F5 to toggle camera mode")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
            
            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            
            InitSystem::init_rendering(&mut self.resources, window);
            
            // Захватываем курсор при старте
            InputSystem::grab_cursor(&mut self.resources, true);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                SaveSystem::save_world(&self.resources);
                event_loop.exit();
            }
            
            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.resources.renderer {
                    renderer.resize(physical_size);
                    self.resources.camera.resize(physical_size.width, physical_size.height);
                    self.resources.menu.resize(physical_size.width, physical_size.height);
                    
                    if let Some(gui_renderer) = &mut self.resources.gui_renderer {
                        gui_renderer.resize(renderer.queue(), physical_size.width, physical_size.height);
                    }
                }
            }
            
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(keycode),
                    state,
                    ..
                },
                ..
            } => {
                if let Some(action) = InputSystem::process_keyboard(&mut self.resources, keycode, state) {
                    match action {
                        InputAction::SaveWorld => {
                            SaveSystem::save_world(&self.resources);
                        }
                        InputAction::CycleTime => {
                            if let Some(renderer) = &mut self.resources.renderer {
                                let current = renderer.time_of_day();
                                let next = if current < 0.25 { 0.35 }
                                    else if current < 0.5 { 0.5 }
                                    else if current < 0.75 { 0.7 }
                                    else { 0.0 };
                                renderer.set_time_of_day(next);
                            }
                        }
                        InputAction::SlowTime => {
                            if let Some(renderer) = &mut self.resources.renderer {
                                renderer.set_time_speed(10.0);
                            }
                        }
                        InputAction::FastTime => {
                            if let Some(renderer) = &mut self.resources.renderer {
                                renderer.set_time_speed(120.0);
                            }
                        }
                        _ => {}
                    }
                }
            }
            
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.resources.last_frame).as_secs_f32();
                self.resources.last_frame = now;
                let time = (now - self.resources.start_time).as_secs_f32();
                
                // Update
                UpdateSystem::update(&mut self.resources, dt, time);
                
                // Render
                RenderSystem::render(&mut self.resources, time, dt, event_loop);
                
                if let Some(window) = &self.resources.window {
                    window.request_redraw();
                }
            }
            
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                
                // Проверяем инвентарь первым
                let inventory_visible = if let Some(gui) = &self.resources.gui_renderer {
                    gui.inventory_ref().is_visible()
                } else {
                    false
                };
                
                if inventory_visible {
                    if button == winit::event::MouseButton::Left {
                        if pressed {
                            MenuSystem::handle_mouse_down(&mut self.resources);
                        } else {
                            MenuSystem::handle_mouse_up(&mut self.resources);
                        }
                    }
                } else if self.resources.menu.is_visible() {
                    // Меню открыто
                    if button == winit::event::MouseButton::Left {
                        self.resources.menu_mouse_pressed = pressed;
                    }
                    
                    if pressed && button == winit::event::MouseButton::Left {
                        MenuSystem::handle_click(&mut self.resources, event_loop);
                    }
                } else if self.resources.cursor_grabbed {
                    // Игровой режим
                    if pressed {
                        match button {
                            winit::event::MouseButton::Left => {
                                BlockInteractionSystem::handle_break(&mut self.resources);
                            }
                            winit::event::MouseButton::Right => {
                                BlockInteractionSystem::handle_place(&mut self.resources);
                            }
                            winit::event::MouseButton::Middle => {
                                BlockInteractionSystem::handle_pick_block(&mut self.resources);
                            }
                            _ => {}
                        }
                    }
                }
            }
            
            WindowEvent::CursorMoved { position, .. } => {
                self.resources.mouse_pos = (position.x as f32, position.y as f32);
            }
            
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if self.resources.menu.is_visible() {
            return;
        }
        
        match event {
            DeviceEvent::MouseMotion { delta } => {
                InputSystem::process_mouse_motion(&mut self.resources, delta);
            }
            
            DeviceEvent::MouseWheel { delta } => {
                InputSystem::process_mouse_wheel(&mut self.resources, delta);
            }
            
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.resources.window {
            window.request_redraw();
        }
    }
}

/// Запуск игры
pub fn run() {
    env_logger::init();
    
    println!("=== Controls ===");
    println!("WASD - Move");
    println!("Mouse - Look around");
    println!("Space - Jump / Fly up");
    println!("Shift/Ctrl - Sprint / Fly down");
    println!("F - Toggle flight mode");
    println!("LMB - Break block");
    println!("RMB - Place block");
    println!("F5 - Toggle camera mode (1st/3rd person)");
    println!("F6 - Save world");
    println!("Mouse wheel / +/- - Adjust camera distance");
    println!("T - Cycle time of day");
    println!("[ / ] - Slow/fast time speed");
    println!("Escape - Open menu");
    println!("================");
    
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
