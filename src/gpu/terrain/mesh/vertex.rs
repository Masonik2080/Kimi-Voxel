// ============================================
// Terrain Vertex - Структура вершины
// ============================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    pub block_id: u32,  // ID блока для текстурного атласа
}

impl TerrainVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TerrainVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 9]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
    
    /// Создать вершину (block_id = 0 по умолчанию)
    pub fn new(position: [f32; 3], normal: [f32; 3], color: [f32; 3]) -> Self {
        Self { position, normal, color, block_id: 0 }
    }
    
    /// Создать вершину с block_id
    pub fn with_block(position: [f32; 3], normal: [f32; 3], color: [f32; 3], block_id: u8) -> Self {
        Self { position, normal, color, block_id: block_id as u32 }
    }
}
