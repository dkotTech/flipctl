# Установка зависимостей — macOS

Команды по порядку. На маке render-rs собирается **только с servo-движком**
(WPE WebKit на macOS недоступен) — `scripts/demo.sh` и так использует servo,
ничего менять не нужно.

## 1. Homebrew

Если ещё нет:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

## 2. Xcode command line tools (clang)

```bash
xcode-select --install
```

Если уже стоят — команда просто скажет об этом.

## 3. Пакеты для сборки servo

```bash
brew install node cmake pkgconf python
```

## 4. Rust

Если `cargo` ещё не установлен:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

## 5. Зависимости фронтендов

```bash
cd frontend && npm install && cd ..
cd frontend-ifconfig && npm install && cd ..
cd frontend-ping && npm install && cd ..
```

## 6. Запуск демо

```bash
./scripts/demo.sh              # debug-сборка (быстро)
./scripts/demo.sh --release    # release (первая сборка servo — 20–40 минут, LTO)
```

Откроется на http://localhost:8090/.

---

## Известные ограничения на macOS

- Приложения ifconfig/ping вызывают линуксовые утилиты (`ip -j` из iproute2,
  `ping` из iputils). На маке у этих команд другие флаги и формат вывода, так
  что сами приложения покажут ошибку выполнения — рендер, главный UI и вся
  инфраструктура при этом работают. Парсеры под BSD-утилиты — отдельная задача.
