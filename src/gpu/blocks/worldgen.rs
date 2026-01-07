// ============================================
// World Generation Block Resolver
// ============================================
// Получает блоки для генерации мира из реестра по string ID

use super::{BlockType, global_registry};
use super::types::*;

/// Кэшированные ID блоков для быстрого доступа при генерации
pub struct WorldGenBlocks {
    pub bedrock: BlockType,
    pub stone: BlockType,
    pub dirt: BlockType,
    pub air: BlockType,
    pub sand: BlockType,
    pub grass: BlockType,
    pub snow: BlockType,
    pub coal_ore: BlockType,
    pub iron_ore: BlockType,
    pub gold_ore: BlockType,
    pub diamond_ore: BlockType,
    pub copper_ore: BlockType,
    pub water: BlockType,
    pub lava: BlockType,
}

impl Default for WorldGenBlocks {
    fn default() -> Self { Self::new() }
}

impl WorldGenBlocks {
    pub fn new() -> Self {
        Self {
            bedrock: resolve_block("deepslate"),
            stone: resolve_block("stone"),
            dirt: resolve_block("dirt"),
            air: resolve_block("air"),
            sand: resolve_block("sand"),
            grass: resolve_block("grass"),
            snow: resolve_block("snow"),
            coal_ore: resolve_block("coal_ore"),
            iron_ore: resolve_block("iron_ore"),
            gold_ore: resolve_block("gold_ore"),
            diamond_ore: resolve_block("diamond_ore"),
            copper_ore: resolve_block("copper_ore"),
            water: resolve_block("water"),
            lava: resolve_block("lava"),
        }
    }
    
    #[inline]
    pub fn surface_block(&self, height: f32) -> BlockType {
        if height < 8.0 { self.sand }
        else if height < 30.0 { self.grass }
        else if height < 50.0 { self.stone }
        else { self.snow }
    }
    
    #[inline]
    pub fn subsurface_block(&self, height: f32) -> BlockType {
        if height < 8.0 { self.sand }
        else if height >= 50.0 { self.stone }
        else { self.dirt }
    }
    
    #[inline]
    pub fn block_at_depth(&self, y: i32, surface_y: i32, surface_height: f32) -> BlockType {
        if y > surface_y { self.air }
        else if y < -29 { self.bedrock }
        else if y < surface_y - 4 { self.stone }
        else if y < surface_y { self.subsurface_block(surface_height) }
        else { self.surface_block(surface_height) }
    }
}

/// Резолвит string ID в BlockType через реестр
pub fn resolve_block(id: &str) -> BlockType {
    if let Ok(registry) = global_registry().read() {
        if let Some(numeric_id) = registry.get_numeric_id(id) {
            return numeric_id;
        }
    }
    // Fallback
    match id {
        "air" => AIR,
        "stone" => STONE,
        "dirt" => DIRT,
        "grass" => GRASS,
        "sand" => SAND,
        "gravel" => GRAVEL,
        "deepslate" => DEEPSLATE,
        "snow" => SNOW,
        "water" => WATER,
        "lava" => LAVA,
        "coal_ore" => COAL_ORE,
        "iron_ore" => IRON_ORE,
        "gold_ore" => GOLD_ORE,
        "diamond_ore" => DIAMOND_ORE,
        "copper_ore" => COPPER_ORE,
        _ => STONE,
    }
}

/// Резолвит BlockType в string ID
pub fn block_to_id(block: BlockType) -> &'static str {
    if let Ok(registry) = global_registry().read() {
        if let Some(id) = registry.get_string_id(block) {
            return Box::leak(id.to_string().into_boxed_str());
        }
    }
    match block {
        AIR => "air",
        STONE => "stone",
        DIRT => "dirt",
        GRASS => "grass",
        SAND => "sand",
        GRAVEL => "gravel",
        DEEPSLATE => "deepslate",
        SNOW => "snow",
        WATER => "water",
        LAVA => "lava",
        _ => "unknown",
    }
}

use std::sync::OnceLock;
static WORLDGEN_BLOCKS: OnceLock<WorldGenBlocks> = OnceLock::new();

pub fn worldgen_blocks() -> &'static WorldGenBlocks {
    WORLDGEN_BLOCKS.get_or_init(WorldGenBlocks::new)
}
