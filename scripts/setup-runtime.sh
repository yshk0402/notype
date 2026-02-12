#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "== notype runtime setup =="
echo "1) sudo apt-get update"
echo "2) sudo apt-get install -y wtype wl-clipboard cmake"
echo "3) ./scripts/install-whisper-cli-local.sh"
echo

if command -v sudo >/dev/null 2>&1; then
  echo "system packages (requires sudo password)..."
  sudo apt-get update
  sudo apt-get install -y wtype wl-clipboard cmake
fi

echo "installing whisper-cli to ~/.local/bin ..."
"${ROOT_DIR}/scripts/install-whisper-cli-local.sh"

echo
echo "verify:"
echo "which arecord wtype wl-copy whisper-cli"
