#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ANDROID_CLIENT_DIR="$REPO_ROOT/android-client"
APP_ID="com.sokobanitron.app.dev"
APP_ACTIVITY="com.sokobanitron.app.dev.MainActivity"
LOCAL_LEVEL_SETS_ROOT="$REPO_ROOT/tmp/level_sets"
LOCAL_TO_IMPORT_DIR="$LOCAL_LEVEL_SETS_ROOT/to_import"
LOCAL_IMPORTED_DIR="$LOCAL_LEVEL_SETS_ROOT/imported"

archive_transferred_level_sets() {
  mkdir -p "$LOCAL_IMPORTED_DIR"

  shopt -s nullglob
  local source_paths=("$LOCAL_TO_IMPORT_DIR"/*.slc)
  shopt -u nullglob

  if [[ "${#source_paths[@]}" -eq 0 ]]; then
    return 0
  fi

  local source_path file_name destination stem extension suffix
  for source_path in "${source_paths[@]}"; do
    file_name="$(basename "$source_path")"
    destination="$LOCAL_IMPORTED_DIR/$file_name"

    if [[ -e "$destination" ]]; then
      stem="${file_name%.*}"
      extension="${file_name##*.}"
      suffix=2
      while [[ -e "$LOCAL_IMPORTED_DIR/$stem-$suffix.$extension" ]]; do
        suffix=$((suffix + 1))
      done
      destination="$LOCAL_IMPORTED_DIR/$stem-$suffix.$extension"
    fi

    mv "$source_path" "$destination"
  done
}

if ! command -v adb >/dev/null 2>&1; then
  echo "adb is required but was not found in PATH."
  exit 1
fi

if [[ ! -x "$ANDROID_CLIENT_DIR/gradlew" ]]; then
  echo "Missing executable Gradle wrapper: $ANDROID_CLIENT_DIR/gradlew"
  exit 1
fi

DEVICE_LINES="$(adb devices | tail -n +2 | grep -E '\s+device$' || true)"
DEVICE_COUNT="$(printf '%s\n' "$DEVICE_LINES" | sed '/^$/d' | wc -l | tr -d ' ')"

if [[ "$DEVICE_COUNT" -eq 0 ]]; then
  echo "No Android device detected. Connect a device with USB debugging enabled."
  exit 1
fi

if [[ "$DEVICE_COUNT" -gt 1 && -z "${ANDROID_SERIAL:-}" ]]; then
  echo "Multiple Android devices detected. Set ANDROID_SERIAL to choose one:"
  printf '%s\n' "$DEVICE_LINES"
  exit 1
fi

echo "Building Rust JNI library (release)..."
"$REPO_ROOT/scripts/build_android_jni_device.sh" release

echo "Installing Android app..."
(
  cd "$ANDROID_CLIENT_DIR"
  ./gradlew :app:installDebug
)

archive_transferred_level_sets

echo "Launching Android app..."
adb shell am force-stop "$APP_ID" >/dev/null 2>&1 || true
adb shell am start -n "$APP_ID/$APP_ACTIVITY"

echo
echo "Launched $APP_ID on the connected device."
