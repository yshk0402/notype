#!/usr/bin/env bash
set -euo pipefail

mkdir -p "$HOME/.config/autostart"
cp "$(dirname "$0")/../desktop/notype-autostart.desktop" "$HOME/.config/autostart/notype.desktop"
echo "installed: $HOME/.config/autostart/notype.desktop"
