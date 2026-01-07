use crate::gpu::terrain::voxel::CHUNK_SIZE;
use crate::gpu::terrain::mesh::TerrainVertex;
use crate::gpu::terrain::generation::{get_lod_height, get_color};

/// Генерация LOD чанка (thread-safe, для параллельной обработки)
pub fn generate_lod_chunk(cx: i32, cz: i32, scale: i32) -> (Vec<TerrainVertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(5000);
    let mut indices = Vec::with_capacity(7500);
    
    let base_x = cx * CHUNK_SIZE;
    let base_z = cz * CHUNK_SIZE;
    let size = CHUNK_SIZE + 2;
    let s = scale as f32;
    
    // Генерируем heightmap
    let mut heights = vec![0.0f32; (size * size) as usize];
    for z in 0..size {
        for x in 0..size {
            heights[(z * size + x) as usize] = get_lod_height(
                (base_x + (x - 1) * scale) as f32,
                (base_z + (z - 1) * scale) as f32,
                scale
            );
        }
    }
    
    generate_top_faces(&mut vertices, &mut indices, &heights, base_x, base_z, scale);
    generate_side_faces(&mut vertices, &mut indices, &heights, base_x, base_z, scale, s);
    generate_skirts(&mut vertices, &mut indices, &heights, base_x, base_z, scale, s);
    
    (vertices, indices)
}


/// Greedy meshing для верхних граней
fn generate_top_faces(
    vertices: &mut Vec<TerrainVertex>,
    indices: &mut Vec<u32>,
    heights: &[f32],
    base_x: i32,
    base_z: i32,
    scale: i32,
) {
    let size = CHUNK_SIZE + 2;
    let mut visited = vec![false; (CHUNK_SIZE * CHUNK_SIZE) as usize];
    
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let idx = (z * CHUNK_SIZE + x) as usize;
            if visited[idx] { continue; }
            
            let h = heights[((z + 1) * size + (x + 1)) as usize];
            let wx = (base_x + x * scale) as f32;
            let wz = (base_z + z * scale) as f32;
            let color = get_color(wx, wz, true);
            
            // Расширяем по X (только если тот же цвет и высота)
            let mut width = 1;
            while x + width < CHUNK_SIZE {
                let next_h = heights[((z + 1) * size + (x + width + 1)) as usize];
                let next_wx = (base_x + (x + width) * scale) as f32;
                let next_color = get_color(next_wx, wz, true);
                if visited[(z * CHUNK_SIZE + x + width) as usize] || next_h != h || next_color != color { break; }
                width += 1;
            }
            
            // Расширяем по Z
            let mut depth = 1;
            'outer: while z + depth < CHUNK_SIZE {
                for dx in 0..width {
                    let next_h = heights[((z + depth + 1) * size + (x + dx + 1)) as usize];
                    let next_wx = (base_x + (x + dx) * scale) as f32;
                    let next_wz = (base_z + (z + depth) * scale) as f32;
                    let next_color = get_color(next_wx, next_wz, true);
                    if visited[((z + depth) * CHUNK_SIZE + x + dx) as usize] || next_h != h || next_color != color { break 'outer; }
                }
                depth += 1;
            }
            
            // Помечаем как посещённые
            for dz in 0..depth {
                for dx in 0..width {
                    visited[((z + dz) * CHUNK_SIZE + x + dx) as usize] = true;
                }
            }
            
            // Создаём quad
            let w = (width * scale) as f32;
            let d = (depth * scale) as f32;
            
            let base_v = vertices.len() as u32;
            vertices.push(TerrainVertex { position: [wx, h, wz], normal: [0.0, 1.0, 0.0], color, block_id: 0 });
            vertices.push(TerrainVertex { position: [wx, h, wz + d], normal: [0.0, 1.0, 0.0], color, block_id: 0 });
            vertices.push(TerrainVertex { position: [wx + w, h, wz + d], normal: [0.0, 1.0, 0.0], color, block_id: 0 });
            vertices.push(TerrainVertex { position: [wx + w, h, wz], normal: [0.0, 1.0, 0.0], color, block_id: 0 });
            indices.extend_from_slice(&[base_v, base_v + 1, base_v + 2, base_v, base_v + 2, base_v + 3]);
        }
    }
}

/// Боковые грани между разными высотами
fn generate_side_faces(
    vertices: &mut Vec<TerrainVertex>,
    indices: &mut Vec<u32>,
    heights: &[f32],
    base_x: i32,
    base_z: i32,
    scale: i32,
    s: f32,
) {
    let size = CHUNK_SIZE + 2;
    
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let h = heights[((z + 1) * size + (x + 1)) as usize];
            let wx = (base_x + x * scale) as f32;
            let wz = (base_z + z * scale) as f32;
            
            let h_xp = heights[((z + 1) * size + (x + 2)) as usize];
            if h_xp < h { add_side_x(vertices, indices, wx + s, wz, h_xp, h, 1.0, s, wx, wz); }
            
            let h_xn = heights[((z + 1) * size + x) as usize];
            if h_xn < h { add_side_x(vertices, indices, wx, wz, h_xn, h, -1.0, s, wx, wz); }
            
            let h_zp = heights[((z + 2) * size + (x + 1)) as usize];
            if h_zp < h { add_side_z(vertices, indices, wx, wz + s, h_zp, h, 1.0, s, wx, wz); }
            
            let h_zn = heights[(z * size + (x + 1)) as usize];
            if h_zn < h { add_side_z(vertices, indices, wx, wz, h_zn, h, -1.0, s, wx, wz); }
        }
    }
}

