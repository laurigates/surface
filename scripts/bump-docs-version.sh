#!/usr/bin/env sh
# Rewrites pinned `Connorrmcd6/surface@vX.Y.Z` refs in the README and docs to match the
# workspace version in Cargo.toml — the single source of truth. Run during release prep
# so the canonical docs carry the right version before the tag (the site sync just copies).
set -eu

cd "$(dirname "$0")/.."

version=$(awk '
  /^\[workspace\.package\]/ { in_ws = 1; next }
  /^\[/ { in_ws = 0 }
  in_ws && /^version *= *"/ {
    match($0, /"[^"]+"/); print substr($0, RSTART + 1, RLENGTH - 2); exit
  }
' Cargo.toml)

if [ -z "$version" ]; then
  echo "error: could not read [workspace.package] version from Cargo.toml" >&2
  exit 1
fi

files=$(grep -rlE 'Connorrmcd6/surface@v[0-9]+\.[0-9]+\.[0-9]+' README.md docs || true)
if [ -z "$files" ]; then
  echo "no pinned version refs found"
  exit 0
fi

for f in $files; do
  sed -i.bak -E "s#(Connorrmcd6/surface@v)[0-9]+\.[0-9]+\.[0-9]+#\1${version}#g" "$f"
  rm -f "$f.bak"
  echo "bumped $f -> v$version"
done
