#!/usr/bin/env bash
# Build platform archives under dist/ for curl/irm installers.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VERSION="${VCU_VERSION:-$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/')}"
TARGET_TRIPLE="${VCU_TARGET_TRIPLE:-$(rustc -vV | awk '/^host:/{print $2}')}"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64) ARCH_LABEL=x64 ;;
  arm64|aarch64) ARCH_LABEL=arm64 ;;
  *) ARCH_LABEL="$ARCH" ;;
esac
case "$OS" in
  darwin) PLAT="macos-${ARCH_LABEL}" ;;
  linux) PLAT="linux-${ARCH_LABEL}" ;;
  mingw*|msys*|cygwin*|windows*) PLAT="windows-${ARCH_LABEL}" ;;
  *) PLAT="${OS}-${ARCH_LABEL}" ;;
esac

echo "Building release for $PLAT (triple=$TARGET_TRIPLE version=$VERSION)"
cargo build --release -p vcu-cli -p vcu-daemon -p vcu-mcp
if [[ "$OS" == "darwin" ]]; then
  bash "$ROOT/scripts/build-stage.sh" "$ROOT/target/release/vcu-stage"
fi

STAGE="dist/stage/vcu-${VERSION}-${PLAT}"
rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/extension" "$STAGE/skills" "$STAGE/docs"

cp target/release/vcu "$STAGE/bin/" 2>/dev/null || cp target/release/vcu.exe "$STAGE/bin/"
cp target/release/vcu-daemon "$STAGE/bin/" 2>/dev/null || cp target/release/vcu-daemon.exe "$STAGE/bin/"
cp target/release/vcu-mcp "$STAGE/bin/" 2>/dev/null || cp target/release/vcu-mcp.exe "$STAGE/bin/"
if [[ -x "$ROOT/target/release/vcu-stage" ]]; then
  cp "$ROOT/target/release/vcu-stage" "$STAGE/bin/vcu-stage"
fi
chmod +x "$STAGE/bin/"* 2>/dev/null || true

# bundle extension + skills for offline install
cp -R extension/* "$STAGE/extension/" 2>/dev/null || true
cp -R skills/* "$STAGE/skills/" 2>/dev/null || true
mkdir -p "$STAGE/playbooks"
cp -R playbooks/* "$STAGE/playbooks/" 2>/dev/null || true
mkdir -p "$STAGE/scripts/macos" "$STAGE/scripts/install"
cp -R scripts/macos/* "$STAGE/scripts/macos/" 2>/dev/null || true
cp scripts/install/install.sh scripts/install/install.ps1 "$STAGE/scripts/install/" 2>/dev/null || true
cp docs/INSTALL.md "$STAGE/docs/" 2>/dev/null || true
cp README.md LICENSE "$STAGE/" 2>/dev/null || true

cat > "$STAGE/install-local.sh" <<'EOS'
#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${VCU_PREFIX:-$HOME/.local}"
BIN="$PREFIX/bin"
SHARE="$PREFIX/share/vcu"
mkdir -p "$BIN" "$SHARE"
install -m 755 "$HERE/bin/vcu" "$BIN/vcu"
install -m 755 "$HERE/bin/vcu-daemon" "$BIN/vcu-daemon"
install -m 755 "$HERE/bin/vcu-mcp" "$BIN/vcu-mcp"
if [[ -x "$HERE/bin/vcu-stage" ]]; then
  install -m 755 "$HERE/bin/vcu-stage" "$BIN/vcu-stage"
fi
rm -rf "$SHARE/extension" "$SHARE/skills"
mkdir -p "$SHARE"
cp -R "$HERE/extension" "$SHARE/" 2>/dev/null || true
cp -R "$HERE/skills" "$SHARE/" 2>/dev/null || true
echo "Installed vcu to $BIN"
"$BIN/vcu" --version
"$BIN/vcu" init --json || true
echo "Extension path: $SHARE/extension"
echo "Add to PATH if needed: export PATH=\"$BIN:\$PATH\""
EOS
chmod +x "$STAGE/install-local.sh"

ARCHIVE="dist/vcu-${VERSION}-${PLAT}.tar.gz"
mkdir -p dist
tar -C dist/stage -czf "$ARCHIVE" "vcu-${VERSION}-${PLAT}"
(
  cd dist
  shasum -a 256 "$(basename "$ARCHIVE")" > "$(basename "$ARCHIVE").sha256"
)
echo "Wrote $ARCHIVE"
cat "dist/$(basename "$ARCHIVE").sha256"
# also write latest pointer for this platform
echo "$VERSION $PLAT $(basename "$ARCHIVE")" > "dist/latest-${PLAT}.txt"
cp "$ARCHIVE" "dist/vcu-latest-${PLAT}.tar.gz"
cp "dist/$(basename "$ARCHIVE").sha256" "dist/vcu-latest-${PLAT}.tar.gz.sha256"
echo "Also: dist/vcu-latest-${PLAT}.tar.gz"
cp scripts/install/install.sh dist/install.sh
cp scripts/install/install.ps1 dist/install.ps1 2>/dev/null || true
echo "Installers: dist/install.sh dist/install.ps1"
