#!/usr/bin/env bash
set -euo pipefail

UUID="notype@dev.notype"
DEST_DIR="${HOME}/.local/share/gnome-shell/extensions/${UUID}"

if command -v gnome-extensions >/dev/null 2>&1; then
  gnome-extensions disable "${UUID}" >/dev/null 2>&1 || true
fi

if [[ -d "${DEST_DIR}" ]]; then
  rm -rf "${DEST_DIR}"
  echo "removed ${DEST_DIR}"
else
  echo "extension directory not found: ${DEST_DIR}"
fi

echo "if extension still appears in UI, log out and log back in."
