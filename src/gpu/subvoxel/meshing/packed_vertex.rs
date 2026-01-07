// ============================================
// Packed Vertex - Компактный формат вершины
// ============================================
//
// Вместо 36 байт (3x f32 position + 3x f32 normal + 3x f32 color)
// используем 8 байт:
// - Position: 3x u8 (относительно чанка, 0-255)
// - Normal: 1 байт (индекс 0-5, куб имеет 6 нормалей)
// - Color: u32 RGBA8
//
// Экономия: 4.5x меньше bandwidth GPU

use bytemuck::{Pod, Zeroable};

/// Упакованная вершина субвокселя (8 байт)
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PackedVertex {
    /// Позиция относительно чанка (x, y, z в субвоксельных единицах 0-255)
    pub pos_x: u8,
    pub pos_y: u8,
    pub pos_z: u8,
    /// Индекс нормали (0-5) + флаги
    pub normal_flags: u8,
    /// Цвет RGBA8
    pub color: u32,
}

/// Индексы нормалей
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalIndex {
    PosX = 0,
    NegX = 1,
    PosY = 2,
    NegY = 3,
    PosZ = 4,
    NegZ = 5,
}

impl NormalIndex {
    #[inline]
    pub fn to_vec3(&self) -> [f32; 3] {
        match self {
            NormalIndex::PosX => [1.0, 0.0, 0.0],
            NormalIndex::NegX => [-1.0, 0.0, 0.0],
            NormalIndex::PosY => [0.0, 1.0, 0.0],
            NormalIndex::NegY => [0.0, -1.0, 0.0],
            NormalIndex::PosZ => [0.0, 0.0, 1.0],
            NormalIndex::NegZ => [0.0, 0.0, -1.0],
        }
    }
}

impl PackedVertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Uint8x4,  // pos_x, pos_y, pos_z, normal_flags
        1 => Uint32,   // color
        2 => Uint32,   // padding/reserved (для выравнивания)
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    #[inline]
    pub fn new(pos: [u8; 3], normal: NormalIndex, color: [u8; 4]) -> Self {
        Self {
            pos_x: pos[0],
            pos_y: pos[1],
            pos_z: pos[2],
            normal_flags: normal as u8,
            color: u32::from_le_bytes(color),
        }
    }

    /// Создать из float координат (конвертирует в u8)
    #[inline]
    pub fn from_float(
        x: f32, y: f32, z: f32,
        normal: NormalIndex,
        r: f32, g: f32, b: f32,
    ) -> Self {
        Self {
            pos_x: (x * 4.0).clamp(0.0, 255.0) as u8,
            pos_y: (y * 4.0).clamp(0.0, 255.0) as u8,
            pos_z: (z * 4.0).clamp(0.0, 255.0) as u8,
            normal_flags: normal as u8,
            color: pack_color(r, g, b, 1.0),
        }
    }
}

/// Упаковать цвет в u32 RGBA8
#[inline]
pub fn pack_color(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let r = (r.clamp(0.0, 1.0) * 255.0) as u8;
    let g = (g.clamp(0.0, 1.0) * 255.0) as u8;
    let b = (b.clamp(0.0, 1.0) * 255.0) as u8;
    let a = (a.clamp(0.0, 1.0) * 255.0) as u8;
    u32::from_le_bytes([r, g, b, a])
}

/// Распаковать цвет из u32
#[inline]
pub fn unpack_color(packed: u32) -> [f32; 4] {
    let bytes = packed.to_le_bytes();
    [
        bytes[0] as f32 / 255.0,
        bytes[1] as f32 / 255.0,
        bytes[2] as f32 / 255.0,
        bytes[3] as f32 / 255.0,
    ]
}

// ============================================
// Ещё более компактный формат: 4 байта на вершину
// ============================================

/// Ультра-компактная вершина (4 байта)
/// Для случаев когда нужна максимальная экономия
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MicroVertex {
    /// Биты 0-7: X (0-255)
    /// Биты 8-15: Y (0-255)  
    /// Биты 16-23: Z (0-255)
    /// Биты 24-26: Normal index (0-5)
    /// Биты 27-31: Color palette index (0-31)
    pub packed: u32,
}

impl MicroVertex {
    #[inline]
    pub fn new(x: u8, y: u8, z: u8, normal: u8, color_idx: u8) -> Self {
        Self {
            packed: (x as u32) 
                | ((y as u32) << 8)
                | ((z as u32) << 16)
                | ((normal as u32 & 0x7) << 24)
                | ((color_idx as u32 & 0x1F) << 27),
        }
    }

    #[inline]
    pub fn x(&self) -> u8 { (self.packed & 0xFF) as u8 }
    
    #[inline]
    pub fn y(&self) -> u8 { ((self.packed >> 8) & 0xFF) as u8 }
    
    #[inline]
    pub fn z(&self) -> u8 { ((self.packed >> 16) & 0xFF) as u8 }
    
    #[inline]
    pub fn normal(&self) -> u8 { ((self.packed >> 24) & 0x7) as u8 }
    
    #[inline]
    pub fn color_idx(&self) -> u8 { ((self.packed >> 27) & 0x1F) as u8 }
}

/// Палитра цветов для MicroVertex (32 цвета)
pub struct ColorPalette {
    pub colors: [[f32; 3]; 32],
}

impl Default for ColorPalette {
    fn default() -> Self {
        let mut colors = [[0.0; 3]; 32];
        // Базовые цвета блоков
        colors[0] = [0.0, 0.0, 0.0];       // Air/Empty
        colors[1] = [0.5, 0.5, 0.5];       // Stone
        colors[2] = [0.55, 0.35, 0.2];     // Dirt
        colors[3] = [0.3, 0.6, 0.2];       // Grass top
        colors[4] = [0.4, 0.3, 0.2];       // Grass side
        colors[5] = [0.8, 0.8, 0.6];       // Sand
        colors[6] = [0.4, 0.25, 0.15];     // Wood
        colors[7] = [0.2, 0.5, 0.2];       // Leaves
        colors[8] = [0.3, 0.3, 0.35];      // Cobblestone
        colors[9] = [0.9, 0.9, 0.9];       // Snow
        colors[10] = [0.2, 0.4, 0.8];      // Water
        // ... остальные по необходимости
        Self { colors }
    }
}
