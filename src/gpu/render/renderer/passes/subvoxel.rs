use crate::gpu::render::pipelines::Pipelines;
use crate::gpu::render::bind_groups::{CoreBindGroups, AtlasResources};
use crate::gpu::render::shadow::ShadowResources;
use crate::gpu::subvoxel::SubVoxelRenderer;

/// SubVoxel pass — рендеринг ку-вокселей
/// Оптимизировано: рендерит каждый чанк отдельным draw call
pub fn render<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    depth_texture: &'a wgpu::TextureView,
    pipelines: &'a Pipelines,
    core_bind_groups: &'a CoreBindGroups,
    shadow: &'a ShadowResources,
    atlas: &'a AtlasResources,
    subvoxel_renderer: &'a SubVoxelRenderer,
) {
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("SubVoxel Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load, // Не очищаем, рисуем поверх
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_texture,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load, // Используем существующий depth
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    // Используем terrain pipeline для суб-вокселей (тот же формат вершин)
    render_pass.set_pipeline(&pipelines.terrain);
    render_pass.set_bind_group(0, &core_bind_groups.uniform_bind_group, &[]);
    render_pass.set_bind_group(1, &core_bind_groups.light_bind_group, &[]);
    render_pass.set_bind_group(2, &shadow.bind_group, &[]);
    render_pass.set_bind_group(3, &atlas.bind_group, &[]);
    
    // Рендерим каждый чанк отдельно
    for (vertex_buffer, index_buffer, num_indices) in subvoxel_renderer.iter_chunks() {
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..num_indices, 0, 0..1);
    }
}
