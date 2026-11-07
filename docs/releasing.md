# Releasing Inkling

[CI](../.github/workflows/ci.yml) runs on pull requests and pushes to `master`. On an Apple Silicon runner it installs locked dependencies, runs `scripts/dev check` and builds the release ZIP. Packaging checks the extracted bundle’s signatures and architectures, then runs the standalone brain with Finder’s limited `PATH` and verifies that the SDK launches a test executable from Claude Code’s native install path. GUI scenarios run separately with `scripts/dev check --e2e` on a Mac. Neither check needs Claude credentials.

For a downloadable preview, run **CI → Run workflow** in GitHub Actions and download the `inkling-macos-arm64` artifact from the completed run. It contains the install ZIP and its SHA-256 file. The same files can be built locally with `scripts/package.sh`. Follow the [install instructions](../README.md#install-and-run) to try the preview.

The release helper embeds [Bun 1.4.2](https://github.com/oven-sh/bun/tree/bun-v1.4.2). The bundle includes Bun’s notices, with links to its JavaScriptCore source and relinking instructions. To rebuild the helper with a relinked Bun, put that `bun` first on `PATH` and run `scripts/package.sh` from the matching Inkling source tag.

## Cut a release

1. Keep user-facing changes under `## Unreleased` in [CHANGELOG.md](../CHANGELOG.md). In the release PR, move those entries under an exact version heading such as `## 0.1.0` and leave a fresh `## Unreleased` above it.
2. Set the same version in `Cargo.toml` and refresh `Cargo.lock` with `cargo check` if the version changed. The app bundle reads its version from Cargo. Update the brain’s package version, handshake version and SDK client version to match.
3. Merge the release PR into `master`. Once CI passes, tag the exact release commit from `master`. For example, if that commit is the current tip:

   ```sh
   git switch master
   git pull --ff-only origin master
   scripts/release-notes.sh v0.1.0
   git tag -a v0.1.0 -m "Inkling v0.1.0"
   git push origin v0.1.0
   ```

[Release](../.github/workflows/release.yml) checks that the tag points to a commit on `master` and matches both Cargo and a nonempty changelog section. It runs the same checks and package build as CI. After verifying the artifact’s checksum, it uploads the ZIP and checksum to a draft GitHub Release, then publishes the draft with the notes from the changelog. The app uses an ad hoc signature and ships without Apple notarization.

If upload or publication fails, rerun the failed job to finish the draft. If the tagged source itself needs a fix, merge it and cut a new version. Published tags identify the exact source used for each ZIP.
