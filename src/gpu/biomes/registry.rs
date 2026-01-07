// ============================================
// Biome Registry - Реестр биомов
// ============================================

use super::types::*;
use crate::gpu::blocks::{SAND, STONE, GRASS, DIRT, SNOW, BlockType};
use std::sync::OnceLock;

/// Реестр всех биомов
pub struct BiomeRegistry {
    biomes: Vec<BiomeDefinition>,
}

impl BiomeRegistry {
    pub fn new() -> Self {
        let mut registry = Self { biomes: Vec::new() };
        registry.register_default_biomes();
        registry
    }

    fn register_default_biomes(&mut self) {
        // Океан - глубоко под водой
        self.register(
            BiomeDefinition::new(BIOME_OCEAN, "ocean", SAND, SAND, STONE)
                .with_terrain(-15.0, 5.0, TerrainType::Ocean)
                .with_climate(0.5, 1.0)
        );

        // Равнины - стандартный биом (редкие деревья)
        self.register(
            BiomeDefinition::new(BIOME_PLAINS, "plains", GRASS, DIRT, STONE)
                .with_terrain(20.0, 8.0, TerrainType::Rolling)
                .with_climate(0.5, 0.4)
                .with_trees(0.001)
        );

        // Пустыня - жарко и сухо (без деревьев)
        self.register(
            BiomeDefinition::new(BIOME_DESERT, "desert", SAND, SAND, STONE)
                .with_terrain(22.0, 6.0, TerrainType::Rolling)
                .with_climate(0.9, 0.1)
        );

        // Лес - умеренный и влажный (много деревьев)
        self.register(
            BiomeDefinition::new(BIOME_FOREST, "forest", GRASS, DIRT, STONE)
                .with_terrain(25.0, 12.0, TerrainType::Rolling)
                .with_climate(0.5, 0.6)
                .with_trees(0.015)
        );

        // Тайга - холодный лес (ели)
        self.register(
            BiomeDefinition::new(BIOME_TAIGA, "taiga", GRASS, DIRT, STONE)
                .with_terrain(22.0, 10.0, TerrainType::Rolling)
                .with_climate(0.25, 0.6)
                .with_trees(0.012)
        );

        // Тундра - холодно и сухо (редкие ели)
        self.register(
            BiomeDefinition::new(BIOME_TUNDRA, "tundra", SNOW, DIRT, STONE)
                .with_terrain(18.0, 4.0, TerrainType::Flat)
                .with_climate(0.0, 0.3)
                .with_trees(0.002)
        );

        // Болото - плоское, чуть ниже воды
        self.register(
            BiomeDefinition::new(BIOME_SWAMP, "swamp", GRASS, DIRT, STONE)
                .with_terrain(8.0, 2.0, TerrainType::Flat)
                .with_climate(0.6, 0.9)
                .with_trees(0.008)
        );

        // Горы - плавные величественные склоны (без деревьев)
        self.register(
            BiomeDefinition::new(BIOME_MOUNTAINS, "mountains", STONE, STONE, STONE)
                .with_terrain(25.0, 60.0, TerrainType::Mountains3D)
                .with_climate(0.3, 0.3)
                .with_3d_noise(0.2)
        );

        // Саванна - жарко, умеренно сухо (редкие деревья)
        self.register(
            BiomeDefinition::new(BIOME_SAVANNA, "savanna", GRASS, DIRT, STONE)
                .with_terrain(20.0, 5.0, TerrainType::Flat)
                .with_climate(0.8, 0.3)
                .with_trees(0.002)
        );

        // Джунгли - жарко и очень влажно (очень много деревьев)
        self.register(
            BiomeDefinition::new(BIOME_JUNGLE, "jungle", GRASS, DIRT, STONE)
                .with_terrain(28.0, 15.0, TerrainType::Rolling)
                .with_climate(0.9, 0.9)
                .with_trees(0.025)
        );
    }

    pub fn register(&mut self, biome: BiomeDefinition) {
        let id = biome.id as usize;
        if id >= self.biomes.len() {
            self.biomes.resize(id + 1, self.biomes.first().cloned().unwrap_or_else(|| {
                BiomeDefinition::new(0, "unknown", STONE, STONE, STONE)
                    .with_terrain(20.0, 10.0, TerrainType::Rolling)
            }));
        }
        self.biomes[id] = biome;
    }

    #[inline]
    pub fn get(&self, id: BiomeId) -> &BiomeDefinition {
        self.biomes.get(id as usize).unwrap_or(&self.biomes[0])
    }

    pub fn count(&self) -> usize {
        self.biomes.len()
    }
}

impl Default for BiomeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

static BIOME_REGISTRY: OnceLock<BiomeRegistry> = OnceLock::new();

pub fn biome_registry() -> &'static BiomeRegistry {
    BIOME_REGISTRY.get_or_init(BiomeRegistry::new)
}
