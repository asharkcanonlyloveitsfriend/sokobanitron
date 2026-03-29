#!/usr/bin/env bash
set -euo pipefail

KINDLE_HOST=kindle

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
/sbin/initctl start lab126_gui || true
EOF
