# Sokobanitron

Sokobanitron is a pointer-first Sokoban game and level editor designed for E-ink displays. The shared Rust application renders a grayscale interface and runs behind thin desktop, Android, and Kindle clients.

![Sokobanitron gameplay](docs/screenshots/gameplay.png)

The project is under active development. The desktop client is the easiest way to run and test it; the Kindle client currently targets the 7th-generation Kindle Paperwhite hardware described in the device notes.

## Features

- Tap-to-move Sokoban gameplay with undo, restart, level selection, and saved progress
- Built-in level editor with draw, pan, and selection modes
- SLC level-set import with automatic board normalization and portrait orientation
- SQLite persistence for imported puzzles, progress, solutions, ratings, and favorites
- Shared grayscale renderer and input model across desktop, Android, and Kindle
- E-ink-aware partial refreshes and presentation timing on Kindle

## Quick start (desktop)

### Requirements

- Rust 1.95.0 (also declared in [`mise.toml`](mise.toml))
- A Sokoban level set in SLC format

If you use [mise](https://mise.jdx.dev/), install the pinned toolchain with:

```sh
mise install
```

Create the import directory and place one or more `.slc` files in it:

```sh
mkdir -p tmp/level_sets/to_import
cp /path/to/levels.slc tmp/level_sets/to_import/
```

Then run the desktop client from the repository root:

```sh
cargo run -p desktop-client
```

With mise, the equivalent command is:

```sh
mise exec -- cargo run -p desktop-client
```

On startup, Sokobanitron imports each valid SLC file, records its puzzles in `tmp/level_sets/sokobanitron.db`, and moves the source file to `tmp/level_sets/imported`. Invalid files remain in `to_import` and produce a warning. At least one successfully imported level set is required to launch.

## Controls

The interface is designed around taps or left clicks:

- Tap a reachable floor tile to move the player there.
- Tap a box to push it when the destination is open.
- Double-tap the box that was just moved to undo the move.
- Double-tap the player to restart the current level.
- Use the on-screen menu for level selection, level-set selection, and the editor.
- Swipe in the level and level-set selectors to change pages.
- In the editor, use the on-screen mode and tool controls; touch devices also support pinch-to-zoom.

Desktop keyboard shortcuts:

| Key | Action |
| --- | --- |
| `Backspace` | Undo |
| `Escape` | Restart the current level |

## Other clients

### Android

The Android app requires Android SDK 36, an NDK with an arm64 toolchain, and a connected Android 11+ arm64 device with USB debugging enabled. Configure `android-client/local.properties` with `sdk.dir`, put any level sets to bundle in `tmp/level_sets/to_import`, then run:

```sh
scripts/launch_android.sh
```

The script builds the release Rust JNI library, installs the debug application, transfers the bundled level sets into the app's private storage, and launches it. Set `ANDROID_SERIAL` when more than one device is connected. The current helper script supports macOS NDK host toolchains and physical `arm64-v8a` devices only.

To build just the JNI library and copy it into the Android project:

```sh
scripts/build_android_jni_device.sh release
```

### Kindle

The Kindle client writes directly to the framebuffer and EPDC refresh interface. Its deployment scripts assume:

- a local Docker image named `kindle-rust-builder`
- an SSH host alias named `kindle`
- a device compatible with the framebuffer, input-device, and display constants in `kindle-client/src/config.rs`

Copy SLC files to `/mnt/us/sokobanitron/level_sets/to_import` on the device, then build, deploy, and launch with:

```sh
scripts/kindle/run-kindle.sh
```

Stop the app and restore the Kindle UI with:

```sh
scripts/kindle/stop-kindle.sh
```

See [`kindle-client/README.md`](kindle-client/README.md) for framebuffer diagnostics, refresh metrics, touch calibration, and device investigation commands. Additional hardware findings are in [`docs/kindle-interface-notes.md`](docs/kindle-interface-notes.md).

## Project structure

| Path | Responsibility |
| --- | --- |
| `sokobanitron-core` | Grid validation and normalization, canonicalization, pathfinding, and solution validation |
| `sokobanitron-gameplay` | Board state, movement rules, history, and gameplay sessions |
| `sokobanitron-level-editor` | Editable world model, editor commands, snapshots, and validation |
| `sokobanitron-presentation` | Shared layout, hit testing, assets, grayscale rendering, and animation |
| `sokobanitron-app` | Application state, input policy, persistence, and shared runtime orchestration |
| `desktop-client` | `winit`/`pixels` desktop host |
| `android-client` | Kotlin Android host |
| `sokobanitron-android-jni` | Rust-to-Android JNI and native-window bridge |
| `kindle-client` | Linux framebuffer, touch, and power integration for Kindle |
| `archived` | Previous Android and engine implementations retained for reference |

Platform clients are deliberately thin: gameplay, editor behavior, persistence, layout, hit testing, and rendering live in the shared Rust crates.

## Development

Run the workspace tests:

```sh
cargo test --workspace
```

Run the formatting and lint checks used by the repository's pre-commit hook:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

Optional benchmarks live in `sokobanitron-core/benches` and can be run with `cargo bench -p sokobanitron-core`.

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the project's development principles and compatibility policy.
