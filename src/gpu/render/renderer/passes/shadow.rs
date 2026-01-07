use crate::gpu::terrain::GpuChunkManager;
use crate::gpu::render::pipelines::Pipelines;
use crate::gpu::render::shadow::ShadowResources;
use crate::gpu::subvoxel::SubVoxelRenderer;

use crate::gpu::render::renderer::culling::is_chunk_visible;

/// Shadow pass — рендеринг теней для всех каскадов
pub fn render(
    encoder: &mut wgpu::CommandEncoder,
    shadow: &ShadowResources,
    pipelines: &Pipelines,
    gpu_chunks: &GpuChunkManager,
    subvoxel_renderer: Option<&SubVoxelRenderer>,
) {
    for i in 0..shadow.config.num_cascades {
        let cascade_matrix = shadow.uniform.light_vp[i];
        
        let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&format!("Shadow Pass {}", i)),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &shadow.views[i],
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        shadow_pass.set_pipeline(&pipelines.shadow);
        shadow_pass.set_bind_group(0, &shadow.pass_bind_groups[i], &[]);

        // Рендерим terrain chunks
        for gpu_chunk in gpu_chunks.iter() {
            if is_chunk_visible(&cascade_matrix, gpu_chunk.key.x, gpu_chunk.key.z, gpu_chunk.key.scale) {
                shadow_pass.set_vertex_buffer(0, gpu_chunk.vertex_buffer.slice(..));
                shadow_pass.set_index_buffer(gpu_chunk.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..gpu_chunk.index_count, 0, 0..1);
            }
        }
        
        // Рендерим субвоксели в shadow map (по чанкам)
        if let Some(sv_renderer) = subvoxel_renderer {
            for (vertex_buffer, index_buffer, num_indices) in sv_renderer.iter_chunks() {
                shadow_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                shadow_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..num_indices, 0, 0..1);
            }
        }
    }
}
