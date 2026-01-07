use crate::gpu::render::uniforms::{Uniforms, LightUniform};
use crate::gpu::player::Camera;
use crate::gpu::player::Player;
use crate::gpu::terrain::WorldChanges;

use crate::gpu::render::renderer::core::{RenderComponents, LightingResources, TerrainResources, CachedCamera};

/// Обновление состояния рендерера каждый кадр
pub fn update(
    queue: &wgpu::Queue,
    camera: &Camera,
    player: &Player,
    time: f32,
    dt: f32,
    world_changes: &WorldChanges,
    components: &mut RenderComponents,
    lighting: &mut LightingResources,
    terrain: &mut TerrainResources,
    cached: &mut CachedCamera,
) {
    // День/ночь
    lighting.day_night.update(dt);

    // Uniforms
    let mut uniforms = Uniforms::new();
    uniforms.update(camera, time);
    uniforms.update_day_night(&lighting.day_night);
    cached.update(&uniforms, camera.view_matrix(), camera.projection_matrix(), camera.position);
    
    queue.write_buffer(
        &lighting.core_bind_groups.uniform_buffer,
        0,
        bytemuck::cast_slice(&[uniforms]),
    );

    // Light
    let primary = lighting.day_night.primary_light();
    let light = LightUniform {
        direction: primary.light_direction().into(),
        intensity: primary.intensity,
        color: (primary.color * lighting.day_night.ambient_intensity * 3.0).into(),
        _pad: 0.0,
    };
    queue.write_buffer(
        &lighting.core_bind_groups.light_buffer,
        0,
        bytemuck::cast_slice(&[light]),
    );

    // Shadows
    lighting.shadow.update(queue, camera.position, &lighting.day_night);

    // Celestial
    components.celestial.update(queue, cached.view_proj, camera.position, &lighting.day_night);

    // Player model
    components.player_model.update(queue, player);

    // Terrain
    terrain.terrain_manager.update(
        player.position.x,
        player.position.z,
        &world_changes.get_all_changes_copy(),
        world_changes.version(),
    );

    if let Some(mesh) = terrain.terrain_manager.try_get_mesh() {
        components.gpu_chunks.retain_only(&mesh.required_keys);
        for chunk_data in mesh.new_chunks {
            components.gpu_chunks.upload(chunk_data.key, &chunk_data.vertices, &chunk_data.indices);
        }
    }
}
