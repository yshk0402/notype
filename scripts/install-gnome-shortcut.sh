#!/usr/bin/env bash
set -euo pipefail

echo "warning: this script is legacy."
echo "recommended: ./scripts/install-gnome-extension.sh"
echo "non-GNOME environment: ./scripts/install-hotkey-sxhkd.sh"
echo

if ! command -v gnome-shell >/dev/null 2>&1; then
  echo "error: gnome-shell is not installed. use ./scripts/install-hotkey-sxhkd.sh" >&2
  exit 1
fi

SCHEMA="org.gnome.settings-daemon.plugins.media-keys"
BASE="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings"
NAME="notype-toggle"
BINDING="<Alt>x"
ALT_CANDIDATES=("<Super>semicolon" "<Alt>z")

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APPIMAGE_PATH="${HOME}/Applications/notype.AppImage"
RELEASE_BIN="${ROOT_DIR}/src-tauri/target/release/notype"
DEBUG_BIN="${ROOT_DIR}/src-tauri/target/debug/notype"

if [[ -x "${APPIMAGE_PATH}" ]]; then
  COMMAND="${APPIMAGE_PATH} --toggle"
elif [[ -x "${RELEASE_BIN}" ]]; then
  COMMAND="${RELEASE_BIN} --toggle"
elif [[ -x "${DEBUG_BIN}" ]]; then
  COMMAND="${DEBUG_BIN} --toggle"
else
  echo "error: no executable notype binary found"
  echo "checked:"
  echo "  - ${APPIMAGE_PATH}"
  echo "  - ${RELEASE_BIN}"
  echo "  - ${DEBUG_BIN}"
  echo "build one of:"
  echo "  cargo build --manifest-path src-tauri/Cargo.toml --release"
  echo "  cargo build --manifest-path src-tauri/Cargo.toml"
  exit 1
fi

check_conflicts() {
  local pattern="$1"
  local found=""

  found+="$(gsettings list-recursively org.gnome.desktop.wm.keybindings | grep -F "'${pattern}'" || true)"
  found+=$'\n'
  found+="$(gsettings list-recursively org.gnome.settings-daemon.plugins.media-keys | grep -F "'${pattern}'" || true)"

  local filtered
  filtered="$(printf "%s\n" "$found" | grep -v "notype-toggle" | sed '/^$/d' || true)"
  if [[ -n "$filtered" ]]; then
    echo "conflict detected for ${pattern}:"
    printf "%s\n" "$filtered"
    echo "try one of alternative bindings: ${ALT_CANDIDATES[*]}"
    return 1
  fi
  return 0
}

if ! check_conflicts "$BINDING"; then
  exit 1
fi

current_raw="$(gsettings get ${SCHEMA} custom-keybindings)"
paths=()
while IFS= read -r path; do
  [[ -n "$path" ]] && paths+=("$path")
done < <(printf "%s" "$current_raw" | grep -o "${BASE}/custom[0-9]\+/" || true)

for path in "${paths[@]}"; do
  key="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:${path}"
  key_name="$(gsettings get "$key" name | tr -d "'")"

  if [[ "$key_name" == "$NAME" ]]; then
    gsettings set "$key" command "'${COMMAND}'"
    gsettings set "$key" binding "'${BINDING}'"
    echo "updated existing shortcut: ${NAME} -> ${BINDING}"
    echo "command: ${COMMAND}"
    exit 0
  fi
done

next_idx=0
if ((${#paths[@]} > 0)); then
  while [[ " ${paths[*]} " == *" ${BASE}/custom${next_idx}/ "* ]]; do
    next_idx=$((next_idx + 1))
  done
fi

new_path="${BASE}/custom${next_idx}/"
key="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:${new_path}"

updated_paths=("${paths[@]}" "$new_path")
list="["
for idx in "${!updated_paths[@]}"; do
  list+="'${updated_paths[$idx]}'"
  if [[ "$idx" -lt $((${#updated_paths[@]} - 1)) ]]; then
    list+=", "
  fi
done
list+="]"

gsettings set ${SCHEMA} custom-keybindings "$list"
gsettings set "$key" name "'${NAME}'"
gsettings set "$key" command "'${COMMAND}'"
gsettings set "$key" binding "'${BINDING}'"

echo "installed GNOME shortcut: ${NAME} -> ${BINDING}"
echo "command: ${COMMAND}"
