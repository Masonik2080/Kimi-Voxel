// ============================================
// Block Types - Data-Driven Architecture
// ============================================
// BlockType = u8 (numeric_id). Все данные из JSON.

/// BlockType - просто numeric_id блока
pub type BlockType = u8;

// Константы для всех блоков (соответствуют numeric_id в JSON)
pub const AIR: BlockType = 0;
pub const STONE: BlockType = 1;
pub const DIRT: BlockType = 2;
pub const GRASS: BlockType = 3;
pub const SAND: BlockType = 4;
pub const GRAVEL: BlockType = 5;
pub const COBBLESTONE: BlockType = 10;
pub const MOSSY_COBBLESTONE: BlockType = 11;
pub const GRANITE: BlockType = 12;
pub const DIORITE: BlockType = 13;
pub const ANDESITE: BlockType = 14;
pub const DEEPSLATE: BlockType = 15;
pub const COAL_ORE: BlockType = 20;
pub const IRON_ORE: BlockType = 21;
pub const GOLD_ORE: BlockType = 22;
pub const DIAMOND_ORE: BlockType = 23;
pub const EMERALD_ORE: BlockType = 24;
pub const REDSTONE_ORE: BlockType = 25;
pub const LAPIS_ORE: BlockType = 26;
pub const COPPER_ORE: BlockType = 27;
pub const OAK_LOG: BlockType = 30;
pub const OAK_PLANKS: BlockType = 31;
pub const OAK_LEAVES: BlockType = 32;
pub const BIRCH_LOG: BlockType = 33;
pub const BIRCH_PLANKS: BlockType = 34;
pub const BIRCH_LEAVES: BlockType = 35;
pub const SPRUCE_LOG: BlockType = 36;
pub const SPRUCE_PLANKS: BlockType = 37;
pub const SPRUCE_LEAVES: BlockType = 38;
pub const WATER: BlockType = 50;
pub const LAVA: BlockType = 51;
pub const ICE: BlockType = 52;
pub const SNOW: BlockType = 53;
pub const CLAY: BlockType = 54;
pub const BRICKS: BlockType = 60;
pub const STONE_BRICKS: BlockType = 61;
pub const OBSIDIAN: BlockType = 62;
pub const GLASS: BlockType = 63;
pub const IRON_BLOCK: BlockType = 70;
pub const GOLD_BLOCK: BlockType = 71;
pub const DIAMOND_BLOCK: BlockType = 72;
pub const EMERALD_BLOCK: BlockType = 73;
pub const COPPER_BLOCK: BlockType = 74;

// Custom blocks (100+)
pub const CUSTOM_100: BlockType = 100;
pub const CUSTOM_101: BlockType = 101;
pub const CUSTOM_102: BlockType = 102;
pub const CUSTOM_103: BlockType = 103;
pub const CUSTOM_104: BlockType = 104;

/// Проверка: блок твёрдый?
#[inline]
pub fn is_solid(block: BlockType) -> bool {
    block != AIR && block != WATER && block != GLASS
}

/// Проверка: блок прозрачный?
#[inline]
pub fn is_transparent(block: BlockType) -> bool {
    matches!(block, AIR | WATER | GLASS | OAK_LEAVES | BIRCH_LEAVES | SPRUCE_LEAVES)
}

/// Получить цвет блока из реестра
#[inline]
pub fn get_block_color(block: BlockType) -> [f32; 3] {
    if let Ok(registry) = super::global_registry().read() {
        if let Some(def) = registry.get_by_numeric(block) {
            return def.color.top();
        }
    }
    [0.5, 0.5, 0.5]
}

/// Получить цвета граней (top, side) из реестра
#[inline]
pub fn get_face_colors(block: BlockType) -> ([f32; 3], [f32; 3]) {
    if let Ok(registry) = super::global_registry().read() {
        if let Some(def) = registry.get_by_numeric(block) {
            return (def.color.top(), def.color.side());
        }
    }
    ([0.5, 0.5, 0.5], [0.4, 0.4, 0.4])
}

/// Получить имя блока из реестра
#[inline]
pub fn get_block_name(block: BlockType) -> String {
    if let Ok(registry) = super::global_registry().read() {
        if let Some(def) = registry.get_by_numeric(block) {
            return def.name.clone();
        }
    }
    format!("block_{}", block)
}

/// Получить hardness блока
#[inline]
pub fn get_block_hardness(block: BlockType) -> f32 {
    if let Ok(registry) = super::global_registry().read() {
        if let Some(def) = registry.get_by_numeric(block) {
            return def.hardness;
        }
    }
    1.0
}
