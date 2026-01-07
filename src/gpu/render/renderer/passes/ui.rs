use crate::gpu::render::renderer::core::RenderComponents;

/// UI pass — рендеринг интерфейса (crosshair, FPS)
pub fn render<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    components: &'a RenderComponents,
) {
    let mut ui_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("UI Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    
    components.crosshair.render(&mut ui_pass);
    components.fps_counter.render(&mut ui_pass);
}
