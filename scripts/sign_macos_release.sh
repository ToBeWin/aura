#!/bin/bash

set -euo pipefail

APP_PATH="${1:-/tmp/aura-mac-universal/release/bundle/macos/Aura.app}"
DMG_PATH="${2:-/tmp/aura-mac-universal/release/bundle/macos/Aura-macos-universal.dmg}"

SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:-}"
TEAM_ID="${APPLE_TEAM_ID:-}"
APPLE_ID="${APPLE_ID:-}"
APPLE_PASSWORD="${APPLE_PASSWORD:-}"
NOTARY_PROFILE="${APPLE_NOTARY_PROFILE:-}"
ASC_KEY_PATH="${APPLE_API_KEY_PATH:-}"
ASC_KEY_ID="${APPLE_API_KEY_ID:-}"
ASC_ISSUER="${APPLE_API_ISSUER:-}"

function section() {
  echo
  echo "== $1 =="
}

function fail() {
  echo "Error: $1" >&2
  exit 1
}

function require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    fail "Missing required command: $1"
  fi
}

function has_notary_credentials() {
  if [[ -n "$NOTARY_PROFILE" ]]; then
    return 0
  fi

  if [[ -n "$ASC_KEY_PATH" && -n "$ASC_KEY_ID" && -n "$ASC_ISSUER" ]]; then
    return 0
  fi

  if [[ -n "$APPLE_ID" && -n "$APPLE_PASSWORD" && -n "$TEAM_ID" ]]; then
    return 0
  fi

  return 1
}

function submit_notary() {
  local artifact_path="$1"

  if [[ -n "$NOTARY_PROFILE" ]]; then
    xcrun notarytool submit "$artifact_path" --keychain-profile "$NOTARY_PROFILE" --wait
    return
  fi

  if [[ -n "$ASC_KEY_PATH" && -n "$ASC_KEY_ID" && -n "$ASC_ISSUER" ]]; then
    xcrun notarytool submit \
      "$artifact_path" \
      --key "$ASC_KEY_PATH" \
      --key-id "$ASC_KEY_ID" \
      --issuer "$ASC_ISSUER" \
      --wait
    return
  fi

  xcrun notarytool submit \
    "$artifact_path" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_PASSWORD" \
    --team-id "$TEAM_ID" \
    --wait
}

require_cmd codesign
require_cmd xcrun
require_cmd spctl

[[ -d "$APP_PATH" ]] || fail "App bundle not found: $APP_PATH"

if [[ -z "$SIGNING_IDENTITY" ]]; then
  fail "APPLE_SIGNING_IDENTITY is required. Expected a Developer ID Application identity."
fi

section "Signing Aura.app"
codesign \
  --force \
  --deep \
  --options runtime \
  --timestamp \
  --sign "$SIGNING_IDENTITY" \
  "$APP_PATH"

section "Verifying Aura.app signature"
codesign --verify --deep --strict --verbose=2 "$APP_PATH"
spctl -a -vvv --type exec "$APP_PATH" || true

if [[ -f "$DMG_PATH" ]]; then
  section "Signing DMG"
  codesign \
    --force \
    --timestamp \
    --sign "$SIGNING_IDENTITY" \
    "$DMG_PATH"

  section "Verifying DMG signature"
  codesign --verify --verbose=2 "$DMG_PATH"
else
  echo
  echo "DMG not found, skipping DMG signing: $DMG_PATH"
fi

if has_notary_credentials; then
  if [[ -f "$DMG_PATH" ]]; then
    section "Submitting DMG for notarization"
    submit_notary "$DMG_PATH"

    section "Stapling notarization ticket to DMG"
    xcrun stapler staple "$DMG_PATH"
  else
    section "Submitting Aura.app for notarization"
    submit_notary "$APP_PATH"

    section "Stapling notarization ticket to Aura.app"
    xcrun stapler staple "$APP_PATH"
  fi
else
  echo
  echo "No notarization credentials detected. Signing completed, notarization skipped."
fi

echo
echo "Signing flow finished."
