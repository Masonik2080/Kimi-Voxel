// ============================================
// Inventory - Full-screen block inventory
// Hi-Tech glassmorphism style with scrollbar
// Press E to toggle
// ============================================

mod render;

pub use render::InventoryRenderer;

use crate::gpu::blocks::{
    BlockType, global_registry, BlockCategory as DataBlockCategory,
    get_face_colors, get_block_name, AIR,
    STONE, DIRT, GRASS, SAND, GRAVEL,
    COBBLESTONE, MOSSY_COBBLESTONE, GRANITE, DIORITE, ANDESITE, DEEPSLATE,
    COAL_ORE, IRON_ORE, GOLD_ORE, DIAMOND_ORE, EMERALD_ORE, REDSTONE_ORE, LAPIS_ORE, COPPER_ORE,
    OAK_LOG, OAK_PLANKS, OAK_LEAVES, BIRCH_LOG, BIRCH_PLANKS, BIRCH_LEAVES,
    SPRUCE_LOG, SPRUCE_PLANKS, SPRUCE_LEAVES,
    WATER, LAVA, ICE, SNOW, CLAY,
    BRICKS, STONE_BRICKS, OBSIDIAN, GLASS,
    IRON_BLOCK, GOLD_BLOCK, DIAMOND_BLOCK, EMERALD_BLOCK, COPPER_BLOCK,
    CUSTOM_100, CUSTOM_101, CUSTOM_102, CUSTOM_103, CUSTOM_104,
};

/// Количество колонок в инвентаре
pub const INVENTORY_COLS: usize = 8;

/// Размер одного слота в пикселях
pub const INV_SLOT_SIZE: f32 = 72.0;

/// Отступ между слотами
pub const INV_SLOT_GAP: f32 = 8.0;

/// Отступ от краёв панели
pub const INV_PADDING: f32 = 20.0;

/// Высота заголовка
pub const HEADER_HEIGHT: f32 = 50.0;

/// Ширина скроллбара
pub const SCROLLBAR_WIDTH: f32 = 12.0;

/// Категория блоков
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockCategory {
    All,
    Basic,
    Stone,
    Ores,
    Wood,
    Nature,
    Building,
    Metal,
}

impl BlockCategory {
    pub fn name(&self) -> &'static str {
        match self {
            BlockCategory::All => "All Blocks",
            BlockCategory::Basic => "Basic",
            BlockCategory::Stone => "Stone",
            BlockCategory::Ores => "Ores",
            BlockCategory::Wood => "Wood",
            BlockCategory::Nature => "Nature",
            BlockCategory::Building => "Building",
            BlockCategory::Metal => "Metal",
        }
    }
}

/// Предмет в инвентаре
#[derive(Clone, Debug)]
pub struct InventoryItem {
    pub block_type: BlockType,
    pub name: &'static str,
    pub top_color: [f32; 3],
    pub side_color: [f32; 3],
    pub category: BlockCategory,
}

impl InventoryItem {
    pub fn from_block(block_type: BlockType) -> Self {
        let name = get_block_name(block_type);
        let (top, side) = get_face_colors(block_type);
        let category = Self::categorize(block_type);
        
        Self {
            block_type,
            name: Box::leak(name.into_boxed_str()),
            top_color: top,
            side_color: side,
            category,
        }
    }
    
    fn categorize(block_type: BlockType) -> BlockCategory {
        // Сначала пробуем получить категорию из реестра
        if let Ok(registry) = global_registry().read() {
            if let Some(def) = registry.get_by_numeric(block_type) {
                return match def.category {
                    DataBlockCategory::Basic => BlockCategory::Basic,
                    DataBlockCategory::Stone => BlockCategory::Stone,
                    DataBlockCategory::Ore => BlockCategory::Ores,
                    DataBlockCategory::Wood => BlockCategory::Wood,
                    DataBlockCategory::Nature => BlockCategory::Nature,
                    DataBlockCategory::Building => BlockCategory::Building,
                    DataBlockCategory::Metal => BlockCategory::Metal,
                };
            }
        }
        
        // Fallback для встроенных блоков
        match block_type {
            STONE | DIRT | GRASS | SAND | GRAVEL => BlockCategory::Basic,
            
            COBBLESTONE | MOSSY_COBBLESTONE | GRANITE | DIORITE | ANDESITE | DEEPSLATE => BlockCategory::Stone,
            
            COAL_ORE | IRON_ORE | GOLD_ORE | DIAMOND_ORE | EMERALD_ORE | 
            REDSTONE_ORE | LAPIS_ORE | COPPER_ORE => BlockCategory::Ores,
            
            OAK_LOG | OAK_PLANKS | OAK_LEAVES | BIRCH_LOG | BIRCH_PLANKS | 
            BIRCH_LEAVES | SPRUCE_LOG | SPRUCE_PLANKS | SPRUCE_LEAVES => BlockCategory::Wood,
            
            WATER | LAVA | ICE | SNOW | CLAY => BlockCategory::Nature,
            
            BRICKS | STONE_BRICKS | OBSIDIAN | GLASS => BlockCategory::Building,
            
            IRON_BLOCK | GOLD_BLOCK | DIAMOND_BLOCK | EMERALD_BLOCK | COPPER_BLOCK => BlockCategory::Metal,
            
            _ => BlockCategory::Basic,
        }
    }
}

