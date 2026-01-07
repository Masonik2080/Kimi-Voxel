// ============================================
// Audio Utils - Утилиты
// ============================================

use std::time::{SystemTime, UNIX_EPOCH};

/// Простой псевдо-рандом без зависимостей
pub fn rand_simple() -> f32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 1000) as f32 / 1000.0
}
