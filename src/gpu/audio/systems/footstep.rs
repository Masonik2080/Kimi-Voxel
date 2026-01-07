// ============================================
// Footstep System - Система шагов
// ============================================

use kira::{
    manager::AudioManager,
    sound::static_sound::StaticSoundSettings,
    Volume,
};
use ultraviolet::Vec3;

use crate::gpu::audio::{FootstepState, SoundResources, SoundModifiers, rand_simple};

/// Система обработки шагов
pub fn footstep_system(
    audio: &mut AudioManager,
    sounds: &SoundResources,
    state: &mut FootstepState,
    player_pos: Vec3,
    is_moving: bool,
    is_on_ground: bool,
    is_sprinting: bool,
    modifiers: &SoundModifiers,
    dt: f32,
) {
    state.time_since_last_step += dt;
    
    // Первый кадр - инициализация позиции
    if state.first_frame {
        state.first_frame = false;
        state.last_position = player_pos;
        return;
    }
    
    // Расчёт горизонтального движения
    let movement = player_pos - state.last_position;
    let horizontal_movement = Vec3::new(movement.x, 0.0, movement.z);
    let distance = horizontal_movement.mag();
    state.last_position = player_pos;
    
    // Не играем звук если не на земле или не двигаемся
    if !is_on_ground || !is_moving || distance < 0.001 {
        return;
    }
    
    state.distance_traveled += distance;
    
    // Параметры шага
    let step_distance = if is_sprinting { 2.8 } else { 3.5 };
    let min_interval = if is_sprinting { 0.28 } else { 0.4 };
    
    // Проверка на воспроизведение
    if state.distance_traveled >= step_distance && state.time_since_last_step >= min_interval {
        state.distance_traveled = 0.0;
        state.time_since_last_step = 0.0;
        play_footstep(audio, sounds, modifiers);
    }
}

/// Воспроизвести звук шага
fn play_footstep(audio: &mut AudioManager, sounds: &SoundResources, modifiers: &SoundModifiers) {
    if let Some(ref sound_data) = sounds.footstep {
        let volume_variation = 0.85 + rand_simple() * 0.3;
        let pitch_variation = 0.92 + rand_simple() * 0.16;
        
        let base_volume = 0.25 * volume_variation;
        let base_pitch = pitch_variation;
        
        let (volume, pitch) = modifiers.apply(base_volume, base_pitch);
        
        let settings = StaticSoundSettings::new()
            .volume(Volume::Amplitude(volume))
            .playback_rate(pitch);
        
        let _ = audio.play(sound_data.clone().with_settings(settings));
    }
}
