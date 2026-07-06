#!/usr/bin/env bash
# Ставит все зависимости flipctl на Linux (Arch/Manjaro, Debian/Ubuntu, Fedora):
#   - Rust (rustup), Node.js + npm
#   - тулчейн для сборки servo-движка render-rs (cmake, clang, python)
#   - node_modules для всех фронтендов
#
#   ./scripts/setup-linux.sh            # servo (используется в demo.sh)
#   ./scripts/setup-linux.sh --webkit   # + системные библиотеки WPE WebKit
set -euo pipefail
cd "$(dirname "$0")/.."

WEBKIT=0
[[ "${1:-}" == "--webkit" ]] && WEBKIT=1

echo "==> системные пакеты"
if command -v pacman >/dev/null; then
  sudo pacman -S --needed --noconfirm \
    base-devel curl nodejs npm \
    cmake clang pkgconf python \
    fontconfig freetype2
  if [[ $WEBKIT == 1 ]]; then
    # wpewebkit тянет libwpe и wpebackend-fdo как зависимости
    sudo pacman -S --needed --noconfirm glib2 wayland wpewebkit
  fi
elif command -v apt-get >/dev/null; then
  sudo apt-get update
  sudo apt-get install -y \
    build-essential curl nodejs npm \
    cmake clang pkg-config python3 \
    libfontconfig1-dev libfreetype-dev
  if [[ $WEBKIT == 1 ]]; then
    sudo apt-get install -y \
      libglib2.0-dev libwayland-dev \
      libwpe-1.0-dev libwpebackend-fdo-1.0-dev
    # имя dev-пакета WPE WebKit зависит от версии дистрибутива
    sudo apt-get install -y libwpewebkit-2.0-dev \
      || sudo apt-get install -y libwpewebkit-1.1-dev \
      || sudo apt-get install -y libwpewebkit-1.0-dev
  fi
elif command -v dnf >/dev/null; then
  sudo dnf install -y \
    @development-tools curl nodejs npm \
    cmake clang pkgconf python3 \
    fontconfig-devel freetype-devel
  if [[ $WEBKIT == 1 ]]; then
    sudo dnf install -y \
      glib2-devel wayland-devel \
      libwpe-devel wpebackend-fdo-devel wpewebkit-devel
  fi
else
  echo "неизвестный пакетный менеджер — поставь вручную: nodejs npm cmake clang pkg-config python3"
  exit 1
fi

echo "==> rust"
if ! command -v cargo >/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi
rustc --version

echo "==> node_modules фронтендов"
for dir in frontend frontend-ifconfig frontend-ping; do
  echo "    $dir"
  (cd "$dir" && npm install --silent)
done

echo
echo "готово. дальше: ./scripts/demo.sh (использует servo-движок)"
if [[ $WEBKIT == 0 ]]; then
  echo "для webkit-движка render-rs перезапусти с флагом: ./scripts/setup-linux.sh --webkit"
fi
