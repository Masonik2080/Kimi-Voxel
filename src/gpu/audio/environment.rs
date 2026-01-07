// ============================================
// Environment Analyzer - Рейтрейсинг окружения
// ============================================

use ultraviolet::Vec3;
use super::components::{EnvironmentParams, EnvironmentType};

/// Рейтрейсер для анализа окружения
pub struct EnvironmentAnalyzer {
    cached_params: EnvironmentParams,
    time_since_update: f32,
    update_interval: f32,
}

impl EnvironmentAnalyzer {
    pub fn new() -> Self {
        Self {
            cached_params: EnvironmentParams::default(),
            time_since_update: 0.0,
            update_interval: 0.2, // 5 раз в секунду
        }
    }
    
    /// Анализировать окружение вокруг позиции
    pub fn analyze<F>(&mut self, pos: Vec3, dt: f32, is_solid: F) -> EnvironmentParams
    where
        F: Fn(i32, i32, i32) -> bool,
    {
        self.time_since_update += dt;
        
        if self.time_since_update < self.update_interval {
            return self.cached_params;
        }
        self.time_since_update = 0.0;
        
        let distances = self.cast_rays(pos, &is_solid);
        self.cached_params = self.analyze_distances(&distances);
        self.cached_params
    }
    
    /// Получить текущие параметры без пересчёта
    pub fn current_params(&self) -> EnvironmentParams {
        self.cached_params
    }
    
    /// Рейкастим в 26 направлениях
    fn cast_rays<F>(&self, pos: Vec3, is_solid: &F) -> Vec<(Vec3, f32)>
    where
        F: Fn(i32, i32, i32) -> bool,
    {
        let max_distance = 25.0;
        let step = 0.5;
        let mut distances = Vec::with_capacity(26);
        
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    
                    let dir = Vec3::new(dx as f32, dy as f32, dz as f32).normalized();
                    let dist = self.raycast_distance(pos, dir, max_distance, step, is_solid);
                    distances.push((dir, dist));
                }
            }
        }
        
        distances
    }
    
    /// Анализ результатов рейкастинга
    fn analyze_distances(&self, distances: &[(Vec3, f32)]) -> EnvironmentParams {
        let max_distance = 25.0;
        
        let horizontal_distances: Vec<f32> = distances.iter()
            .filter(|(dir, _)| dir.y.abs() < 0.5)
            .map(|(_, d)| *d)
            .collect();
        
        let up_distance = distances.iter()
            .filter(|(dir, _)| dir.y > 0.5)
            .map(|(_, d)| *d)
            .next()
            .unwrap_or(max_distance);
        
        let down_distance = distances.iter()
            .filter(|(dir, _)| dir.y < -0.5)
            .map(|(_, d)| *d)
            .next()
            .unwrap_or(max_distance);
        
        let avg_horizontal = if horizontal_distances.is_empty() {
            max_distance
        } else {
            horizontal_distances.iter().sum::<f32>() / horizontal_distances.len() as f32
        };
        
        let min_horizontal = horizontal_distances.iter()
            .cloned()
            .fold(max_distance, f32::min);
        
        let total_avg: f32 = distances.iter().map(|(_, d)| *d).sum::<f32>() / distances.len() as f32;
        let enclosure = 1.0 - (total_avg / max_distance).clamp(0.0, 1.0);
        
        let depth = if up_distance < 5.0 && down_distance > 2.0 {
            (5.0 - up_distance) * 2.0 + down_distance.min(20.0)
        } else {
            0.0
        };
        
        let env_type = self.determine_environment_type(enclosure, min_horizontal, up_distance, depth);
        
        EnvironmentParams {
            env_type,
            avg_wall_distance: avg_horizontal,
            enclosure,
            ceiling_height: up_distance,
            depth_underground: depth,
        }
    }
    
    /// Определить тип окружения
    fn determine_environment_type(
        &self,
        enclosure: f32,
        min_horizontal: f32,
        up_distance: f32,
        depth: f32,
    ) -> EnvironmentType {
        if enclosure > 0.7 && min_horizontal < 2.0 {
            EnvironmentType::TightSpace
        } else if enclosure > 0.5 && up_distance < 8.0 {
            if depth > 10.0 {
                EnvironmentType::DeepUnderground
            } else {
                EnvironmentType::Cave
            }
        } else if enclosure > 0.3 {
            EnvironmentType::Forest
        } else {
            EnvironmentType::OpenField
        }
    }
    
    /// Рейкаст в направлении
    fn raycast_distance<F>(&self, origin: Vec3, dir: Vec3, max_dist: f32, step: f32, is_solid: &F) -> f32
    where
        F: Fn(i32, i32, i32) -> bool,
    {
        let mut dist = step;
        while dist < max_dist {
            let check_pos = origin + dir * dist;
            let bx = check_pos.x.floor() as i32;
            let by = check_pos.y.floor() as i32;
            let bz = check_pos.z.floor() as i32;
            
            if is_solid(bx, by, bz) {
                return dist;
            }
            dist += step;
        }
        max_dist
    }
}

impl Default for EnvironmentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
