#!/bin/bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_NAME="Aura"
VERSION="0.1.0"
MACOS_MIN="10.15"

ARM_TARGET="aarch64-apple-darwin"
INTEL_TARGET="x86_64-apple-darwin"

ARM_BUILD_ROOT="/tmp/aura-mac-build"
INTEL_BUILD_ROOT="/tmp/aura-mac-intel-build"
UNIVERSAL_ROOT="/tmp/aura-mac-universal"
DMG_STAGE_ROOT="/tmp/aura-mac-dmg-stage"

ARM_APP="$ARM_BUILD_ROOT/release/bundle/macos/${APP_NAME}.app"
INTEL_APP="$INTEL_BUILD_ROOT/$INTEL_TARGET/release/bundle/macos/${APP_NAME}.app"
UNIVERSAL_APP="$UNIVERSAL_ROOT/release/bundle/macos/${APP_NAME}.app"

ARM_ZIP="$ARM_BUILD_ROOT/release/bundle/macos/${APP_NAME}-macos-aarch64.zip"
INTEL_ZIP="$INTEL_BUILD_ROOT/$INTEL_TARGET/release/bundle/macos/${APP_NAME}-macos-x86_64.zip"
UNIVERSAL_ZIP="$UNIVERSAL_ROOT/release/bundle/macos/${APP_NAME}-macos-universal.zip"
UNIVERSAL_DMG="$UNIVERSAL_ROOT/release/bundle/macos/${APP_NAME}-macos-universal.dmg"
SIGN_SCRIPT="$ROOT_DIR/scripts/sign_macos_release.sh"

function section() {
  echo
  echo "== $1 =="
}

function require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1"
    exit 1
  fi
}

function ensure_rust_target() {
  local target="$1"
  if ! rustup target list --installed | grep -q "^${target}$"; then
    echo "Installing Rust target ${target}..."
    rustup target add "$target"
  fi
}

function build_app_bundle() {
  local cargo_target_dir="$1"
  local target="$2"

  if [[ "$target" == "$ARM_TARGET" ]]; then
    CARGO_TARGET_DIR="$cargo_target_dir" \
    MACOSX_DEPLOYMENT_TARGET="$MACOS_MIN" \
    CMAKE_OSX_DEPLOYMENT_TARGET="$MACOS_MIN" \
    CMAKE_ARGS="-DCMAKE_OSX_DEPLOYMENT_TARGET=${MACOS_MIN}" \
    npm run tauri -- build --bundles app
  else
    CARGO_TARGET_DIR="$cargo_target_dir" \
    MACOSX_DEPLOYMENT_TARGET="$MACOS_MIN" \
    CMAKE_OSX_DEPLOYMENT_TARGET="$MACOS_MIN" \
    CMAKE_ARGS="-DCMAKE_OSX_DEPLOYMENT_TARGET=${MACOS_MIN}" \
    npm run tauri -- build --target "$target" --bundles app
  fi
}

function zip_app() {
  local app_path="$1"
  local zip_path="$2"
  rm -f "$zip_path"
  ditto -c -k --sequesterRsrc --keepParent "$app_path" "$zip_path"
}

function adhoc_sign_app() {
  local app_path="$1"
  if [[ ! -d "$app_path" ]]; then
    echo "App bundle not found for ad-hoc signing: $app_path"
    exit 1
  fi

  codesign \
    --force \
    --deep \
    --sign - \
    "$app_path"
}

function build_universal_app() {
  rm -rf "$UNIVERSAL_APP"
  mkdir -p "$(dirname "$UNIVERSAL_APP")"
  cp -R "$ARM_APP" "$UNIVERSAL_APP"

  lipo -create \
    -output "$UNIVERSAL_APP/Contents/MacOS/aura" \
    "$ARM_APP/Contents/MacOS/aura" \
    "$INTEL_APP/Contents/MacOS/aura"
}

