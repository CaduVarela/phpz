#!/bin/sh
# Installs phpz: curl -fsSL https://raw.githubusercontent.com/CaduVarela/phpz/master/install.sh | sh
set -eu

REPOSITORY="CaduVarela/phpz"
INSTALL_DIRECTORY="${PHPZ_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${PHPZ_VERSION:-latest}"

if [ -n "${PHPZ_RELEASE_URL:-}" ]; then
    BASE_URL="$PHPZ_RELEASE_URL"
elif [ "$VERSION" = "latest" ]; then
    BASE_URL="https://github.com/$REPOSITORY/releases/latest/download"
else
    BASE_URL="https://github.com/$REPOSITORY/releases/download/$VERSION"
fi

fail() {
    echo "phpz install: $1" >&2
    exit 1
}

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) ASSET="phpz-linux-x64" ;;
    Linux-aarch64 | Linux-arm64) ASSET="phpz-linux-arm64" ;;
    Darwin-x86_64) ASSET="phpz-macos-x64" ;;
    Darwin-arm64) ASSET="phpz-macos-arm64" ;;
    *) fail "unsupported system: $(uname -s) $(uname -m)" ;;
esac

command -v curl >/dev/null 2>&1 || fail "curl is required"

if command -v sha256sum >/dev/null 2>&1; then
    checksum() { sha256sum "$1" | cut -d' ' -f1; }
elif command -v shasum >/dev/null 2>&1; then
    checksum() { shasum -a 256 "$1" | cut -d' ' -f1; }
else
    fail "sha256sum or shasum is required to verify the download"
fi

WORK_DIRECTORY="$(mktemp -d)"
trap 'rm -rf "$WORK_DIRECTORY"' EXIT

echo "Downloading $ASSET"
curl -fsSL "$BASE_URL/$ASSET" -o "$WORK_DIRECTORY/$ASSET" || fail "could not download $BASE_URL/$ASSET"
curl -fsSL "$BASE_URL/SHA256SUMS" -o "$WORK_DIRECTORY/SHA256SUMS" || fail "could not download the checksum list"

EXPECTED="$(grep " $ASSET\$" "$WORK_DIRECTORY/SHA256SUMS" | cut -d' ' -f1)"
[ -n "$EXPECTED" ] || fail "no checksum is listed for $ASSET"
[ "$(checksum "$WORK_DIRECTORY/$ASSET")" = "$EXPECTED" ] || fail "the checksum of the download does not match, nothing was installed"
echo "Verified SHA-256 checksum"

mkdir -p "$INSTALL_DIRECTORY"
mv "$WORK_DIRECTORY/$ASSET" "$INSTALL_DIRECTORY/phpz"
chmod +x "$INSTALL_DIRECTORY/phpz"
echo "Installed $INSTALL_DIRECTORY/phpz"

case ":$PATH:" in
    *":$INSTALL_DIRECTORY:"*) ;;
    *)
        case "${SHELL:-}" in
            */zsh) PROFILE="$HOME/.zshrc" ;;
            *) PROFILE="$HOME/.bashrc" ;;
        esac
        if ! grep -qs "$INSTALL_DIRECTORY" "$PROFILE"; then
            printf '\nexport PATH="%s:$PATH"\n' "$INSTALL_DIRECTORY" >> "$PROFILE"
            echo "Added $INSTALL_DIRECTORY to $PROFILE"
        fi
        ;;
esac

"$INSTALL_DIRECTORY/phpz" setup
echo "Done. Open a new terminal and run: phpz install 8.3"
