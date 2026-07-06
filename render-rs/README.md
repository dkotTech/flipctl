# render-rs

Headless HTML-рендер для flipctl: рендерит веб-страницы без дисплея и стримит
их как видео (PNG-кадры) по WebSocket. Управляется удалённо по HTTP API —
клавиатура, мышь, колесо.

Каждый рендер загружает HTML с указанного endpoint'а (например, порт
приложения flipctl-бекенда) — рендеров можно запустить несколько.

## Движки

Движок выбирается на этапе сборки cargo-фичами (взаимоисключающие):

```bash
# WPE WebKit (по умолчанию) — C FFI, бинарь ~5 MB
# зависимости: sudo pacman -S wpewebkit  (тянет libwpe, wpebackend-fdo)
cargo build --release

# Servo — чистый Rust, без системных зависимостей, бинарь ~80 MB
cargo build --release --no-default-features --features servo
```

|                       | webkit (WPE)     | servo              |
|-----------------------|------------------|--------------------|
| Headless              | ✅ (FDO SHM)     | ✅ (software ctx)  |
| CSS/JS совместимость  | высокая          | хорошая            |
| Зависимости           | пакет wpewebkit  | нет                |
| Бинарь                | ~5 MB            | ~80 MB             |

## Запуск

```bash
./target/release/render-rs \
  --port 8090 \
  --url http://localhost:5173/ \
  --url http://localhost:5174/ \
  --width 640 --height 480 --fps 30
```

Каждый `--url` поднимает отдельный рендер. Дальше рендеры можно
создавать/удалять на лету через API.

## API (порт 8090)

| Метод / путь                     | Что делает |
|----------------------------------|------------|
| `GET /`                          | менеджер рендеров (web UI) |
| `GET /view/{id}`                 | интерактивный viewer: canvas + проброс мыши/клавиатуры |
| `GET /api/renders`               | список рендеров |
| `POST /api/renders`              | создать: `{"url":"http://...","width":640,"height":480,"fps":30}` |
| `DELETE /api/renders/{id}`       | остановить рендер |
| `POST /api/renders/{id}/input`   | событие ввода (см. ниже) |
| `POST /api/renders/{id}/navigate`| перейти на другой url: `{"url":"..."}` |
| `GET /api/renders/{id}/ws`       | WebSocket: text-метаданные `{w,h,url}`, затем бинарные PNG-кадры; входящие text-сообщения — те же input-события |
| `GET /api/renders/{id}/frame`    | последний кадр как PNG (скриншот) |
| `GET /api/renders/{id}/stream`   | multipart-поток PNG (fallback без WebSocket) |

### Raw-подписка для дисплеев (MCU)

Тот же WS-эндпоинт с query-параметрами — сервер сам конвертирует кадры:

```
GET /api/renders/{id}/ws?format=rgb565&w=256&h=144&fps=10
```

- `format`: `png` (по умолчанию) | `rgb565` | `rgb888`
- `w`,`h`: серверный даунскейл (Nearest); без них — родной размер рендера
- `fps`: персональный темп подписчика (1–60), независим от других клиентов

Бинарное сообщение: `"FR" | fmt u8 | flags u8 | w u16 le | h u16 le | seq u32 le | pixels`.
Скармливать дисплею удобнее через [display-agent](../display-agent/README.md).

### События ввода

`key` использует значения JS `KeyboardEvent.key` ("a", "Enter", "ArrowUp", " ").

```jsonc
{"type":"key","key":"Enter"}                          // state: press (по умолч.) | down | up
{"type":"mouse_move","x":300,"y":180}
{"type":"mouse_button","button":"left","x":300,"y":180} // state: click (по умолч.) | down | up
{"type":"wheel","x":320,"y":240,"dy":40}
```

Пример — клик и набор текста:

```bash
curl -X POST localhost:8090/api/renders/1/input \
  -H 'Content-Type: application/json' \
  -d '{"type":"mouse_button","x":500,"y":36}'
curl -X POST localhost:8090/api/renders/1/input \
  -H 'Content-Type: application/json' -d '{"type":"key","key":"1"}'
```

## Как устроено

```
main thread                    per render                 tokio thread
┌─────────────────┐   spec    ┌──────────────────────┐   ┌─────────────────┐
│ engine loop      │◄──mpsc───│                      │   │ axum API :8090  │
│  webkit: GLib    │          │ pixels → sync(1) →   │   │  /api/renders   │
│  servo: spin     │──frames─►│ encoder thread (PNG) │──►│  /ws /frame ... │
└─────────────────┘           │  → broadcast(4)      │   └─────────────────┘
                              └──────────────────────┘
```

- Движок владеет главным потоком (GLib main loop / servo event loop);
  команды из API приходят по каналу.
- Кодирование PNG — в отдельном потоке на рендер; занятый кодировщик
  просто пропускает кадр (sync_channel(1)), рендер не блокируется.
- Последний кадр кешируется: новый WebSocket-клиент получает картинку
  сразу, не дожидаясь перерисовки страницы.

## Известные особенности

- WPE: выпадающий список нативного `<select>` не отрисовывается
  (попап рисует embedder). Значение можно менять с клавиатуры
  (фокус + стрелки) или через JS.
- Servo: частичная поддержка CSS grid — верстка может слегка отличаться;
  `requestAnimationFrame` без vsync не тикает, используйте `setInterval`.
- libEGL warnings при старте безвредны — рендер идёт в софте.
