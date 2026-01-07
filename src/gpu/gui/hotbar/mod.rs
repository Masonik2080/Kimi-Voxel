// ============================================
// Hotbar - Hi-Tech style inventory bar
// Glassmorphism + Neon cyan accents
// ============================================

mod render;

pub use render::{HotbarRenderer, HotbarSlot};

use crate::gpu::blocks::{BlockType, get_face_colors, AIR, STONE, DIRT, GRASS, OAK_PLANKS, COBBLESTONE, WATER};

/// Количество слотов в хотбаре
pub const HOTBAR_SLOTS: usize = 9;

/// Размер одного слота в пикселях
pub const SLOT_SIZE: f32 = 64.0;

/// Отступ между слотами
pub const SLOT_GAP: f32 = 10.0;

/// Отступ от низа экрана
pub const BOTTOM_PADDING: f32 = 40.0;

/// Состояние хотбара
pub struct Hotbar {
    /// Слоты с предметами (None = пустой слот)
    slots: [Option<HotbarItem>; HOTBAR_SLOTS],
    /// Индекс выбранного слота (0-8)
    selected: usize,
    /// Видимость хотбара
    visible: bool,
}

/// Предмет в слоте хотбара
#[derive(Clone, Debug)]
pub struct HotbarItem {
    /// Тип блока
    pub block_type: BlockType,
    /// Количество (для стакающихся предметов)
    pub count: u32,
    /// Цвет верхней грани (RGB)
    pub top_color: [f32; 3],
    /// Цвет боковых граней (RGB)
    pub side_color: [f32; 3],
}

impl HotbarItem {
    /// Создать предмет из типа блока
    pub fn from_block(block_type: BlockType) -> Self {
        let (top, side) = get_face_colors(block_type);
        Self {
            block_type,
            count: 1,
            top_color: top,
            side_color: side,
        }
    }
}

impl Default for Hotbar {
    fn default() -> Self {
        Self::new()
    }
}

impl Hotbar {
    pub fn new() -> Self {
        // Создаём хотбар с несколькими стартовыми блоками
        let mut slots: [Option<HotbarItem>; HOTBAR_SLOTS] = Default::default();
        
        // Стартовые блоки
        slots[0] = Some(HotbarItem::from_block(STONE));
        slots[1] = Some(HotbarItem::from_block(DIRT));
        slots[2] = Some(HotbarItem::from_block(GRASS));
        slots[3] = Some(HotbarItem::from_block(OAK_PLANKS));
        slots[4] = Some(HotbarItem::from_block(COBBLESTONE));
        slots[5] = Some(HotbarItem::from_block(WATER));
        
        Self {
            slots,
            selected: 0,
            visible: true,
        }
    }
    
    /// Выбрать слот по индексу (0-8)
    pub fn select(&mut self, index: usize) {
        if index < HOTBAR_SLOTS {
            self.selected = index;
        }
    }
    
    /// Выбрать слот по клавише (1-9)
    pub fn select_by_key(&mut self, key: u32) {
        if key >= 1 && key <= 9 {
            self.selected = (key - 1) as usize;
        }
    }
    
    /// Получить индекс выбранного слота
    pub fn selected(&self) -> usize {
        self.selected
    }
    
    /// Получить предмет в выбранном слоте
    pub fn selected_item(&self) -> Option<&HotbarItem> {
        self.slots[self.selected].as_ref()
    }
    
    /// Получить тип блока в выбранном слоте (для установки)
    pub fn selected_block_type(&self) -> Option<BlockType> {
        self.slots[self.selected].as_ref().map(|item| item.block_type)
    }
    
    /// Получить предмет в слоте по индексу
    pub fn get_item(&self, index: usize) -> Option<&HotbarItem> {
        self.slots.get(index).and_then(|s| s.as_ref())
    }
    
    /// Установить предмет в слот
    pub fn set_item(&mut self, index: usize, item: Option<HotbarItem>) {
        if index < HOTBAR_SLOTS {
            self.slots[index] = item;
        }
    }
    
    /// Pick block - взять блок и добавить в хотбар
    /// Возвращает true если блок был добавлен
    pub fn pick_block(&mut self, block_type: BlockType) -> bool {
        // Не добавляем воздух
        if block_type == AIR {
            return false;
        }
        
        // Сначала ищем этот блок в хотбаре
        for (i, slot) in self.slots.iter().enumerate() {
            if let Some(item) = slot {
                if item.block_type == block_type {
                    // Блок уже есть - просто выбираем этот слот
                    self.selected = i;
                    return true;
                }
            }
        }
        
        // Блока нет - ищем пустой слот
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(HotbarItem::from_block(block_type));
                self.selected = i;
                return true;
            }
        }
        
        // Нет пустых слотов - заменяем текущий выбранный
        self.slots[self.selected] = Some(HotbarItem::from_block(block_type));
        true
    }
    
    /// Получить все слоты
    pub fn slots(&self) -> &[Option<HotbarItem>; HOTBAR_SLOTS] {
        &self.slots
    }
    
    /// Показать/скрыть хотбар
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
    
    /// Проверить видимость
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// Обработка клика мыши (возвращает true если клик был по хотбару)
    pub fn handle_click(&mut self, mx: f32, my: f32, screen_width: f32, screen_height: f32) -> bool {
        if !self.visible {
            return false;
        }
        
        let hotbar_width = HOTBAR_SLOTS as f32 * SLOT_SIZE + (HOTBAR_SLOTS - 1) as f32 * SLOT_GAP;
        let hotbar_x = (screen_width - hotbar_width) / 2.0;
        let hotbar_y = screen_height - BOTTOM_PADDING - SLOT_SIZE;
        
        // Проверяем попадание в область хотбара
        if my >= hotbar_y && my <= hotbar_y + SLOT_SIZE {
            for i in 0..HOTBAR_SLOTS {
                let slot_x = hotbar_x + i as f32 * (SLOT_SIZE + SLOT_GAP);
                if mx >= slot_x && mx <= slot_x + SLOT_SIZE {
                    self.selected = i;
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Прокрутка колёсиком мыши
    pub fn scroll(&mut self, delta: i32) {
        if delta > 0 {
            self.selected = (self.selected + 1) % HOTBAR_SLOTS;
        } else if delta < 0 {
            self.selected = (self.selected + HOTBAR_SLOTS - 1) % HOTBAR_SLOTS;
        }
    }
}
