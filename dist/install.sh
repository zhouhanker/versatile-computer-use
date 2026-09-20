#!/usr/bin/env bash
# VCU one-line installer (macOS/Linux):
#   curl -fsSL <url>/install.sh | sh
#   curl -fsSL <url>/install.sh | VCU_PREFIX=~/.local sh
#
# Does NOT require npm, cargo, or a source checkout when a release archive is available.
set -euo pipefail

VCU_VERSION="${VCU_VERSION:-latest}"
VCU_PREFIX="${VCU_PREFIX:-$HOME/.local}"
VCU_BIN_DIR="${VCU_BIN_DIR:-$VCU_PREFIX/bin}"
VCU_SHARE_DIR="${VCU_SHARE_DIR:-$VCU_PREFIX/share/vcu}"
# Base URL hosting archives. Override for private mirrors or local file server.
# Example local test: VCU_BASE_URL=file:///path/to/dist
VCU_BASE_URL="${VCU_BASE_URL:-https://github.com/zhouhanker/versatile-computer-use/releases/latest/download}"
VCU_REPO_LATEST_API="${VCU_REPO_LATEST_API:-}"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64) ARCH_LABEL=x64 ;;
  arm64|aarch64) ARCH_LABEL=arm64 ;;
  *) echo "Unsupported arch: $ARCH" >&2; exit 1 ;;
esac
case "$OS" in
  darwin) PLAT="macos-${ARCH_LABEL}" ;;
  linux) PLAT="linux-${ARCH_LABEL}" ;;
  *) echo "Unsupported OS: $OS (use install.ps1 on Windows)" >&2; exit 1 ;;
esac

need_cmd() { command -v "$1" >/dev/null 2>&1 || { echo "missing required command: $1" >&2; exit 1; }; }
need_cmd curl
need_cmd tar
need_cmd shasum || need_cmd sha256sum

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

resolve_archive_url() {
  if [[ -n "${VCU_ARCHIVE_URL:-}" ]]; then
    echo "$VCU_ARCHIVE_URL"
    return
  fi
  if [[ "$VCU_BASE_URL" == file://* ]]; then
    local base="${VCU_BASE_URL#file://}"
    if [[ "$VCU_VERSION" == "latest" ]]; then
      echo "file://${base}/vcu-latest-${PLAT}.tar.gz"
    else
      echo "file://${base}/vcu-${VCU_VERSION}-${PLAT}.tar.gz"
    fi
    return
  fi
  if [[ "$VCU_VERSION" == "latest" ]]; then
    # default BASE is .../releases/latest/download
    echo "${VCU_BASE_URL%/}/vcu-latest-${PLAT}.tar.gz"
  else
    # versioned: .../releases/download/vX.Y.Z/
    base="${VCU_BASE_URL%/}"
    base="${base%/latest/download}"
    echo "${base}/download/v${VCU_VERSION}/vcu-${VCU_VERSION}-${PLAT}.tar.gz"
  fi
}

download() {
  local url="$1" out="$2"
  if [[ "$url" == file://* ]]; then
    local path="${url#file://}"
    cp "$path" "$out"
    return
  fi
  curl -fsSL --retry 3 -o "$out" "$url"
}

ARCHIVE_URL="$(resolve_archive_url)"
echo "Downloading $ARCHIVE_URL"
download "$ARCHIVE_URL" "$TMP/vcu.tgz"
# optional checksum
SHA_URL="${ARCHIVE_URL}.sha256"
if download "$SHA_URL" "$TMP/vcu.tgz.sha256" 2>/dev/null; then
  echo "Verifying checksum"
  # normalize to "hash  vcu.tgz" regardless of original filename in the sha file
  HASH="$(awk '{print $1}' "$TMP/vcu.tgz.sha256" | head -1)"
  echo "$HASH  vcu.tgz" > "$TMP/vcu.tgz.sha256"
  if command -v shasum >/dev/null 2>&1; then
    (cd "$TMP" && shasum -a 256 -c vcu.tgz.sha256)
  else
    (cd "$TMP" && sha256sum -c vcu.tgz.sha256)
  fi
else
  echo "WARN: no checksum file at $SHA_URL (continuing)"
fi

tar -xzf "$TMP/vcu.tgz" -C "$TMP"
STAGE="$(find "$TMP" -maxdepth 1 -type d -name 'vcu-*' | head -1)"
test -n "$STAGE"
test -x "$STAGE/bin/vcu"

mkdir -p "$VCU_BIN_DIR" "$VCU_SHARE_DIR"
install -m 755 "$STAGE/bin/vcu" "$VCU_BIN_DIR/vcu"
install -m 755 "$STAGE/bin/vcu-daemon" "$VCU_BIN_DIR/vcu-daemon"
install -m 755 "$STAGE/bin/vcu-mcp" "$VCU_BIN_DIR/vcu-mcp"
if [[ -x "$STAGE/bin/vcu-stage" ]]; then
  install -m 755 "$STAGE/bin/vcu-stage" "$VCU_BIN_DIR/vcu-stage"
fi
rm -rf "$VCU_SHARE_DIR/extension" "$VCU_SHARE_DIR/skills"
cp -R "$STAGE/extension" "$VCU_SHARE_DIR/" 2>/dev/null || true
cp -R "$STAGE/skills" "$VCU_SHARE_DIR/" 2>/dev/null || true
if [[ -d "$STAGE/playbooks" ]]; then
  mkdir -p "$VCU_SHARE_DIR/playbooks"
  cp -R "$STAGE/playbooks/." "$VCU_SHARE_DIR/playbooks/"
fi

# PATH hint
if ! command -v vcu >/dev/null 2>&1; then
  case ":$PATH:" in
    *":$VCU_BIN_DIR:"*) ;;
    *) echo "Add to PATH: export PATH=\"$VCU_BIN_DIR:\$PATH\"" ;;
  esac
fi

"$VCU_BIN_DIR/vcu" --version
"$VCU_BIN_DIR/vcu" init --json || true
echo "OK: vcu installed to $VCU_BIN_DIR"
echo "Extension bundle: $VCU_SHARE_DIR/extension"
if [[ -x "$VCU_BIN_DIR/vcu-stage" ]]; then
  echo "Stage helper: $VCU_BIN_DIR/vcu-stage"
fi
echo "Start daemon: vcu daemon start --foreground"
echo "MCP config: vcu mcp print-config --json"
