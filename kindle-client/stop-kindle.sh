#!/usr/bin/env bash
set -euo pipefail

BIN=kindle-client
KINDLE_HOST=kindle

ssh "$KINDLE_HOST" <<'EOF'
pkill kindle-client 2>/dev/null || true
while pgrep kindle-client >/dev/null 2>&1; do
  sleep 1
done
/sbin/initctl start lab126_gui || true
EOF