/// Состояние инвентаря
pub struct Inventory {
    /// Все доступные блоки
    items: Vec<InventoryItem>,
    /// Видимость инвентаря
    visible: bool,
    /// Текущая позиция скролла (0.0 - 1.0)
    scroll: f32,
    /// Максимальный скролл
    max_scroll: f32,
    /// Выбранный блок (для передачи в хотбар)
    selected_block: Option<BlockType>,
    /// Текущая категория
    category: BlockCategory,
    /// Индекс слота под курсором
    hovered_slot: Option<usize>,
    /// Перетаскиваемый блок (drag & drop)
    dragging_block: Option<BlockType>,
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

impl Inventory {
    pub fn new() -> Self {
        let items = Self::create_all_items();
        
        Self {
            items,
            visible: false,
            scroll: 0.0,
            max_scroll: 0.0,
            selected_block: None,
            category: BlockCategory::All,
            hovered_slot: None,
            dragging_block: None,
        }
    }
    
    fn create_all_items() -> Vec<InventoryItem> {
        let mut items = Vec::new();
        
        // Сначала добавляем блоки из глобального реестра (data-driven)
        if let Ok(registry) = global_registry().read() {
            for def in registry.all_blocks() {
                // Пропускаем Air
                if def.numeric_id == 0 {
                    continue;
                }
                
                // BlockType = u8, просто используем numeric_id
                let block_type: BlockType = def.numeric_id;
                let category = match def.category {
                    DataBlockCategory::Basic => BlockCategory::Basic,
                    DataBlockCategory::Stone => BlockCategory::Stone,
                    DataBlockCategory::Ore => BlockCategory::Ores,
                    DataBlockCategory::Wood => BlockCategory::Wood,
                    DataBlockCategory::Nature => BlockCategory::Nature,
                    DataBlockCategory::Building => BlockCategory::Building,
                    DataBlockCategory::Metal => BlockCategory::Metal,
                };
                
                items.push(InventoryItem {
                    block_type,
                    name: Box::leak(def.name.clone().into_boxed_str()),
                    top_color: def.color.top(),
                    side_color: def.color.side(),
                    category,
                });
            }
        }
        
        // Если реестр пуст, используем fallback со встроенными блоками
        if items.is_empty() {
            items = Self::create_builtin_items();
        }
        
        // Сортируем по numeric_id для консистентности
        items.sort_by_key(|i| i.block_type);
        
        items
    }
    
    /// Fallback: встроенные блоки (если реестр не загружен)
    fn create_builtin_items() -> Vec<InventoryItem> {
        let block_types: [BlockType; 47] = [
            // Basic
            STONE, DIRT, GRASS, SAND, GRAVEL,
            // Stone
            COBBLESTONE, MOSSY_COBBLESTONE, GRANITE, DIORITE, ANDESITE, DEEPSLATE,
            // Ores
            COAL_ORE, IRON_ORE, GOLD_ORE, DIAMOND_ORE, EMERALD_ORE, REDSTONE_ORE, LAPIS_ORE, COPPER_ORE,
            // Wood
            OAK_LOG, OAK_PLANKS, OAK_LEAVES, BIRCH_LOG, BIRCH_PLANKS, BIRCH_LEAVES,
            SPRUCE_LOG, SPRUCE_PLANKS, SPRUCE_LEAVES,
            // Nature
            WATER, LAVA, ICE, SNOW, CLAY,
            // Building
            BRICKS, STONE_BRICKS, OBSIDIAN, GLASS,
            // Metal blocks
            IRON_BLOCK, GOLD_BLOCK, DIAMOND_BLOCK, EMERALD_BLOCK, COPPER_BLOCK,
            // Custom blocks (from mods)
            CUSTOM_100, CUSTOM_101, CUSTOM_102, CUSTOM_103, CUSTOM_104,
        ];
        
        block_types.iter()
            .filter(|&&bt| bt != AIR)
            .map(|&bt| InventoryItem::from_block(bt))
            .collect()
    }
    
