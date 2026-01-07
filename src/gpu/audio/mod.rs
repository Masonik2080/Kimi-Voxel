// ============================================
// Audio Module - ECS-архитектура
// ============================================
// Пространственный звук с рейтрейсингом окружения

mod components;
mod resources;
mod environment;
mod systems;
mod utils;

pub use components::*;
pub use resources::*;
pub use environment::*;
pub use systems::*;
pub use utils::rand_simple;

use kira::manager::{AudioManager, AudioManagerSettings, backend::DefaultBackend};

/// Главная аудио система - фасад для всех подсистем
pub struct AudioSystem {
    manager: AudioManager,
    sounds: SoundResources,
    environment: EnvironmentAnalyzer,
    current_modifiers: SoundModifiers,
    block_checker: Option<BlockSolidChecker>,
    
    // Состояния подсистем
    footstep_state: FootstepState,
    jump_state: JumpState,
}

impl AudioSystem {
    pub fn new() -> Result<Self, String> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| format!("Failed to create audio manager: {:?}", e))?;
        
        println!("[AUDIO] Аудио система с рейтрейсингом инициализирована");
        
        Ok(Self {
            manager,
            sounds: SoundResources::new(),
            environment: EnvironmentAnalyzer::new(),
            current_modifiers: SoundModifiers::default(),
            block_checker: None,
            footstep_state: FootstepState::new(),
            jump_state: JumpState::new(),
        })
    }
    
    /// Установить функцию проверки твёрдости блока
    pub fn set_block_checker<F>(&mut self, checker: F)
    where
        F: Fn(i32, i32, i32) -> bool + Send + Sync + 'static,
    {
        self.block_checker = Some(Box::new(checker));
    }
    
    pub fn load_sounds(&mut self) -> Result<(), String> {
        self.sounds.load_all()
    }
    
    /// Проиграть звук установки блока
    pub fn play_place_block(&mut self) {
        systems::play_place_block(&mut self.manager, &self.sounds, &self.current_modifiers);
    }
    
    /// Обновить систему (вызывать каждый кадр)
    pub fn update(
        &mut self,
        player_pos: ultraviolet::Vec3,
        _player_forward: ultraviolet::Vec3,
        velocity_y: f32,
        is_moving: bool,
        is_on_ground: bool,
        is_sprinting: bool,
        is_jumping: bool,
        dt: f32,
    ) {
        // Анализируем окружение
        if let Some(ref checker) = self.block_checker {
            let env_params = self.environment.analyze(player_pos, dt, |x, y, z| checker(x, y, z));
            self.current_modifiers = SoundModifiers::from_environment(&env_params);
        }
        
        // Система шагов
        systems::footstep_system(
            &mut self.manager,
            &self.sounds,
            &mut self.footstep_state,
            player_pos,
            is_moving,
            is_on_ground,
            is_sprinting,
            &self.current_modifiers,
            dt,
        );
        
        // Система прыжков
        systems::jump_system(
            &mut self.manager,
            &self.sounds,
            &mut self.jump_state,
            is_on_ground,
            is_jumping,
            velocity_y,
            &self.current_modifiers,
            dt,
        );
    }
    
    /// Получить текущий тип окружения (для отладки)
    #[allow(dead_code)]
    pub fn current_environment(&self) -> EnvironmentType {
        self.environment.current_params().env_type
    }
}
