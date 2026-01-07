// ============================================
// Jump System - Система прыжков
// ============================================

use kira::{
    manager::AudioManager,
    sound::static_sound::StaticSoundSettings,
    Volume,
};

use crate::gpu::audio::{JumpState, SoundResources, SoundModifiers, rand_simple};

/// Система обработки прыжков
pub fn jump_system(
    audio: &mut AudioManager,
    sounds: &SoundResources,
    state: &mut JumpState,
    is_on_ground: bool,
    is_jumping: bool,
    velocity_y: f32,
    modifiers: &SoundModifiers,
    dt: f32,
) {
    // Обновление кулдауна
    if state.cooldown > 0.0 {
        state.cooldown -= dt;
    }
    
    // Детекция прыжка
    let just_jumped = state.was_on_ground && 
                      (!is_on_ground || is_jumping) && 
                      velocity_y > 0.5 &&
                      state.cooldown <= 0.0;
    
    if just_jumped {
        play_jump(audio, sounds, modifiers);
        state.cooldown = 0.3;
    }
    
    state.was_on_ground = is_on_ground;
}

/// Воспроизвести звук прыжка
fn play_jump(audio: &mut AudioManager, sounds: &SoundResources, modifiers: &SoundModifiers) {
    if let Some(ref sound_data) = sounds.jump {
        let volume_variation = 0.9 + rand_simple() * 0.2;
        let pitch_variation = 0.95 + rand_simple() * 0.1;
        
        let base_volume = 0.35 * volume_variation;
        let base_pitch = pitch_variation;
        
        let (volume, pitch) = modifiers.apply(base_volume, base_pitch);
        
        let settings = StaticSoundSettings::new()
            .volume(Volume::Amplitude(volume))
            .playback_rate(pitch);
        
        let _ = audio.play(sound_data.clone().with_settings(settings));
    }
}
