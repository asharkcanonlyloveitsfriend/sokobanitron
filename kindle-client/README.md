# kindle-client

Kindle framebuffer client for Sokobanitron.

## Prerequisites

- Docker image `kindle-rust-builder` available locally.
- SSH host alias `kindle` configured.
- Kindle reachable over SSH/SCP.

## Run

From any directory:

```bash
/Users/jt/code/sokobanitron/scripts/kindle/run-kindle.sh
```

The script builds `kindle-client` for `armv7-unknown-linux-gnueabi`, copies it to
`/mnt/us/sokobanitron/sokobanitron` on the Kindle, stops `lab126_gui`, and starts the binary.

Level sets should be copied manually into:

- `/mnt/us/sokobanitron/level_sets/to_import`

## Stop

```bash
/Users/jt/code/sokobanitron/scripts/kindle/stop-kindle.sh
```

This stops `kindle-client` and restores `lab126_gui`.

## Touch Calibration

Touch mapping constants are in:

- `src/config.rs` (`TOUCH_MIN_X`, `TOUCH_MAX_X`, `TOUCH_MIN_Y`, `TOUCH_MAX_Y`)

Adjust those values if tap locations are offset on-device.

## Interface Notes

Kindle display and EPDC findings are documented in:

- [docs/kindle-interface-notes.md](/Users/jt/code/sokobanitron/docs/kindle-interface-notes.md)

## Investigation Helpers

Sleep-state observation helper:

```bash
/Users/jt/code/sokobanitron/scripts/kindle/observe-sleep.sh
```
