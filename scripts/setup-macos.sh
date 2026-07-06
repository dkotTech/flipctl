#!/usr/bin/env bash
# Ставит все зависимости flipctl на macOS:
#   - Rust (rustup), Node.js + npm
#   - тулчейн для сборки servo (cmake, python, pkg-config; clang из Xcode CLT)
#   - node_modules для всех фронтендов
#
# ВАЖНО: WPE WebKit на macOS недоступен — render-rs собирается только с servo:
#   cargo build --no-default-features --features servo
# (scripts/demo.sh и так использует servo, менять ничего не нужно.)
#
#   ./scripts/setup-macos.sh
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v brew >/dev/null; then
  echo "нужен Homebrew: https://brew.sh"
  echo '  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"'
  exit 1
fi

echo "==> Xcode command line tools (clang)"
if ! xcode-select -p >/dev/null 2>&1; then
  xcode-select --install
  echo "заверши установку CLT и перезапусти скрипт"
  exit 1
fi

echo "==> brew-пакеты"
brew install node cmake pkgconf python

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
echo "примечание: ifconfig/ping внутри приложений зовут линуксовые утилиты"
echo "(ip, ping из iputils) — на маке вывод/флаги отличаются, приложения"
echo "могут потребовать адаптации парсеров."
