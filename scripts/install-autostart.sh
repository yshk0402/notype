#!/usr/bin/env bash
set -euo pipefail

mkdir -p "$HOME/.config/autostart"
cp "$(dirname "$0")/../desktop/notype-autostart.desktop" "$HOME/.config/autostart/notype.desktop"
echo "installed: $HOME/.config/autostart/notype.desktop"

if command -v sxhkd >/dev/null 2>&1 && [[ -f "$(dirname "$0")/../desktop/notype-hotkey-sxhkd.desktop" ]]; then
  cp "$(dirname "$0")/../desktop/notype-hotkey-sxhkd.desktop" \
    "$HOME/.config/autostart/notype-hotkey-sxhkd.desktop"
  echo "installed: $HOME/.config/autostart/notype-hotkey-sxhkd.desktop"
else
  echo "skip sxhkd autostart (sxhkd not installed). install with: sudo apt-get install -y sxhkd"
fi
