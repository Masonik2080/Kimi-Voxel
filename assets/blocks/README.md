# Block Definitions (Data-Driven)

Блоки загружаются из JSON файлов в этой директории.

## Формат файла

```json
{
  "version": "1.0",
  "blocks": [...]
}
```

## Поля блока

| Поле | Тип | По умолчанию | Описание |
|------|-----|--------------|----------|
| `id` | string | required | Уникальный ID (namespace:name) |
| `numeric_id` | u8 | required | Числовой ID (0-255) |
| `name` | string | required | Отображаемое имя |
| `color` | color | [0.5, 0.5, 0.5] | Простой цвет (если нет текстур) |
| `textures` | textures | null | Пиксельные текстуры |
| `hardness` | f32 | 1.0 | Время ломания |
| `transparent` | bool | false | Прозрачность |
| `emissive` | bool | false | Излучает свет |
| `light_level` | u8 | 0 | Уровень света (0-15) |
| `solid` | bool | true | Твёрдый (коллизии) |
| `breakable` | bool | true | Можно сломать |
| `category` | string | "basic" | Категория |

---

## Текстуры

### Одна текстура для всех граней

```json
"textures": "path/to/texture.png"
```

или inline пиксели:

```json
"textures": {
  "width": 16,
  "height": 16,
  "pixels": ["#FF0000", "#00FF00", ...]
}
```

### Разные текстуры для граней

```json
"textures": {
  "top": "grass_top.png",
  "bottom": "dirt.png",
  "side": "grass_side.png"
}
```

Доступные грани: `top`, `bottom`, `north`, `south`, `east`, `west`, `side` (fallback для сторон)

---

## Форматы пикселей

### Hex строки
```json
"pixels": ["#FF0000", "#00FF00", "#0000FF", "#FFFFFF80"]
```
- `#RRGGBB` — RGB
- `#RRGGBBAA` — RGBA с альфой

### RGB/RGBA массивы
```json
"pixels": [[255, 0, 0], [0, 255, 0, 128]]
```

### Индексированная палитра (компактно!)
```json
{
  "palette": ["#8B4513", "#A0522D", "#654321"],
  "width": 8,
  "height": 8,
  "indices": [0, 0, 1, 2, 1, 0, 0, 1, ...]
}
```

---

## Процедурные текстуры

```json
"textures": {
  "type": "noise",
  "params": {
    "color1": "#606060",
    "color2": "#404040",
    "scale": 4.0,
    "seed": 12345
  }
}
```

Типы: `noise`, `gradient`, `checker`, `bricks`, `planks`, `ore`

---

## Примеры

### Граффити-блок (inline пиксели)
```json
{
  "id": "mymod:graffiti",
  "numeric_id": 101,
  "name": "Graffiti Wall",
  "textures": {
    "top": "concrete.png",
    "side": {
      "width": 16,
      "height": 16,
      "pixels": ["#808080", "#FF0000", "#00FF00", ...]
    }
  }
}
```

### Смайлик (8x8 pixel art)
```json
{
  "id": "mymod:smiley",
  "numeric_id": 102,
  "name": "Smiley",
  "textures": {
    "width": 8,
    "height": 8,
    "pixels": [
      "#FFFF00", "#FFFF00", "#FFFF00", "#FFFF00", "#FFFF00", "#FFFF00", "#FFFF00", "#FFFF00",
      "#FFFF00", "#FFFF00", "#000000", "#FFFF00", "#FFFF00", "#000000", "#FFFF00", "#FFFF00",
      ...
    ]
  }
}
```

### Кирпичи с палитрой
```json
{
  "id": "mymod:brick",
  "numeric_id": 103,
  "textures": {
    "palette": ["#8B4513", "#A0522D", "#654321", "#2F1810"],
    "width": 8,
    "height": 8,
    "indices": [0, 0, 0, 3, 1, 1, 1, 1, ...]
  }
}
```

---

## Numeric ID

- 0-99: Встроенные блоки
- 100-199: Кастомные блоки (моды)
- 200-255: Резерв

## Категории

`basic`, `stone`, `ore`, `wood`, `nature`, `building`, `metal`
