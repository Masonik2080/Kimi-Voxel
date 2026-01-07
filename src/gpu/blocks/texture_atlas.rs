// ============================================
// Block Texture Atlas
// ============================================
// Генерирует текстурный атлас из JSON-определений блоков

use super::{global_registry, BlockDefinition};
use super::definition::{TextureDef, PixelValue, FaceTextures};

/// Размер одной текстуры в атласе
pub const TEXTURE_SIZE: u32 = 16;
/// Максимум текстур в атласе (16x16 = 256 блоков)
pub const ATLAS_SIZE: u32 = 16;
/// Размер атласа в пикселях
pub const ATLAS_PIXELS: u32 = ATLAS_SIZE * TEXTURE_SIZE;

/// Текстурный атлас блоков
pub struct BlockTextureAtlas {
    /// RGBA данные атласа (ATLAS_PIXELS x ATLAS_PIXELS x 4)
    pub data: Vec<u8>,
    /// Маппинг block_id -> позиция в атласе (x, y)
    pub block_positions: std::collections::HashMap<u8, (u32, u32)>,
}

impl BlockTextureAtlas {
    /// Создать атлас из реестра блоков
    pub fn from_registry() -> Self {
        let mut atlas = Self {
            data: vec![0u8; (ATLAS_PIXELS * ATLAS_PIXELS * 4) as usize],
            block_positions: std::collections::HashMap::new(),
        };
        
        if let Ok(registry) = global_registry().read() {
            let mut slot = 0u32;
            
            for def in registry.all_blocks() {
                if def.numeric_id == 0 { continue; } // Skip air
                
                let atlas_x = slot % ATLAS_SIZE;
                let atlas_y = slot / ATLAS_SIZE;
                
                atlas.block_positions.insert(def.numeric_id, (atlas_x, atlas_y));
                atlas.render_block_texture(def, atlas_x, atlas_y);
                
                slot += 1;
                if slot >= ATLAS_SIZE * ATLAS_SIZE { break; }
            }
        }
        
        atlas
    }
    
    /// Рендерит текстуру блока в атлас
    fn render_block_texture(&mut self, def: &BlockDefinition, atlas_x: u32, atlas_y: u32) {
        let base_x = atlas_x * TEXTURE_SIZE;
        let base_y = atlas_y * TEXTURE_SIZE;
        
        // Получаем текстуру (side для отображения в инвентаре)
        let texture = self.get_side_texture(def);
        
        match texture {
            Some(tex) => self.render_texture_def(&tex, base_x, base_y),
            None => self.render_solid_color(def, base_x, base_y),
        }
    }
    
    /// Получить текстуру боковой грани
    fn get_side_texture(&self, def: &BlockDefinition) -> Option<TextureDef> {
        match &def.textures {
            Some(FaceTextures::All(tex)) => Some(tex.clone()),
            Some(FaceTextures::PerFace { side, north, .. }) => {
                side.clone().or_else(|| north.clone())
            }
            None => None,
        }
    }
    
    /// Рендерит TextureDef в атлас
    fn render_texture_def(&mut self, tex: &TextureDef, base_x: u32, base_y: u32) {
        match tex {
            TextureDef::Pixels { width, height, pixels } => {
                self.render_pixels(*width as u32, *height as u32, pixels, base_x, base_y);
            }
            TextureDef::Indexed { palette, indices, width, height } => {
                self.render_indexed(palette, indices, *width as u32, *height as u32, base_x, base_y);
            }
            TextureDef::Procedural { proc_type, params } => {
                self.render_procedural(proc_type, params, base_x, base_y);
            }
            TextureDef::File(_) => {
                // TODO: загрузка из файла
                self.fill_magenta(base_x, base_y);
            }
        }
    }
    
