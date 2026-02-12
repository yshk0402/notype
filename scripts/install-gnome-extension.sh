#!/usr/bin/env bash
set -euo pipefail

UUID="notype@dev.notype"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_DIR="${ROOT_DIR}/gnome-extension/${UUID}"
DEST_DIR="${HOME}/.local/share/gnome-shell/extensions/${UUID}"

if ! command -v gnome-shell >/dev/null 2>&1 || ! command -v gnome-extensions >/dev/null 2>&1; then
  echo "error: gnome-shell/gnome-extensions not found. this path is GNOME-only." >&2
  echo "use non-GNOME hotkey setup instead: ./scripts/install-hotkey-sxhkd.sh" >&2
  exit 1
fi

if [[ ! -d "${SRC_DIR}" ]]; then
  echo "error: extension source not found: ${SRC_DIR}" >&2
  exit 1
fi

if ! command -v glib-compile-schemas >/dev/null 2>&1; then
  echo "error: glib-compile-schemas is required" >&2
  echo "install example: sudo apt-get install -y libglib2.0-bin" >&2
  exit 1
fi

mkdir -p "$(dirname "${DEST_DIR}")"
rm -rf "${DEST_DIR}"
cp -R "${SRC_DIR}" "${DEST_DIR}"

glib-compile-schemas "${DEST_DIR}/schemas"

echo "installed extension to ${DEST_DIR}"
gnome-extensions enable "${UUID}" || true
echo "enabled extension: ${UUID}"
echo "current key: <Alt>x (change via dconf key /org/gnome/shell/extensions/notype/toggle-recording)"

echo "if hotkey is not active immediately, log out and log back in."
