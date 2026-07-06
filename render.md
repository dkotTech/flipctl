Что пробовали: рендер HTML → MJPEG стрим

  Задача

  Запустить HTML UI на сервере без дисплея и стримить его как видео по HTTP.

  ---
  1. Tauri + with_webview snapshot

  Идея: Tauri уже рендерит HTML через WebKit — взять оттуда пиксели.

  Проблемы:
  - webkit_web_view_get_snapshot() требует mapped (видимого) окна
  - С opacity=0 snapshot работал, но приложению нужен X11/Wayland
  - На сервере без дисплея — не запускается

  Итог: ❌ не headless

  ---
  2. wry / tao напрямую

  Идея: wry — WebView библиотека под Tauri, может работать отдельно.

  Проблемы:
  - window handle kind not supported на Wayland
  - Тоже требует видимый дисплей для рендера

  Итог: ❌ не headless

  ---
  3. Tauri headless mode

  Исследование: есть ли у Tauri headless режим?

  Вывод: Нет. Tauri — desktop framework, без дисплея не работает. --headless флага нет.

  Итог: ❌ не существует

  ---
  4. Xvfb (виртуальный дисплей)

  Идея: запустить Xvfb как виртуальный X11 дисплей, на нём запустить Tauri.

  Проблемы: лишний процесс, зависимость от X11, костыль. Решили искать лучше.

  Итог: ⚠️  работает, но костыль

  ---
  5. Wails v3 — исследование

  Идея: Go-альтернатива Tauri, может есть headless?

  Проверили:
  - server mode (-tags server) — браузер рендерит на клиенте, пиксели серверу недоступны
  - desktop mode — GTK4 + WebKit, требует дисплей

  Итог: ❌ server mode не рендерит, desktop mode нужен дисплей

  ---
  6. Wails v3 desktop + GTK4 snapshot ✅ (с ограничениями)

  Идея: Wails desktop, окно с opacity=0, GTK4 webkit_web_view_get_snapshot().

  Реализовано в /home/asd/rust_src/wails-test/:
  - CGo: ходим по дереву виджетов → находим WebKitWebView → get_snapshot() → GdkTexture → RGB пиксели → JPEG → MJPEG поток на :9090
  - GDK_BACKEND=x11 чтобы избежать Wayland protocol errors

  Итог: ✅ работает, но нужен X11 или Wayland compositor — не чисто headless

  ---
  7. WPE WebKit + FDO SHM backend ✅ (полный headless)

  Идея: WPE — WebKit для embedded без дисплея. FDO backend рендерит в shared memory через внутренний Wayland сокет.

  Реализовано в /home/asd/rust_src/wpe-screencaster/ (Rust):
  - Raw C FFI: wpe_loader_init → wpe_fdo_initialize_shm → WpeViewBackendExportableFdo → WebKitWebView
  - Frame callback: wl_shm_buffer_begin_access → BGRA → RGB → JPEG → broadcast::channel → Axum MJPEG
  - Зависимость: sudo pacman -S wpewebkit (36 MB пакет)

  Итог: ✅ полностью headless, без X11/Wayland/GPU

  ---
  8. Servo (чистый Rust) ✅ (полный headless)

  Идея: servo crate вышел на crates.io в апреле 2026 с embedding API. SoftwareRenderingContext — рендер в RAM без GPU и дисплея.

  Реализовано в /home/asd/rust_src/servo-screencaster/ (Rust):
  - Чистый Rust, без C FFI, без unsafe
  - ServoBuilder → SoftwareRenderingContext → WebViewBuilder → WebViewDelegate
  - В notify_new_frame_ready: webview.paint() → read_to_image() → JPEG → broadcast
  - Axum MJPEG сервер на отдельном tokio треде

  Размеры бинаря:
  - debug: 1.1 GB
  - release (дефолт): 149 MB
  - release + (strip + lto + opt-level=z + panic=abort): 77 MB

  Итог: ✅ полностью headless, чистый Rust, без внешних зависимостей

  ---
  Сравнение финальных решений

  ┌───────────────────────┬──────────────────┬────────────────────┐
  │                       │ WPE screencaster │ Servo screencaster │
  ├───────────────────────┼──────────────────┼────────────────────┤
  │ Headless              │ ✅               │ ✅                 │
  ├───────────────────────┼──────────────────┼────────────────────┤
  │ Язык                  │ Rust + C FFI     │ Чистый Rust        │
  ├───────────────────────┼──────────────────┼────────────────────┤
  │ Unsafe                │ Много            │ Нет                │
  ├───────────────────────┼──────────────────┼────────────────────┤
  │ Бинарь                │ ~5 MB            │ 77 MB              │
  ├───────────────────────┼──────────────────┼────────────────────┤
  │ Сборка                │ ~3 мин           │ ~10 мин (LTO)      │
  ├───────────────────────┼──────────────────┼────────────────────┤
  │ JS движок             │ JavaScriptCore   │ SpiderMonkey       │
  ├───────────────────────┼──────────────────┼────────────────────┤
  │ Системные зависимости │ wpewebkit пакет  │ нет                │
  ├───────────────────────┼──────────────────┼────────────────────┤
  │ CSS/JS совместимость  │ Высокая (WebKit) │ Хорошая (Servo)    │
  └───────────────────────┴──────────────────┴────────────────────┘

