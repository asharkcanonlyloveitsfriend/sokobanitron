#!/usr/bin/env bash
set -euo pipefail

TARGET=armv7-unknown-linux-gnueabi
BIN=kindle-client
KINDLE_HOST=kindle
KINDLE_PATH=/mnt/us/$BIN
KINDLE_LOG=/mnt/us/$BIN.log
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

docker run --rm \
  -v "$REPO_ROOT":/src \
  -w /src \
  kindle-rust-builder \
  bash -lc 'export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABI_LINKER=/home/builder/x-tools/arm-unknown-linux-gnueabi/bin/arm-unknown-linux-gnueabi-gcc; cargo build -p kindle-client --target "$0"' "$TARGET"

ssh "$KINDLE_HOST" <<'EOF'
pkill kindle-client 2>/dev/null || true
for _ in $(seq 1 5); do
  pgrep kindle-client >/dev/null 2>&1 || break
  sleep 1
done
pkill -9 kindle-client 2>/dev/null || true
EOF

scp "$REPO_ROOT/target/$TARGET/debug/$BIN" "$KINDLE_HOST:$KINDLE_PATH"

ssh "$KINDLE_HOST" <<EOF
/sbin/initctl stop lab126_gui || true
nohup "$KINDLE_PATH" >"$KINDLE_LOG" 2>&1 </dev/null &
EOF