function build_drag_install_dmg() {
  local stage_dir="$DMG_STAGE_ROOT/${APP_NAME}"
  local rw_dmg="${UNIVERSAL_DMG%.dmg}-rw.dmg"
  local volume_name="$APP_NAME"
  local mount_output=""
  local device=""
  local mount_point="/Volumes/${volume_name}"

  rm -rf "$stage_dir"
  mkdir -p "$stage_dir"

  cp -R "$UNIVERSAL_APP" "$stage_dir/"
  ln -sfn /Applications "$stage_dir/Applications"

  rm -f "$UNIVERSAL_DMG"
  rm -f "$rw_dmg"

  hdiutil create \
    -volname "$volume_name" \
    -srcfolder "$stage_dir" \
    -ov \
    -format UDRW \
    "$rw_dmg"

  mount_output="$(hdiutil attach "$rw_dmg" -readwrite -noverify -noautoopen)"
  device="$(echo "$mount_output" | awk '/Apple_HFS/ {print $1; exit}')"

  if [[ -z "$device" ]]; then
    echo "Failed to mount temporary DMG."
    exit 1
  fi

  if command -v osascript >/dev/null 2>&1; then
    osascript <<EOF
tell application "Finder"
  tell disk "${volume_name}"
    open
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set bounds of container window to {160, 120, 820, 460}
    set theViewOptions to the icon view options of container window
    set arrangement of theViewOptions to not arranged
    set icon size of theViewOptions to 128
    set text size of theViewOptions to 16
    set position of item "${APP_NAME}.app" of container window to {170, 170}
    set position of item "Applications" of container window to {470, 170}
    close
    open
    update without registering applications
    delay 1
  end tell
end tell
EOF
  fi

  sync
  hdiutil detach "$device"

  hdiutil convert "$rw_dmg" -format UDZO -ov -o "$UNIVERSAL_DMG"
  rm -f "$rw_dmg"
}

function maybe_sign_release() {
  if [[ "${SIGN_MACOS_RELEASE:-0}" != "1" ]]; then
    echo "Signing skipped. Set SIGN_MACOS_RELEASE=1 to sign the release artifacts."
    return
  fi

  if [[ ! -x "$SIGN_SCRIPT" ]]; then
    echo "Signing script not executable: $SIGN_SCRIPT"
    exit 1
  fi

  "$SIGN_SCRIPT" "$UNIVERSAL_APP" "$UNIVERSAL_DMG"
}

section "Checking prerequisites"
require_cmd cargo
require_cmd rustup
require_cmd npm
require_cmd node
require_cmd lipo
require_cmd ditto
require_cmd hdiutil
require_cmd codesign

section "Ensuring Rust targets"
ensure_rust_target "$ARM_TARGET"
ensure_rust_target "$INTEL_TARGET"

section "Installing Node dependencies"
npm install

section "Building Apple Silicon app"
build_app_bundle "$ARM_BUILD_ROOT" "$ARM_TARGET"

section "Building Intel app"
build_app_bundle "$INTEL_BUILD_ROOT" "$INTEL_TARGET"

section "Packaging per-architecture ZIP files"
zip_app "$ARM_APP" "$ARM_ZIP"
zip_app "$INTEL_APP" "$INTEL_ZIP"

section "Creating universal macOS app"
build_universal_app

section "Applying required ad-hoc signature"
adhoc_sign_app "$UNIVERSAL_APP"

zip_app "$UNIVERSAL_APP" "$UNIVERSAL_ZIP"

section "Creating drag-to-Applications DMG"
if ! build_drag_install_dmg; then
  echo "Warning: DMG creation failed. ZIP artifacts are still available."
fi

section "Signing and notarization"
maybe_sign_release

section "Build artifacts"
if [[ -f "$UNIVERSAL_DMG" ]]; then
  ls -lah "$ARM_ZIP" "$INTEL_ZIP" "$UNIVERSAL_ZIP" "$UNIVERSAL_DMG"
else
  ls -lah "$ARM_ZIP" "$INTEL_ZIP" "$UNIVERSAL_ZIP"
fi

echo
echo "Done."
