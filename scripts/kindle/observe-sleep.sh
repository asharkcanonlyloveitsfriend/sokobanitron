#!/usr/bin/env bash
set -euo pipefail

KINDLE_HOST=${KINDLE_HOST:-kindle}
REMOTE_DIR=${REMOTE_DIR:-/mnt/us/kindle-sleep-observer}
LOCAL_OBSERVATIONS_DIR=${LOCAL_OBSERVATIONS_DIR:-"${TMPDIR:-/tmp}/sokobanitron-kindle-observations"}

usage() {
  cat <<EOF
Usage:
  $0 start
  $0 status
  $0 fetch [local-dir]
  $0 stop

This starts read-only observers on the Kindle itself and writes logs under:
  $REMOTE_DIR

Logs collected:
  powerd-events.log
  blanket-events.log
  state-poll.log

Typical workflow:
  1. Put the Kindle in the exact game runtime you care about.
  2. Run: $0 start
  3. Leave the device idle until it auto-sleeps, or press the power button once.
  4. Wake the device.
  5. Run: $0 fetch
  6. Inspect the captured logs locally.
EOF
}

remote_script() {
  local mode=$1
  ssh "$KINDLE_HOST" "REMOTE_DIR='$REMOTE_DIR' sh -s -- '$mode'" <<'EOF'
set -eu

mkdir -p "$REMOTE_DIR"

stop_one() {
  name=$1
  pid_file="$REMOTE_DIR/$name.pid"
  if [ -f "$pid_file" ]; then
    pid=$(cat "$pid_file" 2>/dev/null || true)
    if [ -n "${pid:-}" ] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      sleep 1
      kill -9 "$pid" 2>/dev/null || true
    fi
    rm -f "$pid_file"
  fi
}

start_observers() {
  stop_one powerd-events
  stop_one blanket-events
  stop_one state-poll

  : >"$REMOTE_DIR/powerd-events.log"
  : >"$REMOTE_DIR/blanket-events.log"
  : >"$REMOTE_DIR/state-poll.log"

  nohup sh -c '
    exec lipc-wait-event -t -m com.lab126.powerd "*"
  ' >>"$REMOTE_DIR/powerd-events.log" 2>&1 &
  echo $! >"$REMOTE_DIR/powerd-events.pid"

  nohup sh -c '
    exec lipc-wait-event -t -m com.lab126.blanket "*"
  ' >>"$REMOTE_DIR/blanket-events.log" 2>&1 &
  echo $! >"$REMOTE_DIR/blanket-events.pid"

  nohup sh -c '
    while :; do
      echo "=== $(date "+%Y-%m-%d %H:%M:%S %z") ==="

      echo "[powerd.state]"
      lipc-get-prop com.lab126.powerd state 2>/dev/null || true

      echo "[powerd.status]"
      lipc-get-prop com.lab126.powerd status 2>/dev/null || true

      echo "[powerd.preventScreenSaver]"
      lipc-get-prop com.lab126.powerd preventScreenSaver 2>/dev/null || true

      echo "[powerd.disableScreenOff]"
      lipc-get-prop com.lab126.powerd disableScreenOff 2>/dev/null || true

      echo "[kaf.frameworkStarted]"
      lipc-get-prop com.lab126.kaf frameworkStarted 2>/dev/null || true

      echo "[sys.power.state]"
      cat /sys/power/state 2>/dev/null || true

      echo "[sys.power.wakeup_count]"
      cat /sys/power/wakeup_count 2>/dev/null || true

      echo "[rtc0.wakealarm]"
      cat /sys/class/rtc/rtc0/wakealarm 2>/dev/null || true

      echo
      sleep 1
    done
  ' >>"$REMOTE_DIR/state-poll.log" 2>&1 &
  echo $! >"$REMOTE_DIR/state-poll.pid"

  echo "started"
}

status_observers() {
  for name in powerd-events blanket-events state-poll; do
    pid_file="$REMOTE_DIR/$name.pid"
    log_file="$REMOTE_DIR/$name.log"
    echo "== $name =="
    if [ -f "$pid_file" ]; then
      pid=$(cat "$pid_file" 2>/dev/null || true)
      if [ -n "${pid:-}" ] && kill -0 "$pid" 2>/dev/null; then
        echo "running pid=$pid"
      else
        echo "not running (stale pid file: ${pid:-unknown})"
      fi
    else
      echo "not started"
    fi
    if [ -f "$log_file" ]; then
      tail -n 12 "$log_file" 2>/dev/null || true
    fi
    echo
  done
}

stop_observers() {
  stop_one powerd-events
  stop_one blanket-events
  stop_one state-poll
  echo "stopped"
}

case "$1" in
  start) start_observers ;;
  status) status_observers ;;
  stop) stop_observers ;;
  *) echo "unknown mode: $1" >&2; exit 2 ;;
esac
EOF
}

fetch_logs() {
  local dest
  if [[ $# -ge 1 ]]; then
    dest=$1
  else
    dest="$LOCAL_OBSERVATIONS_DIR/$(date +%Y%m%d-%H%M%S)"
  fi

  mkdir -p "$dest"
  scp \
    "$KINDLE_HOST:$REMOTE_DIR/powerd-events.log" \
    "$KINDLE_HOST:$REMOTE_DIR/blanket-events.log" \
    "$KINDLE_HOST:$REMOTE_DIR/state-poll.log" \
    "$dest/"

  printf 'Fetched logs to %s\n' "$dest"
}

main() {
  local cmd=${1:-}
  case "$cmd" in
    start)
      remote_script start
      ;;
    status)
      remote_script status
      ;;
    stop)
      remote_script stop
      ;;
    fetch)
      shift || true
      fetch_logs "$@"
      ;;
    ""|-h|--help|help)
      usage
      ;;
    *)
      echo "unknown command: $cmd" >&2
      usage >&2
      exit 2
      ;;
  esac
}

main "$@"
