use crate::gpu::render::pipelines::Pipelines;
use crate::gpu::render::bind_groups::{CoreBindGroups, AtlasResources};
use crate::gpu::render::shadow::ShadowResources;

use crate::gpu::render::renderer::core::{RenderComponents, LightingResources};
use crate::gpu::render::renderer::culling::is_chunk_visible;

/// Main 3D pass — основной рендеринг сцены
pub fn render<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    depth_texture: &'a wgpu::TextureView,
    sky_color: ultraviolet::Vec3,
    cached_view_proj: &[[f32; 4]; 4],
    pipelines: &'a Pipelines,
    core_bind_groups: &'a CoreBindGroups,
    shadow: &'a ShadowResources,
    atlas: &'a AtlasResources,
    components: &'a RenderComponents,
    render_player: bool,
    highlight_block: Option<[i32; 3]>,
) {
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Main Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: sky_color.x as f64,
                    g: sky_color.y as f64,
                    b: sky_color.z as f64,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_texture,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(0.0), // Reversed-Z: clear to 0 instead of 1
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    // Celestial (sun/moon) — на заднем плане
    components.celestial.render(&mut render_pass);

    // Terrain
    render_pass.set_pipeline(&pipelines.terrain);
    render_pass.set_bind_group(0, &core_bind_groups.uniform_bind_group, &[]);
    render_pass.set_bind_group(1, &core_bind_groups.light_bind_group, &[]);
    render_pass.set_bind_group(2, &shadow.bind_group, &[]);
    render_pass.set_bind_group(3, &atlas.bind_group, &[]);

    for gpu_chunk in components.gpu_chunks.iter() {
        if is_chunk_visible(cached_view_proj, gpu_chunk.key.x, gpu_chunk.key.z, gpu_chunk.key.scale) {
            render_pass.set_vertex_buffer(0, gpu_chunk.vertex_buffer.slice(..));
            render_pass.set_index_buffer(gpu_chunk.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..gpu_chunk.index_count, 0, 0..1);
        }
    }

    // Player
    if render_player {
        render_pass.set_pipeline(&pipelines.player);
        render_pass.set_bind_group(0, &core_bind_groups.uniform_bind_group, &[]);
        components.player_model.render(&mut render_pass);
    }

    // Block highlight
    if highlight_block.is_some() {
        components.block_highlight.render(&mut render_pass);
    }
}