    /// Перезагрузить блоки из реестра (после загрузки модов)
    pub fn reload_from_registry(&mut self) {
        self.items = Self::create_all_items();
        self.scroll = 0.0;
    }
    
    /// Переключить видимость
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.selected_block = None;
            self.hovered_slot = None;
        }
    }
    
    /// Показать инвентарь
    pub fn show(&mut self) {
        self.visible = true;
        self.selected_block = None;
    }
    
    /// Скрыть инвентарь
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    /// Проверить видимость
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// Получить отфильтрованные предметы
    pub fn filtered_items(&self) -> Vec<&InventoryItem> {
        if self.category == BlockCategory::All {
            self.items.iter().collect()
        } else {
            self.items.iter().filter(|i| i.category == self.category).collect()
        }
    }
    
    /// Получить все предметы
    pub fn items(&self) -> &[InventoryItem] {
        &self.items
    }
    
    /// Получить текущий скролл
    pub fn scroll(&self) -> f32 {
        self.scroll
    }
    
    /// Установить скролл
    pub fn set_scroll(&mut self, scroll: f32) {
        self.scroll = scroll.clamp(0.0, self.max_scroll);
    }
    
    /// Обновить максимальный скролл
    pub fn update_max_scroll(&mut self, visible_rows: usize, total_rows: usize) {
        if total_rows > visible_rows {
            self.max_scroll = (total_rows - visible_rows) as f32;
        } else {
            self.max_scroll = 0.0;
        }
        self.scroll = self.scroll.clamp(0.0, self.max_scroll);
    }
    
    /// Прокрутка колёсиком (delta > 0 = скролл вверх, delta < 0 = скролл вниз)
    pub fn scroll_by(&mut self, delta: f32) {
        // Крутим вверх (delta > 0) -> scroll уменьшается (контент вниз)
        // Крутим вниз (delta < 0) -> scroll увеличивается (контент вверх)
        self.scroll = (self.scroll - delta).clamp(0.0, self.max_scroll);
    }
    
    /// Получить выбранный блок и сбросить
    pub fn take_selected_block(&mut self) -> Option<BlockType> {
        self.selected_block.take()
    }
    
    /// Установить выбранный блок
    pub fn select_block(&mut self, block_type: BlockType) {
        self.selected_block = Some(block_type);
    }
    
    /// Получить текущую категорию
    pub fn category(&self) -> BlockCategory {
        self.category
    }
    
    /// Установить категорию
    pub fn set_category(&mut self, category: BlockCategory) {
        self.category = category;
        self.scroll = 0.0;
    }
    
    /// Установить hovered слот
    pub fn set_hovered(&mut self, slot: Option<usize>) {
        self.hovered_slot = slot;
    }
    
    /// Получить hovered слот
    pub fn hovered(&self) -> Option<usize> {
        self.hovered_slot
    }
    
    /// Обработка клика (начало drag)
    pub fn handle_click(&mut self, slot_index: usize) -> Option<BlockType> {
        let items = self.filtered_items();
        if slot_index < items.len() {
            let block_type = items[slot_index].block_type;
            // Начинаем перетаскивание
            self.dragging_block = Some(block_type);
            return Some(block_type);
        }
        None
    }
    
    /// Начать перетаскивание блока
    pub fn start_drag(&mut self, block_type: BlockType) {
        self.dragging_block = Some(block_type);
    }
    
    /// Получить перетаскиваемый блок
    pub fn dragging(&self) -> Option<BlockType> {
        self.dragging_block
    }
    
    /// Завершить перетаскивание (drop)
    pub fn end_drag(&mut self) -> Option<BlockType> {
        self.dragging_block.take()
    }
    
    /// Отменить перетаскивание
    pub fn cancel_drag(&mut self) {
        self.dragging_block = None;
    }
    
    /// Получить максимальный скролл
    pub fn max_scroll(&self) -> f32 {
        self.max_scroll
    }
}
