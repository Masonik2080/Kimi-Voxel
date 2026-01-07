// ============================================
// Place Block System - Система установки блоков
// ============================================

use kira::{
    manager::AudioManager,
    sound::static_sound::StaticSoundSettings,
    Volume,
};

use crate::gpu::audio::{SoundResources, SoundModifiers, rand_simple};

/// Воспроизвести звук установки блока
pub fn play_place_block(
    audio: &mut AudioManager,
    sounds: &SoundResources,
    modifiers: &SoundModifiers,
) {
    if let Some(ref sound_data) = sounds.place_block {
        let volume_variation = 0.9 + rand_simple() * 0.2;
        let pitch_variation = 0.95 + rand_simple() * 0.1;
        
        let base_volume = 0.4 * volume_variation;
        let base_pitch = pitch_variation;
        
        let (volume, pitch) = modifiers.apply(base_volume, base_pitch);
        
        let settings = StaticSoundSettings::new()
            .volume(Volume::Amplitude(volume))
            .playback_rate(pitch);
        
        let _ = audio.play(sound_data.clone().with_settings(settings));
    }
}
