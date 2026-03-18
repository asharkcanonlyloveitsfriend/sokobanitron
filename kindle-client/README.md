# kindle-client

Kindle framebuffer client for Sokobanitron.

## Prerequisites

- Docker image `kindle-rust-builder` available locally.
- SSH host alias `kindle` configured.
- Kindle reachable over SSH/SCP.

## Run

From any directory:

```bash
/Users/jt/code/sokobanitron/kindle-client/run-kindle.sh
```

The script builds `kindle-client` for `armv7-unknown-linux-gnueabi`, copies it to
`/mnt/us/kindle-client` on the Kindle, stops `lab126_gui`, and starts the binary.

## Stop

```bash
/Users/jt/code/sokobanitron/kindle-client/stop-kindle.sh
```

This stops `kindle-client` and restores `lab126_gui`.

## Touch Calibration

Touch mapping constants are in:

- `src/config.rs` (`TOUCH_MIN_X`, `TOUCH_MAX_X`, `TOUCH_MIN_Y`, `TOUCH_MAX_Y`)

Adjust those values if tap locations are offset on-device.
