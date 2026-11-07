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

  # Exercise the real SDK's launcher using a fresh home and Finder's PATH.
  # The stub records the launch and exits before a session can start.
  mkdir -p "$unpacked/home/.local/bin"
  cat > "$unpacked/home/.local/bin/claude" <<'STUB'
#!/bin/sh
printf '%s\n' "$@" > "$INKLING_TEST_LAUNCH"
exit 1
STUB
  chmod +x "$unpacked/home/.local/bin/claude"
  env -i HOME="$unpacked/home" PATH=/usr/bin:/bin INKLING_DATA_DIR="$unpacked/data" \
    INKLING_TEST_LAUNCH="$unpacked/claude-args" "$(command -v bun)" -e '
      import { accountStatus } from "./src/claude.ts";
      await accountStatus(3000).catch(() => {});
    '
  grep -qx -- '--input-format' "$unpacked/claude-args"
)
(cd target/dist && shasum -a 256 "$archive" > "$archive.sha256")
echo "built target/dist/$archive"
