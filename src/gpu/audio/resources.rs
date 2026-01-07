// ============================================
// Audio Resources - Загруженные звуки (ECS)
// ============================================

use kira::sound::static_sound::StaticSoundData;

/// Ресурсы звуков - загруженные аудио данные
pub struct SoundResources {
    pub footstep: Option<StaticSoundData>,
    pub jump: Option<StaticSoundData>,
    pub place_block: Option<StaticSoundData>,
}

impl SoundResources {
    pub fn new() -> Self {
        Self {
            footstep: None,
            jump: None,
            place_block: None,
        }
    }
    
    /// Загрузить все звуки
    pub fn load_all(&mut self) -> Result<(), String> {
        self.load_footstep("assets/music/grass-foot-step.wav")?;
        self.load_jump("assets/music/jump.wav")?;
        self.load_place_block("assets/music/place.wav")?;
        Ok(())
    }
    
    fn load_footstep(&mut self, path: &str) -> Result<(), String> {
        match StaticSoundData::from_file(path) {
            Ok(sound) => {
                self.footstep = Some(sound);
                println!("[AUDIO] Загружен звук шага: {}", path);
                Ok(())
            }
            Err(e) => Err(format!("Failed to load footstep sound: {:?}", e))
        }
    }
    
    fn load_jump(&mut self, path: &str) -> Result<(), String> {
        match StaticSoundData::from_file(path) {
            Ok(sound) => {
                self.jump = Some(sound);
                println!("[AUDIO] Загружен звук прыжка: {}", path);
                Ok(())
            }
            Err(e) => Err(format!("Failed to load jump sound: {:?}", e))
        }
    }
    
    fn load_place_block(&mut self, path: &str) -> Result<(), String> {
        match StaticSoundData::from_file(path) {
            Ok(sound) => {
                self.place_block = Some(sound);
                println!("[AUDIO] Загружен звук установки блока: {}", path);
                Ok(())
            }
            Err(e) => Err(format!("Failed to load place block sound: {:?}", e))
        }
    }
}

impl Default for SoundResources {
    fn default() -> Self {
        Self::new()
    }
}
