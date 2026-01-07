// ============================================
// Voxel Chunk - Воксельный чанк
// ============================================
// Data-Driven: все данные блоков из JSON

use std::collections::HashMap;
use crate::gpu::terrain::BlockPos;
use crate::gpu::blocks::{BlockType, AIR, WATER, DEEPSLATE, GRANITE, DIORITE, ANDESITE, 
    COAL_ORE, IRON_ORE, GOLD_ORE, DIAMOND_ORE, EMERALD_ORE, COPPER_ORE, SNOW, GRAVEL, GRASS, DIRT, get_face_colors};
use crate::gpu::terrain::generation::{get_height, CaveParams, is_cave, noise3d, is_solid_3d, hash3d};
use crate::gpu::terrain::mesh::TerrainVertex;
use crate::gpu::biomes::{biome_selector, BIOME_TAIGA, BIOME_TUNDRA, BIOME_FOREST};
use crate::gpu::biomes::features::{ChunkWriter, place_basic_tree, place_spruce_tree, TreeType, LeafSubVoxel};

use super::constants::{CHUNK_SIZE, WORLD_HEIGHT, MIN_HEIGHT};

/// Максимальная дополнительная высота для 3D структур над базовой высотой
const HEIGHT_3D_MARGIN: i32 = 30;
use super::greedy::{greedy_mesh_layer_into, add_greedy_face_with_block, FaceDir, FaceInfo};
use super::context::MeshingContext;

/// Генерирует блок процедурно с учётом биома и 3D-шума
fn generate_block(x: i32, y: i32, z: i32, _terrain_height: i32, cave_ceiling: i32, cave_params: &CaveParams) -> BlockType {
    // 1. Сначала проверяем, есть ли тут вообще земля по 3D-шуму
    // Это создаёт карнизы, арки и сложные формы скал
    if !is_solid_3d(x as f32, y as f32, z as f32) {
        // Если это ниже уровня моря (0), то вода, иначе воздух
        if y < 0 {
            return WATER;
        }
        return AIR;
    }
    
    // 2. Пещеры (вырезаем дырки в тверди)
    if y >= cave_params.min_height && y < cave_ceiling {
        if is_cave(x, y, z, cave_params) {
            return AIR;
        }
    }
    
    // Получаем биом для этой позиции
    let biome = biome_selector().get_biome_def(x, z);
    
    // 3. Определение типа блока
    // Проверяем, есть ли блок выше (для определения поверхности)
    let is_surface = !is_solid_3d(x as f32, (y + 1) as f32, z as f32);
    
    // Bedrock слой
    if y < MIN_HEIGHT + 3 {
        return DEEPSLATE;
    }
    
    // Логика поверхности
    if is_surface {
        // Высоко в горах - камень/снег
        if y > 110 {
            return SNOW;
        }
        if y > 60 {
            // Шум для гравия/камня на склонах
            let gravel_noise = noise3d(x as f32 * 0.1, y as f32 * 0.1, z as f32 * 0.1);
            if gravel_noise > 0.5 {
                return GRAVEL;
            }
            return biome.deep_block; // Камень для гор
        }
        return biome.surface_block; // Трава/Песок
    }
    
    // Чуть ниже поверхности (проверяем 4 блока вверх)
    if !is_solid_3d(x as f32, (y + 4) as f32, z as f32) {
        return biome.subsurface_block; // Земля
    }
    
    // Глубоко внутри - руды и камни
    if let Some(ore) = generate_ore(x, y, z) {
        return ore;
    }
    
    return generate_stone_variety(x, y, z, biome.deep_block);
}

/// Генерация разнообразия камней (granite, diorite, andesite)
fn generate_stone_variety(x: i32, y: i32, z: i32, base_stone: BlockType) -> BlockType {
    // Крупные "жилы" разных типов камня
    let variety_noise = noise3d(x as f32 * 0.03, y as f32 * 0.03, z as f32 * 0.03);
    
    // Второй слой шума для более интересных форм
    let detail_noise = noise3d(x as f32 * 0.08 + 100.0, y as f32 * 0.08, z as f32 * 0.08 + 100.0);
    
    let combined = variety_noise * 0.7 + detail_noise * 0.3;
    
    if combined > 0.65 {
        GRANITE
    } else if combined > 0.55 {
        DIORITE  
    } else if combined < 0.35 {
        ANDESITE
    } else {
        base_stone
    }
}

