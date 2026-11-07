#!/bin/sh
# Validate a stable version tag and print its changelog section.
set -eu
cd "$(dirname "$0")/.."

tag=${1:?usage: scripts/release-notes.sh vMAJOR.MINOR.PATCH}
if ! printf '%s\n' "$tag" | grep -Eq '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$'; then
  echo "Expected a stable version tag (vMAJOR.MINOR.PATCH), got $tag" >&2
  exit 1
fi
package=$(cargo pkgid --locked)
version=${package##*[#@]}
if [ "$tag" != "v$version" ]; then
  echo "Tag $tag must match Cargo.toml version $version" >&2
  exit 1
fi
notes=$(awk -v heading="## ${tag#v}" '
  $0 == heading { found = 1; next }
  found && /^## / { exit }
  found { print }
' CHANGELOG.md)
if ! printf '%s\n' "$notes" | grep -q '[^[:space:]]'; then
  echo "CHANGELOG.md needs a nonempty section headed '## ${tag#v}'" >&2
  exit 1
fi
printf '%s\n' "$notes"
