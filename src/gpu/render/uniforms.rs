use bytemuck::{Pod, Zeroable};
use ultraviolet::Mat4;

use crate::gpu::player::Camera;
use crate::gpu::lighting::DayNightCycle;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub time: f32,
    pub sky_color: [f32; 3],
    pub time_of_day: f32,
    pub fog_color: [f32; 3],
    pub _pad: f32,
}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::identity().into(),
            camera_pos: [0.0, 0.0, 0.0],
            time: 0.0,
            sky_color: [0.5, 0.7, 1.0],
            time_of_day: 0.5,
            fog_color: [0.7, 0.8, 0.9],
            _pad: 0.0,
        }
    }

    pub fn update(&mut self, camera: &Camera, time: f32) {
        self.view_proj = camera.view_projection_matrix().into();
        self.camera_pos = camera.position.into();
        self.time = time;
    }

    pub fn update_day_night(&mut self, cycle: &DayNightCycle) {
        self.sky_color = cycle.sky_color.into();
        self.fog_color = cycle.fog_color.into();
        self.time_of_day = cycle.time.time;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LightUniform {
    pub direction: [f32; 3],
    pub intensity: f32,
    pub color: [f32; 3],
    pub _pad: f32,
}

impl Default for LightUniform {
    fn default() -> Self {
        Self {
            direction: [0.4, -0.8, 0.3],
            intensity: 1.0,
            color: [1.0, 0.98, 0.95],
            _pad: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ShadowUniform {
    pub light_vp: [[[f32; 4]; 4]; 4],
    pub cascade_splits: [f32; 4],
    pub num_cascades: u32,
    pub texel_size: f32,
    pub bias: f32,
    pub _pad: f32,
}

impl Default for ShadowUniform {
    fn default() -> Self {
        Self {
            light_vp: [[[0.0; 4]; 4]; 4],
            cascade_splits: [64.0, 256.0, 512.0, 1024.0],
            num_cascades: 2,
            texel_size: 0.002,
            bias: 0.003, // Увеличен для уменьшения shadow acne
            _pad: 0.0,
        }
    }
}
