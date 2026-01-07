// ============================================
// SubVoxel Vertex - Вершина субвокселя
// ============================================

use bytemuck::{Pod, Zeroable};

/// Вершина субвокселя (совместима с TerrainVertex)
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SubVoxelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

impl SubVoxelVertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x3,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    #[inline]
    pub fn new(position: [f32; 3], normal: [f32; 3], color: [f32; 3]) -> Self {
        Self { position, normal, color }
    }
}
