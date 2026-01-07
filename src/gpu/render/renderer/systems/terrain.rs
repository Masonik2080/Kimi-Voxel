use crate::gpu::terrain::voxel::{VoxelChunk, ChunkNeighbors, CHUNK_SIZE, MIN_HEIGHT};
use crate::gpu::terrain::{GpuChunkManager, ChunkKey};
use crate::gpu::terrain::WorldChanges;

/// Мгновенное обновление чанка при изменении блока
pub fn instant_chunk_update(
    gpu_chunks: &mut GpuChunkManager,
    block_x: i32,
    block_y: i32,
    block_z: i32,
    world_changes: &WorldChanges,
) {
    let chunk_x = block_x.div_euclid(CHUNK_SIZE);
    let chunk_z = block_z.div_euclid(CHUNK_SIZE);
    let section_y = (block_y - MIN_HEIGHT).div_euclid(16);
    let section_min_y = MIN_HEIGHT + section_y * 16;
    let section_max_y = section_min_y + 15;

    let changes = world_changes.get_all_changes_copy();
    let chunk = VoxelChunk::new(chunk_x, chunk_z, &changes);
    let neighbors = ChunkNeighbors {
        pos_x: None,
        neg_x: None,
        pos_z: None,
        neg_z: None,
    };
    let (vertices, indices) = chunk.generate_mesh_section(&neighbors, section_min_y, section_max_y);

    if !vertices.is_empty() {
        let key = ChunkKey::new_section(chunk_x, chunk_z, section_y);
        gpu_chunks.upload(key, &vertices, &indices);
    }
}

/// Обновление подсветки блока
pub fn update_block_highlight(
    queue: &wgpu::Queue,
    block_highlight: &crate::gpu::gui::BlockHighlight,
    view_proj: [[f32; 4]; 4],
    block_pos: Option<[i32; 3]>,
) {
    if let Some(pos) = block_pos {
        block_highlight.update(queue, view_proj, pos);
    }
}
