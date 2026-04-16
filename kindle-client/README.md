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

To test dirty framebuffer writes for partial updates:

```bash
SOKOBANITRON_KINDLE_DIRTY_FB_WRITE=1 /Users/jt/code/sokobanitron/scripts/kindle/run-kindle.sh
```

Omit the environment variable, or set it to `0`, to use the baseline full-framebuffer write path.

To log present-path timings into `/mnt/us/sokobanitron/sokobanitron.log`:

```bash
SOKOBANITRON_KINDLE_PRESENT_METRICS=1 /Users/jt/code/sokobanitron/scripts/kindle/run-kindle.sh
```

The metrics log records the present mode, dirty region, aligned refresh/write region, framebuffer
write mode, grayscale diff/copy time, framebuffer write time, refresh request time, total present
time, and bytes written.

For a full comparison, run:

```bash
SOKOBANITRON_KINDLE_PRESENT_METRICS=1 /Users/jt/code/sokobanitron/scripts/kindle/run-kindle.sh
SOKOBANITRON_KINDLE_PRESENT_METRICS=1 SOKOBANITRON_KINDLE_DIRTY_FB_WRITE=1 /Users/jt/code/sokobanitron/scripts/kindle/run-kindle.sh
```

Gameplay updates derive dirty cells from the previous and current board scenes by default. The
Kindle client uses one union refresh region for pass one. On the PW3 / 7th gen test device,
multiple back-to-back disjoint partial submissions were supported, but they were slower than one
union submission in the on-device test, so the Kindle gameplay path keeps the single-union policy
as the conservative choice for now. Keep using `SOKOBANITRON_KINDLE_DIRTY_FB_WRITE=1` when testing
the declared-dirty-region path.

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
