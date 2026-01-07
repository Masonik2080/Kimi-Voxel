// ============================================
// World File - Чтение/запись файла мира
// ============================================
// Оптимизированный формат с палитрой и чанками
// 
// Математика:
// - Наивный формат: 13 байт/блок (x,y,z,type) = 13GB на 1 млрд блоков
// - Чанковый формат с палитрой: ~0.5-2 байта/блок = 0.5-2GB на 1 млрд блоков
// - После ZSTD сжатия: ещё в 3-10 раз меньше
//
// Секрет: храним только изменённые секции 16x16x16, используем палитру

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write, BufReader, BufWriter};
use std::path::Path;

use serde::{Serialize, Deserialize};

use crate::gpu::blocks::BlockType;
use crate::gpu::terrain::{BlockPos, WorldChanges};
use crate::gpu::subvoxel::{SubVoxel, SubVoxelStorage};

use super::header::{SaveHeader, MAGIC_NUMBER, SAVE_VERSION};

const SECTION_SIZE: i32 = 16;
const SECTION_VOLUME: usize = 16 * 16 * 16; // 4096

/// Сжатая секция с палитрой
#[derive(Debug, Serialize, Deserialize)]
struct SavedSection {
    /// Координаты секции (chunk_x, section_y, chunk_z)
    cx: i32,
    sy: i32,
    cz: i32,
    /// Палитра: индекс -> (block_type, is_change_marker)
    /// is_change_marker=true означает что это реальное изменение
    palette: Vec<(u8, bool)>,
    /// Индексы в палитру (4096 значений, упакованы)
    /// Используем битовую упаковку в зависимости от размера палитры
    data: Vec<u8>,
    /// Бит на индекс (1, 2, 4, 8)
    bits_per_block: u8,
}

/// Тело файла (сжимается ZSTD)
#[derive(Debug, Serialize, Deserialize)]
struct SaveBody {
    sections: Vec<SavedSection>,
    /// Суб-воксели (ку-воксели)
    #[serde(default)]
    subvoxels: Vec<SubVoxel>,
}

/// Результат загрузки мира
#[derive(Debug)]
pub struct LoadedWorld {
    pub seed: u64,
    pub player_pos: [f32; 3],
    pub changes: HashMap<BlockPos, BlockType>,
    pub subvoxels: Vec<SubVoxel>,
}

/// Ошибки сохранения/загрузки
#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    Serialize(String),
    Deserialize(String),
    InvalidMagic,
    UnsupportedVersion(u32),
    Compression(String),
}

impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> Self {
        SaveError::Io(e)
    }
}

/// Основной интерфейс для работы с файлом мира
pub struct WorldFile;

impl WorldFile {
    /// Сохранить мир в файл
    pub fn save(
        path: impl AsRef<Path>,
        seed: u64,
        player_pos: [f32; 3],
        world_changes: &WorldChanges,
        subvoxel_storage: &SubVoxelStorage,
    ) -> Result<(), SaveError> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // 1. Записываем заголовок
        let header = SaveHeader::new(seed, player_pos);
        let header_bytes = bincode::serialize(&header)
            .map_err(|e| SaveError::Serialize(e.to_string()))?;
        writer.write_all(&header_bytes)?;

        // 2. Группируем изменения по секциям
        let sections = Self::build_sections(world_changes);
        
        // 3. Получаем суб-воксели
        let subvoxels = subvoxel_storage.get_all();

        // 4. Сериализуем и сжимаем
        let body = SaveBody { sections, subvoxels };
        let body_bytes = bincode::serialize(&body)
            .map_err(|e| SaveError::Serialize(e.to_string()))?;

        let compressed = zstd::encode_all(&body_bytes[..], 3)
            .map_err(|e| SaveError::Compression(e.to_string()))?;
        writer.write_all(&compressed)?;