/// Генерация руд
fn generate_ore(x: i32, y: i32, z: i32) -> Option<BlockType> {
    // Разные руды на разных глубинах
    
    // Уголь: -20 до 40, частый
    if y >= -20 && y <= 40 {
        let coal_noise = noise3d(x as f32 * 0.12 + 50.0, y as f32 * 0.12, z as f32 * 0.12 + 50.0);
        if coal_noise > 0.75 {
            return Some(COAL_ORE);
        }
    }
    
    // Медь: -30 до 30
    if y >= -30 && y <= 30 {
        let copper_noise = noise3d(x as f32 * 0.1 + 150.0, y as f32 * 0.1, z as f32 * 0.1 + 150.0);
        if copper_noise > 0.78 {
            return Some(COPPER_ORE);
        }
    }
    
    // Железо: -30 до 20
    if y >= -30 && y <= 20 {
        let iron_noise = noise3d(x as f32 * 0.11 + 200.0, y as f32 * 0.11, z as f32 * 0.11 + 200.0);
        if iron_noise > 0.77 {
            return Some(IRON_ORE);
        }
    }
    
    // Золото: -30 до 0, редкое
    if y >= -30 && y <= 0 {
        let gold_noise = noise3d(x as f32 * 0.09 + 300.0, y as f32 * 0.09, z as f32 * 0.09 + 300.0);
        if gold_noise > 0.82 {
            return Some(GOLD_ORE);
        }
    }
    
    // Изумруд: только в горах, -30 до 30
    if y >= -30 && y <= 30 {
        let emerald_noise = noise3d(x as f32 * 0.08 + 400.0, y as f32 * 0.08, z as f32 * 0.08 + 400.0);
        if emerald_noise > 0.88 {
            return Some(EMERALD_ORE);
        }
    }
    
    // Алмазы: -30 до -10, очень редкие
    if y >= -30 && y <= -10 {
        let diamond_noise = noise3d(x as f32 * 0.07 + 500.0, y as f32 * 0.07, z as f32 * 0.07 + 500.0);
        if diamond_noise > 0.9 {
            return Some(DIAMOND_ORE);
        }
    }
    
    None
}

/// Получить цвета для блока
#[inline]
fn get_block_colors(block: BlockType, _y: f32) -> ([f32; 3], [f32; 3]) {
    get_face_colors(block)
}

/// Воксельный чанк
pub struct VoxelChunk {
    blocks: Vec<BlockType>,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub min_y: i32,
    pub max_y: i32,
}

/// Результат генерации чанка с субвокселями листвы
pub struct ChunkGenerationResult {
    pub chunk: VoxelChunk,
    pub leaf_subvoxels: Vec<LeafSubVoxel>,
}

