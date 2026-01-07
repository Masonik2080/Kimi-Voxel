// ============================================
// Data-Driven Block Definition
// ============================================
// Структуры для загрузки блоков из JSON

use serde::{Deserialize, Serialize};

// ============================================
// Texture Definition - пиксельные текстуры
// ============================================

/// Размер текстуры (в пикселях)
pub const TEXTURE_SIZE: usize = 16;

/// Один пиксель текстуры [r, g, b, a] (0-255)
pub type PixelRGBA = [u8; 4];

/// Текстура грани блока
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextureDef {
    /// Ссылка на PNG файл
    File(String),
    
    /// Inline пиксели (16x16 = 256 пикселей, row-major)
    /// Каждый пиксель: [r, g, b, a] или [r, g, b] или "#RRGGBB" или "#RRGGBBAA"
    Pixels {
        width: u8,
        height: u8,
        pixels: Vec<PixelValue>,
    },
    
    /// Палитра + индексы (компактный формат)
    Indexed {
        palette: Vec<PixelValue>,
        /// Индексы в палитру (width * height)
        indices: Vec<u8>,
        width: u8,
        height: u8,
    },
    
    /// Процедурная текстура
    Procedural {
        #[serde(rename = "type")]
        proc_type: ProceduralType,
        params: ProceduralParams,
    },
}

/// Значение пикселя (гибкий формат)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PixelValue {
    /// RGB массив [r, g, b]
    RGB([u8; 3]),
    /// RGBA массив [r, g, b, a]
    RGBA([u8; 4]),
    /// Hex строка "#RRGGBB" или "#RRGGBBAA"
    Hex(String),
    /// Индекс в палитру
    Index(u8),
}

impl PixelValue {
    pub fn to_rgba(&self) -> PixelRGBA {
        match self {
            PixelValue::RGB([r, g, b]) => [*r, *g, *b, 255],
            PixelValue::RGBA(rgba) => *rgba,
            PixelValue::Hex(s) => parse_hex_color(s),
            PixelValue::Index(_) => [255, 0, 255, 255], // Magenta = error
        }
    }
    
    pub fn to_rgb_f32(&self) -> [f32; 3] {
        let [r, g, b, _] = self.to_rgba();
        [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
    }
}

fn parse_hex_color(s: &str) -> PixelRGBA {
    let s = s.trim_start_matches('#');
    match s.len() {
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(255);
            [r, g, b, 255]
        }
        8 => {
            let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(255);
            let a = u8::from_str_radix(&s[6..8], 16).unwrap_or(255);
            [r, g, b, a]
        }
        _ => [255, 0, 255, 255], // Magenta = error
    }
}

/// Типы процедурных текстур
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProceduralType {
    Noise,
    Gradient,
    Checker,
    Bricks,
    Planks,
    Ore,
}

/// Параметры процедурной текстуры
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProceduralParams {
    #[serde(default)]
    pub color1: Option<PixelValue>,
    #[serde(default)]
    pub color2: Option<PixelValue>,
    #[serde(default)]
    pub scale: Option<f32>,
    #[serde(default)]
    pub seed: Option<u32>,
    #[serde(default)]
    pub variation: Option<f32>,
}

impl Default for TextureDef {
    fn default() -> Self {
        TextureDef::File("missing.png".to_string())
    }
}

// ============================================
// Face Textures - текстуры для каждой грани
// ============================================

/// Текстуры граней блока
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FaceTextures {
    /// Одна текстура для всех граней
    All(TextureDef),
    
    /// Разные текстуры для граней
    PerFace {
        #[serde(default)]
        top: Option<TextureDef>,
        #[serde(default)]
        bottom: Option<TextureDef>,
        #[serde(default)]
        north: Option<TextureDef>,
        #[serde(default)]
        south: Option<TextureDef>,
        #[serde(default)]
        east: Option<TextureDef>,
        #[serde(default)]
        west: Option<TextureDef>,
        /// Fallback для сторон
        #[serde(default)]
        side: Option<TextureDef>,
    },
}

