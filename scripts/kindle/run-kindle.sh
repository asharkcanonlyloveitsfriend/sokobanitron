#!/usr/bin/env bash
set -euo pipefail

TARGET=armv7-unknown-linux-gnueabi
LOCAL_BIN=kindle-client
DEVICE_BIN=sokobanitron
KINDLE_HOST=kindle
KINDLE_APP_ROOT=/mnt/us/sokobanitron
KINDLE_PATH=$KINDLE_APP_ROOT/$DEVICE_BIN
KINDLE_LOG=$KINDLE_APP_ROOT/$DEVICE_BIN.log
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILD_PROFILE="${1:-release}"
DIRTY_FB_WRITE="${SOKOBANITRON_KINDLE_DIRTY_FB_WRITE:-0}"
PRESENT_METRICS="${SOKOBANITRON_KINDLE_PRESENT_METRICS:-0}"

if [[ "$BUILD_PROFILE" != "release" && "$BUILD_PROFILE" != "debug" ]]; then
  echo "Usage: $0 [release|debug]"
  exit 1
fi

docker run --rm \
  -v "$REPO_ROOT":/src \
  -w /src \
  kindle-rust-builder \
  bash -lc 'export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABI_LINKER=/home/builder/x-tools/arm-unknown-linux-gnueabi/bin/arm-unknown-linux-gnueabi-gcc; if [[ "$1" == release ]]; then cargo build -p kindle-client --target "$0" --release; else cargo build -p kindle-client --target "$0"; fi' "$TARGET" "$BUILD_PROFILE"

ssh "$KINDLE_HOST" <<'EOF'
pkill sokobanitron 2>/dev/null || true
pkill kindle-client 2>/dev/null || true
for _ in $(seq 1 5); do
  if ! pgrep sokobanitron >/dev/null 2>&1 && ! pgrep kindle-client >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
pkill -9 sokobanitron 2>/dev/null || true
pkill -9 kindle-client 2>/dev/null || true
mkdir -p /mnt/us/sokobanitron
EOF

scp "$REPO_ROOT/target/$TARGET/$BUILD_PROFILE/$LOCAL_BIN" "$KINDLE_HOST:$KINDLE_PATH"

ssh "$KINDLE_HOST" <<EOF
/sbin/initctl stop lab126_gui || true
SOKOBANITRON_KINDLE_DIRTY_FB_WRITE="$DIRTY_FB_WRITE" SOKOBANITRON_KINDLE_PRESENT_METRICS="$PRESENT_METRICS" nohup "$KINDLE_PATH" >"$KINDLE_LOG" 2>&1 </dev/null &
EOF
