#!/usr/bin/env bash
set -euo pipefail

BIN=kindle-client
KINDLE_HOST=kindle

ssh "$KINDLE_HOST" <<'EOF'
pkill kindle-client 2>/dev/null || true
for _ in $(seq 1 5); do
  pgrep kindle-client >/dev/null 2>&1 || break
  sleep 1
done
pkill -9 kindle-client 2>/dev/null || true
/sbin/initctl start lab126_gui || true
EOF
