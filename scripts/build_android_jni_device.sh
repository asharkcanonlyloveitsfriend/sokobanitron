#!/usr/bin/env bash
set -euo pipefail

# Builds and copies the Rust JNI library for physical Android devices only (arm64-v8a).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ANDROID_CLIENT_DIR="$REPO_ROOT/android-client"
LOCAL_PROPERTIES="$ANDROID_CLIENT_DIR/local.properties"
TARGET_TRIPLE="aarch64-linux-android"
ANDROID_ABI_DIR="arm64-v8a"
MIN_SDK=30
BUILD_PROFILE="${1:-release}"

if [[ "$BUILD_PROFILE" != "release" && "$BUILD_PROFILE" != "debug" ]]; then
  echo "Usage: $0 [release|debug]"
  exit 1
fi

if [[ ! -f "$LOCAL_PROPERTIES" ]]; then
  echo "Missing $LOCAL_PROPERTIES"
  exit 1
fi

sdk_dir_from_local_properties() {
  local value
  value="$(grep '^sdk.dir=' "$LOCAL_PROPERTIES" | head -n1 | cut -d'=' -f2- || true)"
  value="${value//\\:/:}"
  value="${value//\\\\/\\}"
  printf '%s' "$value"
}

SDK_DIR="${ANDROID_SDK_ROOT:-$(sdk_dir_from_local_properties)}"
if [[ -z "$SDK_DIR" || ! -d "$SDK_DIR" ]]; then
  echo "Android SDK directory not found. Set ANDROID_SDK_ROOT or sdk.dir in $LOCAL_PROPERTIES"
  exit 1
fi

if [[ -n "${ANDROID_NDK_HOME:-}" ]]; then
  NDK_HOME="$ANDROID_NDK_HOME"
else
  NDK_HOME="$(ls -d "$SDK_DIR"/ndk/* 2>/dev/null | sort -V | tail -n1 || true)"
fi

if [[ -z "$NDK_HOME" || ! -d "$NDK_HOME" ]]; then
  echo "Android NDK not found under $SDK_DIR/ndk."
  echo "Install it in Android Studio: SDK Manager > SDK Tools > NDK (Side by side)."
  exit 1
fi

NDK_TOOLCHAIN="$(ls -d "$NDK_HOME"/toolchains/llvm/prebuilt/darwin-* 2>/dev/null | head -n1 || true)"
if [[ -z "$NDK_TOOLCHAIN" || ! -d "$NDK_TOOLCHAIN" ]]; then
  echo "NDK clang toolchain not found in $NDK_HOME/toolchains/llvm/prebuilt."
  exit 1
fi

if ! rustup target list --installed | grep -q "^${TARGET_TRIPLE}\$"; then
  echo "Installing Rust target $TARGET_TRIPLE..."
  rustup target add "$TARGET_TRIPLE"
fi

export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_TOOLCHAIN/bin/aarch64-linux-android${MIN_SDK}-clang"
export CC_aarch64_linux_android="$CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"
export AR_aarch64_linux_android="$NDK_TOOLCHAIN/bin/llvm-ar"
export CXX_aarch64_linux_android="$NDK_TOOLCHAIN/bin/aarch64-linux-android${MIN_SDK}-clang++"

if [[ ! -x "$CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER" ]]; then
  echo "Expected linker not found: $CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"
  exit 1
fi

echo "Building JNI library for $TARGET_TRIPLE ($BUILD_PROFILE)..."
if [[ "$BUILD_PROFILE" == "release" ]]; then
  cargo build -p sokobanitron-android-jni --target "$TARGET_TRIPLE" --release
else
  cargo build -p sokobanitron-android-jni --target "$TARGET_TRIPLE"
fi

SOURCE_SO="$REPO_ROOT/target/$TARGET_TRIPLE/$BUILD_PROFILE/libsokobanitron_android_jni.so"
DEST_DIR="$ANDROID_CLIENT_DIR/app/src/main/jniLibs/$ANDROID_ABI_DIR"
DEST_SO="$DEST_DIR/libsokobanitron_android_jni.so"

if [[ ! -f "$SOURCE_SO" ]]; then
  echo "Build finished but expected output is missing: $SOURCE_SO"
  exit 1
fi

mkdir -p "$DEST_DIR"
cp "$SOURCE_SO" "$DEST_SO"

echo "Copied:"
echo "  $SOURCE_SO"
echo "to:"
echo "  $DEST_SO"
echo
echo "Next:"
echo "  cd \"$ANDROID_CLIENT_DIR\" && ./gradlew :app:assembleDebug"
