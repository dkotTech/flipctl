# Установка зависимостей — Linux

Команды по порядку. Нужны: Rust, Node.js, тулчейн для сборки servo-движка.

## 1. Системные пакеты

Arch / Manjaro:

```bash
sudo pacman -S --needed base-devel curl nodejs npm cmake clang pkgconf python fontconfig freetype2
```

Debian / Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y build-essential curl nodejs npm cmake clang pkg-config python3 libfontconfig1-dev libfreetype-dev
```

Fedora:

```bash
sudo dnf install -y @development-tools curl nodejs npm cmake clang pkgconf python3 fontconfig-devel freetype-devel
```

## 2. Rust

Если `cargo` ещё не установлен:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

## 3. Зависимости фронтендов

```bash
cd frontend && npm install && cd ..
cd frontend-ifconfig && npm install && cd ..
cd frontend-ping && npm install && cd ..
```

## 4. Запуск демо

```bash
./scripts/demo.sh              # debug-сборка (быстро)
./scripts/demo.sh --release    # release (первая сборка servo долгая — LTO)
```

Демо использует servo-движок. Откроется на http://localhost:8090/.

---

## Опционально: движок WPE WebKit

Для сборки render-rs с webkit-движком (`cargo build` без флагов в `render-rs/`)
дополнительно нужны библиотеки WPE:

Arch / Manjaro (libwpe и wpebackend-fdo приедут как зависимости):

```bash
sudo pacman -S --needed glib2 wayland wpewebkit
```

Debian / Ubuntu (имя dev-пакета wpewebkit зависит от версии дистрибутива —
подойдёт любой из `libwpewebkit-2.0-dev`, `libwpewebkit-1.1-dev`, `libwpewebkit-1.0-dev`):

```bash
sudo apt-get install -y libglib2.0-dev libwayland-dev libwpe-1.0-dev libwpebackend-fdo-1.0-dev libwpewebkit-2.0-dev
```

Fedora:

```bash
sudo dnf install -y glib2-devel wayland-devel libwpe-devel wpebackend-fdo-devel wpewebkit-devel
```
