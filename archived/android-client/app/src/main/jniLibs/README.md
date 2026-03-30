# Native Library Placement

`RustGameEngineBridge` loads `libsokobanitron_game_engine_jni.so` via `System.loadLibrary`.

Place ABI-specific builds in this directory structure:

- `arm64-v8a/libsokobanitron_game_engine_jni.so`
- `x86_64/libsokobanitron_game_engine_jni.so`

Example build (from repo root, with Android targets installed in Rust):

```bash
cargo build -p sokobanitron-game-engine --release --target aarch64-linux-android
cargo build -p sokobanitron-game-engine --release --target x86_64-linux-android
mkdir -p android-client/app/src/main/jniLibs/arm64-v8a android-client/app/src/main/jniLibs/x86_64
cp target/aarch64-linux-android/release/libsokobanitron_game_engine_jni.so android-client/app/src/main/jniLibs/arm64-v8a/
cp target/x86_64-linux-android/release/libsokobanitron_game_engine_jni.so android-client/app/src/main/jniLibs/x86_64/
```