    /// Рендерит inline пиксели
    fn render_pixels(&mut self, width: u32, height: u32, pixels: &[PixelValue], base_x: u32, base_y: u32) {
        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                // Масштабируем если размер не 16x16
                let src_x = (x * width / TEXTURE_SIZE) as usize;
                let src_y = (y * height / TEXTURE_SIZE) as usize;
                let idx = src_y * width as usize + src_x;
                
                let rgba = if idx < pixels.len() {
                    pixels[idx].to_rgba()
                } else {
                    [255, 0, 255, 255] // Magenta for missing
                };
                
                self.set_pixel(base_x + x, base_y + y, rgba);
            }
        }
    }
    
    /// Рендерит индексированную текстуру
    fn render_indexed(&mut self, palette: &[PixelValue], indices: &[u8], width: u32, height: u32, base_x: u32, base_y: u32) {
        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let src_x = (x * width / TEXTURE_SIZE) as usize;
                let src_y = (y * height / TEXTURE_SIZE) as usize;
                let idx = src_y * width as usize + src_x;
                
                let rgba = if idx < indices.len() {
                    let palette_idx = indices[idx] as usize;
                    if palette_idx < palette.len() {
                        palette[palette_idx].to_rgba()
                    } else {
                        [255, 0, 255, 255]
                    }
                } else {
                    [255, 0, 255, 255]
                };
                
                self.set_pixel(base_x + x, base_y + y, rgba);
            }
        }
    }
    
    /// Рендерит процедурную текстуру
    fn render_procedural(&mut self, proc_type: &super::definition::ProceduralType, params: &super::definition::ProceduralParams, base_x: u32, base_y: u32) {
        use super::definition::ProceduralType;
        
        let color1 = params.color1.as_ref().map(|c| c.to_rgba()).unwrap_or([128, 128, 128, 255]);
        let color2 = params.color2.as_ref().map(|c| c.to_rgba()).unwrap_or([64, 64, 64, 255]);
        
        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let rgba = match proc_type {
                    ProceduralType::Checker => {
                        if (x / 2 + y / 2) % 2 == 0 { color1 } else { color2 }
                    }
                    ProceduralType::Noise => {
                        let noise = simple_hash(x + base_x * 100, y + base_y * 100);
                        if noise > 128 { color1 } else { color2 }
                    }
                    ProceduralType::Gradient => {
                        let t = y as f32 / TEXTURE_SIZE as f32;
                        lerp_color(color1, color2, t)
                    }
                    ProceduralType::Bricks => {
                        let brick_h = 4;
                        let brick_w = 8;
                        let mortar = 1;
                        let row = y / brick_h;
                        let offset = if row % 2 == 0 { 0 } else { brick_w / 2 };
                        let bx = (x + offset) % brick_w;
                        let by = y % brick_h;
                        if bx < mortar || by < mortar { color2 } else { color1 }
                    }
                    _ => color1,
                };
                
                self.set_pixel(base_x + x, base_y + y, rgba);
            }
        }
    }
    
    /// Заполняет solid цветом из определения блока
    fn render_solid_color(&mut self, def: &BlockDefinition, base_x: u32, base_y: u32) {
        let [r, g, b] = def.color.side();
        let rgba = [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255];
        
        // Добавляем простую текстуру - обводку
        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let edge = x == 0 || y == 0 || x == TEXTURE_SIZE - 1 || y == TEXTURE_SIZE - 1;
                let pixel = if edge {
                    [(rgba[0] as f32 * 0.7) as u8, (rgba[1] as f32 * 0.7) as u8, (rgba[2] as f32 * 0.7) as u8, 255]
                } else {
                    rgba
                };
                self.set_pixel(base_x + x, base_y + y, pixel);
            }
        }
    }
    
    /// Заполняет magenta (ошибка)
    fn fill_magenta(&mut self, base_x: u32, base_y: u32) {
        for y in 0..TEXTURE_SIZE {
            for x in 0..TEXTURE_SIZE {
                let checker = (x / 4 + y / 4) % 2 == 0;
                let rgba = if checker { [255, 0, 255, 255] } else { [0, 0, 0, 255] };
                self.set_pixel(base_x + x, base_y + y, rgba);
            }
        }
    }
    
    /// Устанавливает пиксель в атласе
    fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        let idx = ((y * ATLAS_PIXELS + x) * 4) as usize;
        if idx + 3 < self.data.len() {
            self.data[idx] = rgba[0];
            self.data[idx + 1] = rgba[1];
            self.data[idx + 2] = rgba[2];
            self.data[idx + 3] = rgba[3];
        }
    }
    
    /// Получить UV координаты для блока
    pub fn get_uv(&self, block_id: u8) -> Option<(f32, f32, f32, f32)> {
        self.block_positions.get(&block_id).map(|&(x, y)| {
            let u0 = x as f32 / ATLAS_SIZE as f32;
            let v0 = y as f32 / ATLAS_SIZE as f32;
            let u1 = (x + 1) as f32 / ATLAS_SIZE as f32;
            let v1 = (y + 1) as f32 / ATLAS_SIZE as f32;
            (u0, v0, u1, v1)
        })
    }
}

/// Простой хеш для процедурных текстур
fn simple_hash(x: u32, y: u32) -> u8 {
    let n = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
    let n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    ((n ^ (n >> 16)) & 0xFF) as u8
}

/// Линейная интерполяция цветов
fn lerp_color(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    [
        (a[0] as f32 * (1.0 - t) + b[0] as f32 * t) as u8,
        (a[1] as f32 * (1.0 - t) + b[1] as f32 * t) as u8,
        (a[2] as f32 * (1.0 - t) + b[2] as f32 * t) as u8,
        255,
    ]
}
