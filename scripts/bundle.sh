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
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/inkling"
ln -s "$(pwd)/brain" "$APP/Contents/Resources/brain"
cp assets/Inkling.icns "$APP/Contents/Resources/Inkling.icns"
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
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSMicrophoneUsageDescription</key><string>Inkling listens while you hold Option, to turn speech into text.</string>
  <key>NSSpeechRecognitionUsageDescription</key><string>Inkling turns what you say into text in your note.</string>
</dict>
</plist>
PLIST
echo "built $APP ($PROFILE)"
