#!/usr/bin/env bash
# Собирает и запускает весь демо-стек flipctl:
#   frontend'ы (main/ifconfig/ping → apps/*.zip) → backend → render-rs (servo).
#
#   ./scripts/demo.sh              # debug-сборка (быстро)
#   ./scripts/demo.sh --release    # release-сборка
#   RENDER_PORT=9000 ./scripts/demo.sh
#
# Ctrl+C останавливает всё.
set -euo pipefail
cd "$(dirname "$0")/.."

RENDER_PORT="${RENDER_PORT:-8090}"
PROFILE=debug
CARGO_FLAGS=()
if [[ "${1:-}" == "--release" ]]; then
  PROFILE=release
  CARGO_FLAGS+=(--release)
fi

echo "==> frontend'ы → apps/*.zip"
for dir in frontend frontend-ifconfig frontend-ping; do
  echo "    $dir"
  (cd "$dir" && npm run --silent pack >/dev/null)
done

echo "==> backend (cargo $PROFILE)"
(cd backend && cargo build --quiet "${CARGO_FLAGS[@]}")

echo "==> render-rs, движок servo (cargo $PROFILE)"
(cd render-rs && cargo build --quiet --no-default-features --features servo "${CARGO_FLAGS[@]}")

cleanup() {
  echo
  echo "==> остановка"
  # shellcheck disable=SC2046
  kill $(jobs -p) 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

echo "==> запуск backend"
"backend/target/$PROFILE/flipctl-backend" apps &

for i in $(seq 1 40); do
  curl -sf -o /dev/null http://localhost:5173/ && break
  sleep 0.5
  [[ $i == 40 ]] && { echo "backend не поднялся"; exit 1; }
done

# Рендер на каждое приложение из бекенда (url'ы из /api/apps)
APP_URLS=$(curl -s http://localhost:5173/api/apps | grep -o '"url":"[^"]*"' | cut -d'"' -f4)

RENDER_ARGS=()
for u in $APP_URLS; do
  RENDER_ARGS+=(--url "$u")
done

echo "==> запуск render-rs (servo) на: $APP_URLS"
"render-rs/target/$PROFILE/render-rs" --port "$RENDER_PORT" "${RENDER_ARGS[@]}" &

sleep 2
echo
echo "════════════════════════════════════════════════"
echo "  демо готово:"
echo "  рендеры:   http://localhost:$RENDER_PORT/"
echo "  напрямую:  http://localhost:5173/ (main)"
i=1
for u in $APP_URLS; do
  echo "  viewer:    http://localhost:$RENDER_PORT/view/$i  ($u)"
  i=$((i + 1))
done
echo "  Ctrl+C — остановить всё"
echo "════════════════════════════════════════════════"

wait
