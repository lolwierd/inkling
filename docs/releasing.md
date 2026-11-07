# Releasing Inkling

[CI](../.github/workflows/ci.yml) runs on pull requests and on pushes to `master`. On an Apple Silicon runner it installs the locked dependencies, runs `scripts/dev check` and builds the release ZIP. Packaging then unzips the bundle and checks its signatures and architectures. It also runs the standalone brain with the limited `PATH` Finder gives apps and makes sure the SDK can launch a test executable from Claude Code's native install path. The GUI scenarios aren't part of CI; run them locally with `scripts/dev check --e2e`. Neither needs Claude credentials.

To get a preview build, open CI in GitHub Actions, choose Run workflow, and download the `inkling-macos-arm64` artifact when the run finishes. It has the install ZIP and its SHA-256 file. `scripts/package.sh` builds the same files locally. The [install instructions](../README.md#install) work for a preview too.

The release helper embeds [Bun 1.4.2](https://github.com/oven-sh/bun/tree/bun-v1.4.2). The bundle includes Bun's notices, which link to its JavaScriptCore source and explain how to relink it. To rebuild the helper with a relinked Bun, put that `bun` first on `PATH` and run `scripts/package.sh` from the matching Inkling tag.

## Cut a release

1. User-facing changes go under `## Unreleased` in [CHANGELOG.md](../CHANGELOG.md) as they land. When you release, move them under a version heading such as `## 0.1.0` and leave an empty `## Unreleased` above it.
2. Set the new version in `Cargo.toml` and run `cargo check` so `Cargo.lock` picks it up. The app bundle reads its version from Cargo. Bump the brain to match: its package version, its handshake version and its SDK client version, and the tests that check them.
3. Get the release commit onto `master`. Once CI passes on it, tag that exact commit. If it's the current tip:

   ```sh
   git switch master
   git pull --ff-only origin master
   scripts/release-notes.sh v0.1.0
   git tag -a v0.1.0 -m "Inkling v0.1.0"
   git push origin v0.1.0
   ```

The [Release](../.github/workflows/release.yml) workflow checks that the tag is on `master`, that it matches the Cargo version, and that the changelog has a section for it. It runs the same checks and package build as CI, verifies the artifact's checksum, uploads the ZIP and checksum to a draft GitHub Release, and then publishes the draft with that changelog section as its notes. The app is signed ad hoc and isn't notarized by Apple.

If the upload or publish step fails, rerun the failed job and it will finish the draft. If the tagged source itself needs a fix, merge the fix and cut a new version. Don't move or reuse a published tag: each one points at the exact source its ZIP was built from.
