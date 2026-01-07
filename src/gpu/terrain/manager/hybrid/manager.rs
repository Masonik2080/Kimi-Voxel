use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};

use crate::gpu::terrain::voxel::CHUNK_SIZE;
use crate::gpu::terrain::BlockPos;
use crate::gpu::blocks::BlockType;

use super::types::{GenerateRequest, GeneratedMesh};
use super::generator::HybridGenerator;

/// Асинхронный менеджер terrain с фоновой генерацией
pub struct HybridTerrainManager {
    request_tx: Sender<GenerateRequest>,
    result_rx: Receiver<GeneratedMesh>,
    _worker: JoinHandle<()>,
    current_chunk_x: i32,
    current_chunk_z: i32,
    pending: bool,
    changes_version: u64,
    last_sent_version: u64,
    lod_distances: [i32; 4],
    lod_changed: bool,
}

impl HybridTerrainManager {
    pub fn new() -> Self {
        let (request_tx, request_rx) = channel::<GenerateRequest>();
        let (result_tx, result_rx) = channel::<GeneratedMesh>();

        let worker = thread::spawn(move || {
            let mut generator = HybridGenerator::new();
            loop {
                match request_rx.recv() {
                    Ok(request) => {
                        if let Some(distances) = request.lod_distances {
                            generator.set_lod_distances(distances);
                        }
                        let mesh = generator.generate(
                            request.player_x,
                            request.player_z,
                            &request.world_changes,
                            request.changes_version,
                        );
                        if result_tx.send(mesh).is_err() { break; }
                    }
                    Err(_) => break,
                }
            }
        });
        
        Self {
            request_tx,
            result_rx,
            _worker: worker,
            current_chunk_x: i32::MIN,
            current_chunk_z: i32::MIN,
            pending: false,
            changes_version: 0,
            last_sent_version: 0,
            lod_distances: [8, 16, 32, 64],
            lod_changed: false,
        }
    }
    
    pub fn set_lod_distances(&mut self, distances: [i32; 4]) {
        if self.lod_distances != distances {
            self.lod_distances = distances;
            self.lod_changed = true;
        }
    }
    
    pub fn get_lod_distances(&self) -> [i32; 4] {
        self.lod_distances
    }
    
    pub fn generate_initial(&mut self, player_x: f32, player_z: f32) -> GeneratedMesh {
        let mut generator = HybridGenerator::new();
        let mesh = generator.generate(player_x, player_z, &HashMap::new(), 0);
        self.current_chunk_x = (player_x / CHUNK_SIZE as f32).floor() as i32;
        self.current_chunk_z = (player_z / CHUNK_SIZE as f32).floor() as i32;
        mesh
    }
    
    pub fn update(&mut self, player_x: f32, player_z: f32, world_changes: &HashMap<BlockPos, BlockType>, changes_version: u64) {
        let chunk_x = (player_x / CHUNK_SIZE as f32).floor() as i32;
        let chunk_z = (player_z / CHUNK_SIZE as f32).floor() as i32;
        self.changes_version = changes_version;
        
        let need_regen = chunk_x != self.current_chunk_x 
            || chunk_z != self.current_chunk_z
            || changes_version != self.last_sent_version
            || self.lod_changed;
        
        if need_regen && !self.pending {
            let lod_distances = if self.lod_changed {
                self.lod_changed = false;
                Some(self.lod_distances)
            } else {
                None
            };
            
            let request = GenerateRequest {
                player_x,
                player_z,
                world_changes: world_changes.clone(),
                changes_version,
                lod_distances,
            };
            
            if self.request_tx.send(request).is_ok() {
                self.pending = true;
                self.last_sent_version = changes_version;
                self.current_chunk_x = chunk_x;
                self.current_chunk_z = chunk_z;
            }
        }
    }
    
    pub fn try_get_mesh(&mut self) -> Option<GeneratedMesh> {
        match self.result_rx.try_recv() {
            Ok(mesh) => {
                self.pending = false;
                Some(mesh)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.pending = false;
                None
            }
        }
    }
}