impl VoxelChunk {
    /// Создать чанк и вернуть субвоксели листвы
    pub fn new_with_subvoxels(chunk_x: i32, chunk_z: i32, world_changes: &HashMap<BlockPos, BlockType>) -> ChunkGenerationResult {
        let total_height = (WORLD_HEIGHT - MIN_HEIGHT) as usize;
        let mut blocks = vec![AIR; CHUNK_SIZE as usize * CHUNK_SIZE as usize * total_height];
        
        let base_x = chunk_x * CHUNK_SIZE;
        let base_z = chunk_z * CHUNK_SIZE;
        let cave_params = CaveParams::default();
        
        let mut min_y = WORLD_HEIGHT;
        let mut max_y = MIN_HEIGHT;
        
        // --- Этап 1: Генерация ландшафта (Terrain Pass) ---
        let mut surface_heights = [[0i32; CHUNK_SIZE as usize]; CHUNK_SIZE as usize];
        
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let world_x = base_x + lx;
                let world_z = base_z + lz;
                
                let terrain_height = get_height(world_x as f32, world_z as f32) as i32;
                let cave_ceiling = terrain_height - cave_params.surface_offset;
                
                surface_heights[lz as usize][lx as usize] = terrain_height;
                
                let gen_max_y = (terrain_height + HEIGHT_3D_MARGIN).min(WORLD_HEIGHT);
                
                for y in MIN_HEIGHT..gen_max_y {
                    let pos = BlockPos::new(world_x, y, world_z);
                    
                    let block = if let Some(&changed) = world_changes.get(&pos) {
                        changed
                    } else {
                        generate_block(world_x, y, world_z, terrain_height, cave_ceiling, &cave_params)
                    };
                    
                    if block != AIR {
                        min_y = min_y.min(y);
                        max_y = max_y.max(y);
                    }
                    
                    let idx = Self::index(lx, y, lz);
                    blocks[idx] = block;
                }
            }
        }
        
        // --- Этап 2: Декорация (Tree Pass) ---
        let mut tree_positions: Vec<(i32, i32, i32, u8, i32)> = Vec::new();
        
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let world_x = base_x + lx;
                let world_z = base_z + lz;
                let terrain_height = surface_heights[lz as usize][lx as usize];
                
                let surface_idx = Self::index(lx, terrain_height, lz);
                let surface_block = blocks.get(surface_idx).copied().unwrap_or(AIR);
                if surface_block != GRASS && surface_block != DIRT {
                    continue;
                }
                
                let biome = biome_selector().get_biome_def(world_x, world_z);
                
                if biome.tree_density > 0.0001 {
                    let rng = hash3d(world_x, terrain_height, world_z);
                    
                    if rng < biome.tree_density {
                        let tree_height = 5 + ((rng * 1000.0) as i32 % 3);
                        let y = terrain_height + 1;
                        max_y = max_y.max(y + tree_height + 2);
                        tree_positions.push((lx, lz, y, biome.id, tree_height));
                    }
                }
            }
        }
        
        // Размещаем деревья и собираем субвоксели
        let leaf_subvoxels = {
            let mut writer = ChunkWriter::new(&mut blocks, Some(world_changes), base_x, base_z);
            
            for (lx, lz, y, biome_id, tree_height) in tree_positions {
                match biome_id {
                    BIOME_TAIGA | BIOME_TUNDRA => {
                        place_spruce_tree(&mut writer, lx, y, lz, tree_height + 1);
                    },
                    BIOME_FOREST => {
                        let world_x = base_x + lx;
                        let world_z = base_z + lz;
                        let rng = hash3d(world_x, y, world_z);
                        if ((rng * 100.0) as i32) % 5 == 0 {
                            place_basic_tree(&mut writer, lx, y, lz, TreeType::Birch, tree_height);
                        } else {
                            place_basic_tree(&mut writer, lx, y, lz, TreeType::Oak, tree_height);
                        }
                    },
                    _ => {
                        place_basic_tree(&mut writer, lx, y, lz, TreeType::Oak, tree_height);
                    }
                }
            }
            
            writer.take_leaf_subvoxels()
        };
        
        ChunkGenerationResult {
            chunk: Self { blocks, chunk_x, chunk_z, min_y, max_y },
            leaf_subvoxels,
        }
    }

    pub fn new(chunk_x: i32, chunk_z: i32, world_changes: &HashMap<BlockPos, BlockType>) -> Self {
        // Для обратной совместимости - игнорируем субвоксели
        Self::new_with_subvoxels(chunk_x, chunk_z, world_changes).chunk
    }
    
    #[inline]
    fn index(lx: i32, y: i32, lz: i32) -> usize {
        let ly = y - MIN_HEIGHT;
        (ly as usize) * (CHUNK_SIZE as usize * CHUNK_SIZE as usize) 
            + (lz as usize) * (CHUNK_SIZE as usize) 
            + (lx as usize)
    }
    
    #[inline]
    pub fn get_local(&self, lx: i32, y: i32, lz: i32) -> BlockType {
        if lx < 0 || lx >= CHUNK_SIZE || lz < 0 || lz >= CHUNK_SIZE || y < MIN_HEIGHT || y >= WORLD_HEIGHT {
            return AIR;
        }
        self.blocks[Self::index(lx, y, lz)]
    }

    
    /// Zero-allocation генерация меша с использованием контекста
    pub fn generate_mesh_with_context(
        &self, 
        neighbors: &ChunkNeighbors, 
        ctx: &mut MeshingContext
    ) -> (Vec<TerrainVertex>, Vec<u32>) {
        ctx.clear_output();
        
        let base_x = self.chunk_x * CHUNK_SIZE;
        let base_z = self.chunk_z * CHUNK_SIZE;
        let chunk_size = CHUNK_SIZE as usize;
        
        self.generate_y_faces(neighbors, ctx, base_x, base_z, chunk_size);
        self.generate_x_faces(neighbors, ctx, base_x, base_z, chunk_size);
        self.generate_z_faces(neighbors, ctx, base_x, base_z, chunk_size);
        
        ctx.take_results()
    }
    
    #[inline]
    fn generate_y_faces(&self, neighbors: &ChunkNeighbors, ctx: &mut MeshingContext, base_x: i32, base_z: i32, chunk_size: usize) {
        for y in self.min_y..=self.max_y + 1 {
            ctx.clear_y_masks();
            
            for lz in 0..CHUNK_SIZE {
                for lx in 0..CHUNK_SIZE {
                    let idx = (lz as usize) * chunk_size + (lx as usize);
                    
                    if y > self.min_y {
                        let block = self.get_local(lx, y - 1, lz);
                        if block != AIR && block != WATER && self.is_face_visible(lx, y, lz, neighbors) {
                            ctx.y_buffers.mask_pos[idx] = Some(FaceInfo::new(block, true));
                        }
                    }
                    
                    if y <= self.max_y {
                        let block = self.get_local(lx, y, lz);
                        if block != AIR && block != WATER && self.is_face_visible(lx, y - 1, lz, neighbors) {
                            ctx.y_buffers.mask_neg[idx] = Some(FaceInfo::new(block, false));
                        }
                    }
                }
            }
            
            greedy_mesh_layer_into(&ctx.y_buffers.mask_pos[..chunk_size * chunk_size], &mut ctx.y_buffers.visited[..chunk_size * chunk_size], chunk_size, chunk_size, &mut ctx.greedy_results);
            for &(u, v, w, h, face) in &ctx.greedy_results {
                let (top_color, _) = get_block_colors(face.block_type, y as f32);
                add_greedy_face_with_block(&mut ctx.vertices, &mut ctx.indices, (base_x + u as i32) as f32, (y - 1) as f32, (base_z + v as i32) as f32, w as f32, h as f32, [0.0, 1.0, 0.0], top_color, FaceDir::PosY, face.block_type);
            }
            
            ctx.y_buffers.clear_visited(chunk_size * chunk_size);
            greedy_mesh_layer_into(&ctx.y_buffers.mask_neg[..chunk_size * chunk_size], &mut ctx.y_buffers.visited[..chunk_size * chunk_size], chunk_size, chunk_size, &mut ctx.greedy_results);
            for &(u, v, w, h, face) in &ctx.greedy_results {
                let (_, side_color) = get_block_colors(face.block_type, y as f32);
                add_greedy_face_with_block(&mut ctx.vertices, &mut ctx.indices, (base_x + u as i32) as f32, y as f32, (base_z + v as i32) as f32, w as f32, h as f32, [0.0, -1.0, 0.0], side_color, FaceDir::NegY, face.block_type);
            }
        }
    }

    
    #[inline]
    fn generate_x_faces(&self, neighbors: &ChunkNeighbors, ctx: &mut MeshingContext, base_x: i32, base_z: i32, chunk_size: usize) {
        let height_range = (self.max_y - self.min_y + 1) as usize;
        
        for lx in 0..=CHUNK_SIZE {
            ctx.clear_x_masks(height_range);
            
            for y in self.min_y..=self.max_y {
                for lz in 0..CHUNK_SIZE {
                    let y_idx = (y - self.min_y) as usize;
                    let idx = y_idx * chunk_size + (lz as usize);
                    
                    if lx > 0 {
                        let block = self.get_local(lx - 1, y, lz);
                        if block != AIR && block != WATER && self.is_face_visible(lx, y, lz, neighbors) {
                            ctx.x_buffers.mask_pos[idx] = Some(FaceInfo::new(block, false));
                        }
                    }
                    
                    if lx < CHUNK_SIZE {
                        let block = self.get_local(lx, y, lz);
                        if block != AIR && block != WATER && self.is_face_visible(lx - 1, y, lz, neighbors) {
                            ctx.x_buffers.mask_neg[idx] = Some(FaceInfo::new(block, false));
                        }
                    }
                }
            }
            
            let mask_size = chunk_size * height_range;
            
            greedy_mesh_layer_into(&ctx.x_buffers.mask_pos[..mask_size], &mut ctx.x_buffers.visited[..mask_size], chunk_size, height_range, &mut ctx.greedy_results);
            for &(u, v, w, h, face) in &ctx.greedy_results {
                let (_, side_color) = get_block_colors(face.block_type, (self.min_y + v as i32) as f32);
                add_greedy_face_with_block(&mut ctx.vertices, &mut ctx.indices, (base_x + lx - 1) as f32, (self.min_y + v as i32) as f32, (base_z + u as i32) as f32, w as f32, h as f32, [1.0, 0.0, 0.0], side_color, FaceDir::PosX, face.block_type);
            }
            
            ctx.x_buffers.clear_visited(mask_size);
            greedy_mesh_layer_into(&ctx.x_buffers.mask_neg[..mask_size], &mut ctx.x_buffers.visited[..mask_size], chunk_size, height_range, &mut ctx.greedy_results);
            for &(u, v, w, h, face) in &ctx.greedy_results {
                let (_, side_color) = get_block_colors(face.block_type, (self.min_y + v as i32) as f32);
                add_greedy_face_with_block(&mut ctx.vertices, &mut ctx.indices, (base_x + lx) as f32, (self.min_y + v as i32) as f32, (base_z + u as i32) as f32, w as f32, h as f32, [-1.0, 0.0, 0.0], side_color, FaceDir::NegX, face.block_type);
            }
        }
    }
    
    #[inline]
    fn generate_z_faces(&self, neighbors: &ChunkNeighbors, ctx: &mut MeshingContext, base_x: i32, base_z: i32, chunk_size: usize) {
        let height_range = (self.max_y - self.min_y + 1) as usize;
        
        for lz in 0..=CHUNK_SIZE {
            ctx.clear_z_masks(height_range);
            
            for y in self.min_y..=self.max_y {
                for lx in 0..CHUNK_SIZE {
                    let y_idx = (y - self.min_y) as usize;
                    let idx = y_idx * chunk_size + (lx as usize);
                    
                    if lz > 0 {
                        let block = self.get_local(lx, y, lz - 1);
                        if block != AIR && block != WATER && self.is_face_visible(lx, y, lz, neighbors) {
                            ctx.z_buffers.mask_pos[idx] = Some(FaceInfo::new(block, false));
                        }
                    }
                    
                    if lz < CHUNK_SIZE {
                        let block = self.get_local(lx, y, lz);
                        if block != AIR && block != WATER && self.is_face_visible(lx, y, lz - 1, neighbors) {
                            ctx.z_buffers.mask_neg[idx] = Some(FaceInfo::new(block, false));
                        }
                    }
                }
            }
            
            let mask_size = chunk_size * height_range;
            
            greedy_mesh_layer_into(&ctx.z_buffers.mask_pos[..mask_size], &mut ctx.z_buffers.visited[..mask_size], chunk_size, height_range, &mut ctx.greedy_results);
            for &(u, v, w, h, face) in &ctx.greedy_results {
                let (_, side_color) = get_block_colors(face.block_type, (self.min_y + v as i32) as f32);
                add_greedy_face_with_block(&mut ctx.vertices, &mut ctx.indices, (base_x + u as i32) as f32, (self.min_y + v as i32) as f32, (base_z + lz - 1) as f32, w as f32, h as f32, [0.0, 0.0, 1.0], side_color, FaceDir::PosZ, face.block_type);
            }
            
            ctx.z_buffers.clear_visited(mask_size);
            greedy_mesh_layer_into(&ctx.z_buffers.mask_neg[..mask_size], &mut ctx.z_buffers.visited[..mask_size], chunk_size, height_range, &mut ctx.greedy_results);
            for &(u, v, w, h, face) in &ctx.greedy_results {
                let (_, side_color) = get_block_colors(face.block_type, (self.min_y + v as i32) as f32);
                add_greedy_face_with_block(&mut ctx.vertices, &mut ctx.indices, (base_x + u as i32) as f32, (self.min_y + v as i32) as f32, (base_z + lz) as f32, w as f32, h as f32, [0.0, 0.0, -1.0], side_color, FaceDir::NegZ, face.block_type);
            }
        }
    }

    
    #[allow(dead_code)]
    pub fn generate_mesh(&self, neighbors: &ChunkNeighbors) -> (Vec<TerrainVertex>, Vec<u32>) {
        let mut ctx = MeshingContext::new();
        self.generate_mesh_with_context(neighbors, &mut ctx)
    }
    
    pub fn generate_mesh_section_with_context(&self, neighbors: &ChunkNeighbors, section_min_y: i32, section_max_y: i32, ctx: &mut MeshingContext) -> (Vec<TerrainVertex>, Vec<u32>) {
        ctx.clear_output();
        let base_x = self.chunk_x * CHUNK_SIZE;
        let base_z = self.chunk_z * CHUNK_SIZE;
        let chunk_size = CHUNK_SIZE as usize;
        let actual_min = section_min_y.max(self.min_y);
        let actual_max = section_max_y.min(self.max_y);
        if actual_min > actual_max { return ctx.take_results(); }
        
        // Simplified section mesh generation
        for y in actual_min..=actual_max + 1 {
            ctx.clear_y_masks();
            for lz in 0..CHUNK_SIZE {
                for lx in 0..CHUNK_SIZE {
                    let idx = (lz as usize) * chunk_size + (lx as usize);
                    if y > actual_min && y - 1 <= actual_max {
                        let block = self.get_local(lx, y - 1, lz);
                        if block != AIR && block != WATER && self.is_face_visible(lx, y, lz, neighbors) {
                            ctx.y_buffers.mask_pos[idx] = Some(FaceInfo::new(block, true));
                        }
                    }
                    if y >= actual_min && y <= actual_max {
                        let block = self.get_local(lx, y, lz);
                        if block != AIR && block != WATER && self.is_face_visible(lx, y - 1, lz, neighbors) {
                            ctx.y_buffers.mask_neg[idx] = Some(FaceInfo::new(block, false));
                        }
                    }
                }
            }
            greedy_mesh_layer_into(&ctx.y_buffers.mask_pos[..chunk_size * chunk_size], &mut ctx.y_buffers.visited[..chunk_size * chunk_size], chunk_size, chunk_size, &mut ctx.greedy_results);
            for &(u, v, w, h, face) in &ctx.greedy_results {
                let (top_color, _) = get_block_colors(face.block_type, y as f32);
                add_greedy_face_with_block(&mut ctx.vertices, &mut ctx.indices, (base_x + u as i32) as f32, (y - 1) as f32, (base_z + v as i32) as f32, w as f32, h as f32, [0.0, 1.0, 0.0], top_color, FaceDir::PosY, face.block_type);
            }
            ctx.y_buffers.clear_visited(chunk_size * chunk_size);
            greedy_mesh_layer_into(&ctx.y_buffers.mask_neg[..chunk_size * chunk_size], &mut ctx.y_buffers.visited[..chunk_size * chunk_size], chunk_size, chunk_size, &mut ctx.greedy_results);
            for &(u, v, w, h, face) in &ctx.greedy_results {
                let (_, side_color) = get_block_colors(face.block_type, y as f32);
                add_greedy_face_with_block(&mut ctx.vertices, &mut ctx.indices, (base_x + u as i32) as f32, y as f32, (base_z + v as i32) as f32, w as f32, h as f32, [0.0, -1.0, 0.0], side_color, FaceDir::NegY, face.block_type);
            }
        }
        ctx.take_results()
    }
    
    pub fn generate_mesh_section(&self, neighbors: &ChunkNeighbors, min_y: i32, max_y: i32) -> (Vec<TerrainVertex>, Vec<u32>) {
        let mut ctx = MeshingContext::new();
        self.generate_mesh_section_with_context(neighbors, min_y, max_y, &mut ctx)
    }
    
    #[inline]
    fn is_face_visible(&self, lx: i32, y: i32, lz: i32, neighbors: &ChunkNeighbors) -> bool {
        if lx >= 0 && lx < CHUNK_SIZE && lz >= 0 && lz < CHUNK_SIZE {
            if y < MIN_HEIGHT || y >= WORLD_HEIGHT { return y >= WORLD_HEIGHT; }
            return self.get_local(lx, y, lz) == AIR;
        }
        if lx < 0 { if let Some(neg_x) = neighbors.neg_x { return neg_x.get_local(CHUNK_SIZE - 1, y, lz) == AIR; } }
        else if lx >= CHUNK_SIZE { if let Some(pos_x) = neighbors.pos_x { return pos_x.get_local(0, y, lz) == AIR; } }
        if lz < 0 { if let Some(neg_z) = neighbors.neg_z { return neg_z.get_local(lx, y, CHUNK_SIZE - 1) == AIR; } }
        else if lz >= CHUNK_SIZE { if let Some(pos_z) = neighbors.pos_z { return pos_z.get_local(lx, y, 0) == AIR; } }
        true
    }
}

pub struct ChunkNeighbors<'a> {
    pub pos_x: Option<&'a VoxelChunk>,
    pub neg_x: Option<&'a VoxelChunk>,
    pub pos_z: Option<&'a VoxelChunk>,
    pub neg_z: Option<&'a VoxelChunk>,
}
