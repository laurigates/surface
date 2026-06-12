#!/bin/sh
# Surface installer. Detects (os, arch), downloads the matching release binary, installs it.
#
#   curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/Connorrmcd6/surface/main/install.sh | sh
#
# Env:
#   SURF_VERSION       tag to install (e.g. v0.1.0); default: latest release
#   SURF_INSTALL_DIR   install directory; default: $HOME/.local/bin
set -eu

REPO="Connorrmcd6/surface"
INSTALL_DIR="${SURF_INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Darwin)
    case "$arch" in
      arm64 | aarch64) target="aarch64-apple-darwin" ;;
      x86_64)
        echo "surf: no prebuilt binary for Intel macOS. Install from source instead:" >&2
        echo "  cargo install --git https://github.com/Connorrmcd6/surface surf-cli" >&2
        exit 1 ;;
      *) echo "surf: unsupported macOS arch: $arch" >&2; exit 1 ;;
    esac ;;
  Linux)
    case "$arch" in
      x86_64) target="x86_64-unknown-linux-gnu" ;;
      aarch64 | arm64)
        echo "surf: no prebuilt binary for ARM64 Linux yet. Install from source instead:" >&2
        echo "  cargo install --git https://github.com/Connorrmcd6/surface surf-cli" >&2
        exit 1 ;;
      *) echo "surf: unsupported Linux arch: $arch" >&2; exit 1 ;;
    esac ;;
  *) echo "surf: unsupported OS: $os" >&2; exit 1 ;;
esac

tag="${SURF_VERSION:-latest}"
if [ "$tag" = "latest" ]; then
  tag="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' | head -1 | cut -d'"' -f4)"
  [ -n "$tag" ] || { echo "surf: could not determine latest release" >&2; exit 1; }
fi

url="https://github.com/$REPO/releases/download/$tag/surf-$target.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# SHA-256 of a file, using whichever tool the platform ships (#39).
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  else
    return 1
  fi
}

echo "surf: downloading $tag ($target)..."
curl --proto '=https' --tlsv1.2 -fsSL "$url" -o "$tmp/surf.tar.gz" \
  || { echo "surf: no release asset at $url" >&2; exit 1; }

# Verify integrity before unpacking. Fail closed: a missing checksum or a mismatch aborts the
# install rather than running an unverified binary (the threat #39 addresses). The checksum file
# lists `<hash>  surf-<target>.tar.gz`; we compare the hash value, since the local copy is renamed.
echo "surf: verifying checksum..."
curl --proto '=https' --tlsv1.2 -fsSL "$url.sha256" -o "$tmp/surf.sha256" \
  || { echo "surf: no checksum at $url.sha256 (release may predate checksums; install from source instead)" >&2; exit 1; }
expected="$(cut -d' ' -f1 < "$tmp/surf.sha256")"
actual="$(sha256_of "$tmp/surf.tar.gz")" \
  || { echo "surf: need 'sha256sum' or 'shasum' to verify the download" >&2; exit 1; }
if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
  echo "surf: checksum mismatch — refusing to install" >&2
  echo "  expected: $expected" >&2
  echo "  actual:   $actual" >&2
  exit 1
fi

tar -xzf "$tmp/surf.tar.gz" -C "$tmp"

mkdir -p "$INSTALL_DIR"
install -m 0755 "$tmp/surf" "$INSTALL_DIR/surf"
echo "surf: installed $tag to $INSTALL_DIR/surf"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) echo "surf: add $INSTALL_DIR to your PATH to run \`surf\`." ;;
esac