/// Юбки по краям чанка
fn generate_skirts(
    vertices: &mut Vec<TerrainVertex>,
    indices: &mut Vec<u32>,
    heights: &[f32],
    base_x: i32,
    base_z: i32,
    scale: i32,
    s: f32,
) {
    let size = CHUNK_SIZE + 2;
    let skirt_depth = 8.0;
    
    // -Z edge
    for x in 0..CHUNK_SIZE {
        let h = heights[(1 * size + (x + 1)) as usize];
        let wx = (base_x + x * scale) as f32;
        let wz = base_z as f32;
        add_skirt_z(vertices, indices, wx, wz, h, h - skirt_depth, s, -1.0, wx, wz);
    }
    // +Z edge
    for x in 0..CHUNK_SIZE {
        let h = heights[(CHUNK_SIZE * size + (x + 1)) as usize];
        let wx = (base_x + x * scale) as f32;
        let wz = (base_z + CHUNK_SIZE * scale) as f32;
        add_skirt_z(vertices, indices, wx, wz, h, h - skirt_depth, s, 1.0, wx, wz);
    }
    // -X edge
    for z in 0..CHUNK_SIZE {
        let h = heights[((z + 1) * size + 1) as usize];
        let wx = base_x as f32;
        let wz = (base_z + z * scale) as f32;
        add_skirt_x(vertices, indices, wx, wz, h, h - skirt_depth, s, -1.0, wx, wz);
    }
    // +X edge
    for z in 0..CHUNK_SIZE {
        let h = heights[((z + 1) * size + CHUNK_SIZE) as usize];
        let wx = (base_x + CHUNK_SIZE * scale) as f32;
        let wz = (base_z + z * scale) as f32;
        add_skirt_x(vertices, indices, wx, wz, h, h - skirt_depth, s, 1.0, wx, wz);
    }
}

fn add_side_x(vertices: &mut Vec<TerrainVertex>, indices: &mut Vec<u32>, x: f32, z: f32, h_low: f32, h_high: f32, nx: f32, s: f32, world_x: f32, world_z: f32) {
    let color = get_color(world_x, world_z, false);
    let normal = [nx, 0.0, 0.0];
    let base = vertices.len() as u32;
    if nx < 0.0 {
        vertices.push(TerrainVertex { position: [x, h_low, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_low, z + s], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_high, z + s], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_high, z], normal, color, block_id: 0 });
    } else {
        vertices.push(TerrainVertex { position: [x, h_low, z + s], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_low, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_high, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_high, z + s], normal, color, block_id: 0 });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn add_side_z(vertices: &mut Vec<TerrainVertex>, indices: &mut Vec<u32>, x: f32, z: f32, h_low: f32, h_high: f32, nz: f32, s: f32, world_x: f32, world_z: f32) {
    let color = get_color(world_x, world_z, false);
    let normal = [0.0, 0.0, nz];
    let base = vertices.len() as u32;
    if nz > 0.0 {
        vertices.push(TerrainVertex { position: [x, h_low, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x + s, h_low, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x + s, h_high, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_high, z], normal, color, block_id: 0 });
    } else {
        vertices.push(TerrainVertex { position: [x + s, h_low, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_low, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_high, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x + s, h_high, z], normal, color, block_id: 0 });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn add_skirt_x(vertices: &mut Vec<TerrainVertex>, indices: &mut Vec<u32>, x: f32, z: f32, h_top: f32, h_bottom: f32, s: f32, nx: f32, world_x: f32, world_z: f32) {
    let color = get_color(world_x, world_z, false);
    let normal = [nx, 0.0, 0.0];
    let base = vertices.len() as u32;
    if nx < 0.0 {
        vertices.push(TerrainVertex { position: [x, h_bottom, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_bottom, z + s], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_top, z + s], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_top, z], normal, color, block_id: 0 });
    } else {
        vertices.push(TerrainVertex { position: [x, h_bottom, z + s], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_bottom, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_top, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_top, z + s], normal, color, block_id: 0 });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn add_skirt_z(vertices: &mut Vec<TerrainVertex>, indices: &mut Vec<u32>, x: f32, z: f32, h_top: f32, h_bottom: f32, s: f32, nz: f32, world_x: f32, world_z: f32) {
    let color = get_color(world_x, world_z, false);
    let normal = [0.0, 0.0, nz];
    let base = vertices.len() as u32;
    if nz > 0.0 {
        vertices.push(TerrainVertex { position: [x, h_bottom, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x + s, h_bottom, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x + s, h_top, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_top, z], normal, color, block_id: 0 });
    } else {
        vertices.push(TerrainVertex { position: [x + s, h_bottom, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_bottom, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x, h_top, z], normal, color, block_id: 0 });
        vertices.push(TerrainVertex { position: [x + s, h_top, z], normal, color, block_id: 0 });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}
