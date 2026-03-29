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

docker run --rm \
  -v "$REPO_ROOT":/src \
  -w /src \
  kindle-rust-builder \
  bash -lc 'export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABI_LINKER=/home/builder/x-tools/arm-unknown-linux-gnueabi/bin/arm-unknown-linux-gnueabi-gcc; cargo build -p kindle-client --target "$0"' "$TARGET"

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

scp "$REPO_ROOT/target/$TARGET/debug/$LOCAL_BIN" "$KINDLE_HOST:$KINDLE_PATH"

ssh "$KINDLE_HOST" <<EOF
/sbin/initctl stop lab126_gui || true
nohup "$KINDLE_PATH" >"$KINDLE_LOG" 2>&1 </dev/null &
EOF
