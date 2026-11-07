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
codesign --verify --deep --strict "$app"
test "$(lipo -archs "$app/Contents/MacOS/inkling")" = arm64
brain="$app/Contents/Resources/brain"
test ! -L "$brain"
helper="$brain/dist/inkling-brain"
test "$(lipo -archs "$helper")" = arm64
test ! -e "$brain/node_modules"
test ! -e "$brain/src"
(
  cd "$brain"
  # Exercise the standalone helper using a fresh home and Finder's PATH.
  # The stub records the launch and exits before a session can start.
  mkdir -p "$unpacked/home/.local/bin"
  cat > "$unpacked/home/.local/bin/claude" <<'STUB'
#!/bin/sh
printf '%s\n' "$@" > "$INKLING_TEST_LAUNCH"
exit 1
STUB
  chmod +x "$unpacked/home/.local/bin/claude"
  env -i HOME="$unpacked/home" PATH=/usr/bin:/bin INKLING_DATA_DIR="$unpacked/data" \
    INKLING_TEST_LAUNCH="$unpacked/claude-args" INKLING_TEST_HELPER="$helper" \
    "$(command -v bun)" -e '
      import assert from "node:assert/strict";
      import { createInterface } from "node:readline";
      import { Readable } from "node:stream";
      const child = Bun.spawn([process.env.INKLING_TEST_HELPER], {
        stdin: "pipe", stdout: "pipe", stderr: "inherit", signal: AbortSignal.timeout(10000),
      });
      const lines = createInterface({ input: Readable.fromWeb(child.stdout) })[Symbol.asyncIterator]();
      const read = async () => JSON.parse((await lines.next()).value);
      assert.equal((await read()).ev, "ready");
      child.stdin.write(JSON.stringify({ op: "status", id: "packaging" }) + "\n");
      child.stdin.flush();
      const status = await read();
      assert.equal(status.id, "packaging");
      assert.equal(status.ev, "error"); // The stub exits deliberately.
      child.stdin.write(JSON.stringify({ op: "shutdown" }) + "\n");
      child.stdin.end();
      assert.equal(await child.exited, 0);
    '
  grep -qx -- '--input-format' "$unpacked/claude-args"
)
(cd target/dist && shasum -a 256 "$archive" > "$archive.sha256")
echo "built target/dist/$archive"
