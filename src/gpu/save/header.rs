// ============================================
// Save Header - Заголовок файла сохранения
// ============================================

use serde::{Serialize, Deserialize};

/// Магическое число "RUST" в ASCII
pub const MAGIC_NUMBER: [u8; 4] = [0x52, 0x55, 0x53, 0x54];

/// Версия формата сохранения
pub const SAVE_VERSION: u32 = 1;

/// Заголовок файла сохранения (28 байт)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveHeader {
    /// Магическое число для валидации
    pub magic: [u8; 4],
    /// Версия формата
    pub version: u32,
    /// Сид мира
    pub seed: u64,
    /// Позиция игрока
    pub player_pos: [f32; 3],
}

impl SaveHeader {
    pub fn new(seed: u64, player_pos: [f32; 3]) -> Self {
        Self {
            magic: MAGIC_NUMBER,
            version: SAVE_VERSION,
            seed,
            player_pos,
        }
    }

    /// Проверка валидности заголовка
    pub fn is_valid(&self) -> bool {
        self.magic == MAGIC_NUMBER && self.version == SAVE_VERSION
    }
}

impl Default for SaveHeader {
    fn default() -> Self {
        Self::new(0, [0.0, 64.0, 0.0])
    }
}
