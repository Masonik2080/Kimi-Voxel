// ============================================
// Block Registry - Data-Driven из JSON
// ============================================
// Единый источник правды для всех блоков

use std::collections::HashMap;
use std::path::Path;
use std::fs;
use std::sync::{OnceLock, RwLock};

use super::definition::{BlockDefinition, BlocksFile, BlockCategory, ColorDef};

/// Динамический реестр блоков
pub struct BlockRegistry {
    /// Блоки по string ID
    blocks_by_id: HashMap<String, BlockDefinition>,
    /// Блоки по numeric ID
    blocks_by_numeric: HashMap<u8, BlockDefinition>,
    /// Маппинг string ID -> numeric ID
    id_to_numeric: HashMap<String, u8>,
    /// Маппинг numeric ID -> string ID
    numeric_to_id: HashMap<u8, String>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            blocks_by_id: HashMap::new(),
            blocks_by_numeric: HashMap::new(),
            id_to_numeric: HashMap::new(),
            numeric_to_id: HashMap::new(),
        }
    }
    
    /// Загрузить блоки из JSON строки
    pub fn load_from_json(&mut self, json: &str) -> Result<usize, String> {
        let blocks_file: BlocksFile = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
        let count = blocks_file.blocks.len();
        for block in blocks_file.blocks {
            self.register(block);
        }
        Ok(count)
    }
    
    /// Загрузить блоки из файла
    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<usize, String> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read file: {}", e))?;
        self.load_from_json(&content)
    }
    
    /// Загрузить все JSON из директории
    pub fn load_from_directory<P: AsRef<Path>>(&mut self, dir: P) -> Result<usize, String> {
        let dir = dir.as_ref();
        if !dir.exists() { return Ok(0); }
        
        let mut total = 0;
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(count) = self.load_from_file(&path) {
                    total += count;
                }
            }
        }
        Ok(total)
    }
    
    /// Зарегистрировать блок
    pub fn register(&mut self, block: BlockDefinition) {
        let id = block.id.clone();
        let numeric = block.numeric_id;
        
        self.id_to_numeric.insert(id.clone(), numeric);
        self.numeric_to_id.insert(numeric, id.clone());
        self.blocks_by_id.insert(id, block.clone());
        self.blocks_by_numeric.insert(numeric, block);
    }
    
    /// Получить блок по string ID
    pub fn get(&self, id: &str) -> Option<&BlockDefinition> {
        self.blocks_by_id.get(id)
    }
    
    /// Получить блок по numeric ID
    pub fn get_by_numeric(&self, id: u8) -> Option<&BlockDefinition> {
        self.blocks_by_numeric.get(&id)
    }
    
    /// Получить numeric ID по string ID
    pub fn get_numeric_id(&self, id: &str) -> Option<u8> {
        self.id_to_numeric.get(id).copied()
    }
    
    /// Получить string ID по numeric ID  
    pub fn get_string_id(&self, numeric: u8) -> Option<&str> {
        self.numeric_to_id.get(&numeric).map(|s| s.as_str())
    }
    
    /// Все блоки
    pub fn all_blocks(&self) -> impl Iterator<Item = &BlockDefinition> {
        self.blocks_by_id.values()
    }
    
    /// Блоки по категории
    pub fn blocks_by_category(&self, category: BlockCategory) -> Vec<&BlockDefinition> {
        self.blocks_by_id.values()
            .filter(|b| b.category == category)
            .collect()
    }
    
    /// Количество блоков
    pub fn count(&self) -> usize {
        self.blocks_by_id.len()
    }
}

impl Default for BlockRegistry {
    fn default() -> Self { Self::new() }
}

// ============================================
// Global Registry Singleton
// ============================================

static GLOBAL_REGISTRY: OnceLock<RwLock<BlockRegistry>> = OnceLock::new();

/// Получить глобальный реестр блоков
pub fn global_registry() -> &'static RwLock<BlockRegistry> {
    GLOBAL_REGISTRY.get_or_init(|| {
        let mut registry = BlockRegistry::new();
        
        // Загружаем из встроенного JSON (default_blocks.json)
        if let Err(e) = registry.load_from_json(include_str!("../../../assets/blocks/default_blocks.json")) {
            log::warn!("Failed to load default blocks: {}", e);
            register_fallback_blocks(&mut registry);
        }
        
        // Загружаем моды
        let _ = registry.load_from_json(include_str!("../../../assets/blocks/example_mod.json"));
        let _ = registry.load_from_json(include_str!("../../../assets/blocks/street_art.json"));
        
        RwLock::new(registry)
    })
}

/// Инициализировать с модами
pub fn init_registry_with_mods<P: AsRef<Path>>(mods_dir: P) -> Result<(), String> {
    let registry = global_registry();
    let mut reg = registry.write().map_err(|_| "Lock poisoned")?;
    reg.load_from_directory(mods_dir)?;
    Ok(())
}

/// Fallback блоки если JSON не загрузился
fn register_fallback_blocks(registry: &mut BlockRegistry) {
    registry.register(BlockDefinition {
        id: "air".to_string(),
        numeric_id: 0,
        name: "Air".to_string(),
        color: ColorDef::Uniform([0.0, 0.0, 0.0]),
        transparent: true,
        solid: false,
        breakable: false,
        ..Default::default()
    });
    
    registry.register(BlockDefinition {
        id: "stone".to_string(),
        numeric_id: 1,
        name: "Stone".to_string(),
        color: ColorDef::Uniform([0.5, 0.5, 0.52]),
        ..Default::default()
    });
    
    registry.register(BlockDefinition {
        id: "grass".to_string(),
        numeric_id: 3,
        name: "Grass".to_string(),
        color: ColorDef::PerFace {
            top: [0.36, 0.60, 0.28],
            side: [0.55, 0.40, 0.26],
            bottom: [0.55, 0.40, 0.26],
        },
        ..Default::default()
    });
}
