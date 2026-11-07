#!/bin/sh
# Produce an installable ZIP and checksum in target/dist.
set -eu
cd "$(dirname "$0")/.."

if [ "$(uname -sm)" != "Darwin arm64" ]; then
  echo "Release archives are built on Apple Silicon Macs" >&2
  exit 1
fi

scripts/bundle.sh release --portable
app=target/Inkling.app
version=$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' "$app/Contents/Info.plist")
archive="Inkling-$version-macos-arm64.zip"
mkdir -p target/dist
# ditto updates existing archives, so start from an empty destination.
rm -f "target/dist/$archive" "target/dist/$archive.sha256"
ditto -c -k --keepParent "$app" "target/dist/$archive"

# Verify the app as a downloader receives it, including its bundled sidecar.
unpacked=$(mktemp -d)
trap 'rm -r "$unpacked"' EXIT
ditto -x -k "target/dist/$archive" "$unpacked"
app="$unpacked/Inkling.app"
plutil -lint "$app/Contents/Info.plist"
codesign --verify --strict "$app"
test "$(lipo -archs "$app/Contents/MacOS/inkling")" = arm64
brain="$app/Contents/Resources/brain"
test ! -L "$brain"
for package in "$brain"/node_modules/@anthropic-ai/claude-agent-sdk-*; do
  if [ -e "$package" ]; then
    echo "The ZIP should use the installed Claude Code, but includes $package" >&2
    exit 1
  fi
done
(
  cd "$brain"
  # Wait for the startup handshake without requesting account status or model output.
  ready=$(printf '{"op":"shutdown"}\n' | bun run src/main.ts)
  printf '%s\n' "$ready" | grep -F '"ev":"ready"'
)
(cd target/dist && shasum -a 256 "$archive" > "$archive.sha256")
echo "built target/dist/$archive"