        writer.flush()?;
        Ok(())
    }

    /// Загрузить мир из файла
    pub fn load(path: impl AsRef<Path>) -> Result<LoadedWorld, SaveError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // 1. Читаем заголовок
        let header_size = bincode::serialized_size(&SaveHeader::default()).unwrap_or(32) as usize;
        let mut header_bytes = vec![0u8; header_size];
        reader.read_exact(&mut header_bytes)?;

        let header: SaveHeader = bincode::deserialize(&header_bytes)
            .map_err(|e| SaveError::Deserialize(e.to_string()))?;

        if header.magic != MAGIC_NUMBER {
            return Err(SaveError::InvalidMagic);
        }
        if header.version != SAVE_VERSION {
            return Err(SaveError::UnsupportedVersion(header.version));
        }

        // 2. Читаем и распаковываем тело
        let mut compressed = Vec::new();
        reader.read_to_end(&mut compressed)?;

        let body_bytes = zstd::decode_all(&compressed[..])
            .map_err(|e| SaveError::Compression(e.to_string()))?;

        let body: SaveBody = bincode::deserialize(&body_bytes)
            .map_err(|e| SaveError::Deserialize(e.to_string()))?;

        // 3. Восстанавливаем изменения
        let changes = Self::extract_changes(&body.sections);

        Ok(LoadedWorld {
            seed: header.seed,
            player_pos: header.player_pos,
            changes,
            subvoxels: body.subvoxels,
        })
    }

    /// Группируем изменения по секциям 16x16x16
    fn build_sections(world_changes: &WorldChanges) -> Vec<SavedSection> {
        let all_changes = world_changes.get_all_changes_copy();
        if all_changes.is_empty() {
            return Vec::new();
        }

        // Группируем по секциям
        type SectionKey = (i32, i32, i32); // (chunk_x, section_y, chunk_z)
        let mut section_map: HashMap<SectionKey, Vec<(BlockPos, BlockType)>> = HashMap::new();

        for (pos, block) in all_changes {
            let cx = pos.x.div_euclid(SECTION_SIZE);
            let sy = pos.y.div_euclid(SECTION_SIZE);
            let cz = pos.z.div_euclid(SECTION_SIZE);
            
            section_map
                .entry((cx, sy, cz))
                .or_default()
                .push((pos, block));
        }

        // Конвертируем каждую секцию
        let mut sections = Vec::new();
        
        for ((cx, sy, cz), changes) in section_map {
            // Строим палитру: (block_type, is_real_change)
            // Индекс 0 = "нет изменения" (placeholder)
            let mut palette: Vec<(u8, bool)> = vec![(0, false)]; // placeholder
            let mut palette_map: HashMap<u8, usize> = HashMap::new();
            
            // Массив индексов (4096 элементов)
            let mut indices = vec![0u16; SECTION_VOLUME];
            
            for (pos, block) in changes {
                let lx = pos.x.rem_euclid(SECTION_SIZE) as usize;
                let ly = pos.y.rem_euclid(SECTION_SIZE) as usize;
                let lz = pos.z.rem_euclid(SECTION_SIZE) as usize;
                let idx = ly * 256 + lz * 16 + lx;
                
                let block_id = block as u8;
                
                // Получаем или создаём индекс в палитре
                let palette_idx = if let Some(&existing) = palette_map.get(&block_id) {
                    existing
                } else {
                    let new_idx = palette.len();
                    palette.push((block_id, true)); // true = реальное изменение
                    palette_map.insert(block_id, new_idx);
                    new_idx
                };
                
                indices[idx] = palette_idx as u16;
            }
            
            // Определяем bits_per_block
            let bits = if palette.len() <= 2 { 1 }
                else if palette.len() <= 4 { 2 }
                else if palette.len() <= 16 { 4 }
                else { 8 };
            
            // Упаковываем данные
            let data = Self::pack_indices(&indices, bits);
            
            sections.push(SavedSection {
                cx, sy, cz,
                palette,
                data,
                bits_per_block: bits,
            });
        }

        sections
    }

    /// Упаковка индексов в байты
    fn pack_indices(indices: &[u16], bits: u8) -> Vec<u8> {
        let values_per_byte = 8 / bits as usize;
        let total_bytes = (SECTION_VOLUME + values_per_byte - 1) / values_per_byte;
        let mut data = vec![0u8; total_bytes];
        
        for (i, &idx) in indices.iter().enumerate() {
            let byte_idx = i / values_per_byte;
            let bit_offset = (i % values_per_byte) * bits as usize;
            data[byte_idx] |= (idx as u8 & ((1 << bits) - 1)) << bit_offset;
        }
        
        data
    }

    /// Распаковка индексов из байтов
    fn unpack_indices(data: &[u8], bits: u8) -> Vec<u16> {
        let values_per_byte = 8 / bits as usize;
        let mask = (1u8 << bits) - 1;
        let mut indices = Vec::with_capacity(SECTION_VOLUME);
        
        for i in 0..SECTION_VOLUME {
            let byte_idx = i / values_per_byte;
            let bit_offset = (i % values_per_byte) * bits as usize;
            let value = (data.get(byte_idx).copied().unwrap_or(0) >> bit_offset) & mask;
            indices.push(value as u16);
        }
        
        indices
    }

    /// Извлекаем изменения из секций
    fn extract_changes(sections: &[SavedSection]) -> HashMap<BlockPos, BlockType> {
        let mut changes = HashMap::new();

        for section in sections {
            let base_x = section.cx * SECTION_SIZE;
            let base_y = section.sy * SECTION_SIZE;
            let base_z = section.cz * SECTION_SIZE;
            
            let indices = Self::unpack_indices(&section.data, section.bits_per_block);
            
            for (i, &palette_idx) in indices.iter().enumerate() {
                if palette_idx == 0 {
                    continue; // Нет изменения
                }
                
                if let Some(&(block_id, is_change)) = section.palette.get(palette_idx as usize) {
                    if is_change {
                        let lx = (i % 16) as i32;
                        let lz = ((i / 16) % 16) as i32;
                        let ly = (i / 256) as i32;
                        
                        let pos = BlockPos::new(base_x + lx, base_y + ly, base_z + lz);
                        let block = unsafe { std::mem::transmute::<u8, BlockType>(block_id) };
                        changes.insert(pos, block);
                    }
                }
            }
        }

        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_load_roundtrip() {
        let mut world_changes = WorldChanges::new();
        world_changes.set_block(BlockPos::new(10, 64, 10), BlockType::Stone);
        world_changes.set_block(BlockPos::new(11, 64, 10), BlockType::Dirt);
        world_changes.set_block(BlockPos::new(12, 64, 10), BlockType::Air); // Сломанный блок!
        
        let subvoxel_storage = SubVoxelStorage::new();

        let path = "test_world3.dat";
        
        WorldFile::save(path, 12345, [10.0, 65.0, 10.0], &world_changes, &subvoxel_storage).unwrap();
        let loaded = WorldFile::load(path).unwrap();

        assert_eq!(loaded.seed, 12345);
        assert_eq!(loaded.changes.len(), 3);
        assert_eq!(loaded.changes.get(&BlockPos::new(10, 64, 10)), Some(&BlockType::Stone));
        assert_eq!(loaded.changes.get(&BlockPos::new(12, 64, 10)), Some(&BlockType::Air));

        std::fs::remove_file(path).ok();
    }
}