impl Default for FaceTextures {
    fn default() -> Self {
        FaceTextures::All(TextureDef::default())
    }
}

// ============================================
// Color Definition (legacy + simple cases)
// ============================================

/// Определение цвета блока из JSON (для простых случаев)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColorDef {
    /// Один цвет для всех граней [r, g, b]
    Uniform([f32; 3]),
    /// Разные цвета для граней { top, side, bottom }
    PerFace {
        top: [f32; 3],
        side: [f32; 3],
        #[serde(default = "default_bottom")]
        bottom: [f32; 3],
    },
}

fn default_bottom() -> [f32; 3] {
    [0.5, 0.5, 0.5]
}

impl Default for ColorDef {
    fn default() -> Self {
        ColorDef::Uniform([0.5, 0.5, 0.5])
    }
}

impl ColorDef {
    pub fn top(&self) -> [f32; 3] {
        match self {
            ColorDef::Uniform(c) => *c,
            ColorDef::PerFace { top, .. } => *top,
        }
    }
    
    pub fn side(&self) -> [f32; 3] {
        match self {
            ColorDef::Uniform(c) => *c,
            ColorDef::PerFace { side, .. } => *side,
        }
    }
    
    pub fn bottom(&self) -> [f32; 3] {
        match self {
            ColorDef::Uniform(c) => *c,
            ColorDef::PerFace { bottom, .. } => *bottom,
        }
    }
}

/// Категория блока (для организации в инвентаре)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BlockCategory {
    #[default]
    Basic,
    Stone,
    Ore,
    Wood,
    Nature,
    Building,
    Metal,
}

/// Звуки блока
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockSounds {
    #[serde(default)]
    pub place: Option<String>,
    #[serde(default)]
    pub break_sound: Option<String>,
    #[serde(default)]
    pub step: Option<String>,
}

/// Определение блока из JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDefinition {
    /// Уникальный ID блока (string, например "minecraft:stone")
    pub id: String,
    
    /// Числовой ID для сериализации (0-255)
    pub numeric_id: u8,
    
    /// Отображаемое имя
    pub name: String,
    
    /// Цвет(а) блока
    #[serde(default)]
    pub color: ColorDef,
    
    /// Твёрдость (время ломания базовым инструментом)
    #[serde(default = "default_hardness")]
    pub hardness: f32,
    
    /// Прозрачный ли блок
    #[serde(default)]
    pub transparent: bool,
    
    /// Излучает ли свет
    #[serde(default)]
    pub emissive: bool,
    
    /// Уровень света (0-15)
    #[serde(default)]
    pub light_level: u8,
    
    /// Твёрдый ли блок (для коллизий)
    #[serde(default = "default_true")]
    pub solid: bool,
    
    /// Можно ли сломать
    #[serde(default = "default_true")]
    pub breakable: bool,
    
    /// Категория
    #[serde(default)]
    pub category: BlockCategory,
    
    /// Текстуры граней (пиксельные)
    #[serde(default)]
    pub textures: Option<FaceTextures>,
    
    /// Звуки
    #[serde(default)]
    pub sounds: BlockSounds,
    
    /// Дополнительные теги для модов
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_hardness() -> f32 { 1.0 }
fn default_true() -> bool { true }

impl Default for BlockDefinition {
    fn default() -> Self {
        Self {
            id: "unknown".to_string(),
            numeric_id: 0,
            name: "Unknown".to_string(),
            color: ColorDef::default(),
            hardness: 1.0,
            transparent: false,
            emissive: false,
            light_level: 0,
            solid: true,
            breakable: true,
            category: BlockCategory::Basic,
            textures: None,
            sounds: BlockSounds::default(),
            tags: Vec::new(),
        }
    }
}

/// Файл с определениями блоков
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocksFile {
    /// Версия формата
    #[serde(default = "default_version")]
    pub version: String,
    
    /// Список блоков
    pub blocks: Vec<BlockDefinition>,
}

fn default_version() -> String { "1.0".to_string() }
