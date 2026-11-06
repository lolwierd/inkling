#!/bin/sh
# Build Inkling and wrap it in a minimal macOS app bundle at target/Inkling.app.
# The bundle links to this checkout's brain/ folder, so edits to the sidecar
# take effect on the next launch without rebuilding.
#   ./scripts/bundle.sh            debug build
#   ./scripts/bundle.sh release    optimized build
set -eu
cd "$(dirname "$0")/.."
PROFILE="${1:-debug}"
case "$PROFILE" in
  debug) cargo build ;;
  release) cargo build --release ;;
  *) echo "usage: $0 [debug|release]" >&2; exit 2 ;;
esac
# The binary is wherever cargo writes, which CARGO_TARGET_DIR or a cargo config
# can move away from ./target; cargo metadata says where.
TARGET_DIR=$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
BIN="${TARGET_DIR:-target}/$PROFILE/inkling"
APP=target/Inkling.app
MIN_MACOS=13.0
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/inkling"
ln -s "$(pwd)/brain" "$APP/Contents/Resources/brain"

# The icon. Inkling.icns is one drawing, the light Initial, whatever the system's style.
# macOS draws an app icon in light, dark, clear and tinted only from a compiled asset
# catalog: Xcode's actool turns assets/Inkling.icon (the Initial in layers, made by
# assets/icon-layers.py) into Assets.car, which Info.plist names (CFBundleIconName),
# keeping the .icns as the fallback (CFBundleIconFile). The Command Line Tools have no
# actool, so without Xcode the bundle has the .icns alone.
cp assets/Inkling.icns "$APP/Contents/Resources/Inkling.icns"
ICON_NAME=
DEV=$(xcode-select -p 2>/dev/null || true)
case "$DEV" in
  *.app/Contents/Developer) ;;
  *) DEV=/Applications/Xcode.app/Contents/Developer ;;
esac
OUT=target/icon
rm -rf "$OUT"
mkdir -p "$OUT"
if [ ! -x "$DEV/usr/bin/actool" ]; then
  ICON="Inkling.icns alone, light in every style: no Xcode, so no actool to compile assets/Inkling.icon"
elif DEVELOPER_DIR="$DEV" xcrun actool "$(pwd)/assets/Inkling.icon" \
    --compile "$(pwd)/$OUT" --output-partial-info-plist "$(pwd)/$OUT/partial.plist" \
    --app-icon Inkling --platform macosx --target-device mac --minimum-deployment-target "$MIN_MACOS" \
    --warnings --errors --output-format human-readable-text >"$OUT/actool.log" 2>&1 &&
    [ -f "$OUT/Assets.car" ]; then
  grep ": warning:" "$OUT/actool.log" >&2 || true
  cp "$OUT/Assets.car" "$APP/Contents/Resources/Assets.car"
  ICON_NAME=Inkling
  ICON="assets/Inkling.icon, compiled: light, dark, clear and tinted (Inkling.icns as the fallback)"
elif grep -qi "license" "$OUT/actool.log"; then
  ICON="Inkling.icns alone: Xcode's license is not accepted yet (open Xcode once)"
else
  cat "$OUT/actool.log" >&2
  ICON="Inkling.icns alone: actool could not compile assets/Inkling.icon (see $OUT/actool.log)"
fi
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Inkling</string>
  <key>CFBundleDisplayName</key><string>Inkling</string>
  <key>CFBundleIdentifier</key><string>dev.local.inkling</string>
  <key>CFBundleExecutable</key><string>inkling</string>
  <key>CFBundleIconFile</key><string>Inkling</string>
${ICON_NAME:+  <key>CFBundleIconName</key><string>$ICON_NAME</string>}
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>$MIN_MACOS</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSMicrophoneUsageDescription</key><string>Inkling listens while you hold Option, to turn speech into text.</string>
  <key>NSSpeechRecognitionUsageDescription</key><string>Inkling turns what you say into text in your note.</string>
</dict>
</plist>
PLIST
echo "icon: $ICON"
echo "built $APP ($PROFILE)"
