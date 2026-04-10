#!/bin/bash

set -euo pipefail

APP_NAME="${1:-Aura}"
UNIVERSAL_APP="${2:-/tmp/aura-mac-universal/release/bundle/macos/Aura.app}"
UNIVERSAL_DMG="${3:-/tmp/aura-mac-universal/release/bundle/macos/Aura-macos-universal.dmg}"

STAGE_DIR="/tmp/aura-mac-dmg-stage/${APP_NAME}"
RW_DMG="${UNIVERSAL_DMG%.dmg}-rw.dmg"

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"
cp -R "$UNIVERSAL_APP" "$STAGE_DIR/"
ln -sfn /Applications "$STAGE_DIR/Applications"
rm -f "$UNIVERSAL_DMG" "$RW_DMG"

hdiutil create -volname "$APP_NAME" -srcfolder "$STAGE_DIR" -ov -format UDRW "$RW_DMG"

MOUNT_OUTPUT="$(hdiutil attach "$RW_DMG" -readwrite -noverify -noautoopen)"
DEVICE="$(printf "%s\n" "$MOUNT_OUTPUT" | awk '/Apple_HFS|APFS/ {print $1; exit}')"

if [[ -z "$DEVICE" ]]; then
  echo "Failed to mount DMG"
  exit 1
fi

osascript <<EOF
tell application "Finder"
  tell disk "${APP_NAME}"
    open
    delay 1
    update without registering applications
    delay 1
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set bounds of container window to {180, 140, 760, 420}
    set theViewOptions to the icon view options of container window
    set arrangement of theViewOptions to arranged by grid
    set icon size of theViewOptions to 132
    set text size of theViewOptions to 16
    close
    open
    delay 1
  end tell
end tell
EOF

sync
hdiutil detach "$DEVICE"
hdiutil convert "$RW_DMG" -format UDZO -ov -o "$UNIVERSAL_DMG"
rm -f "$RW_DMG"

echo "$UNIVERSAL_DMG"
