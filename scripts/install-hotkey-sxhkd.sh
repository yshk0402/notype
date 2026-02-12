#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SXHKD_DIR="${HOME}/.config/sxhkd"
SXHKD_RC="${SXHKD_DIR}/sxhkdrc"
TOGGLE_SCRIPT="${ROOT_DIR}/scripts/notype-toggle-dbus.sh"
START_MARK="# >>> notype hotkey >>>"
END_MARK="# <<< notype hotkey <<<"
BLOCK="${START_MARK}
alt + x
    ${TOGGLE_SCRIPT}
${END_MARK}"

if ! command -v sxhkd >/dev/null 2>&1; then
  echo "error: sxhkd is required" >&2
  echo "install example: sudo apt-get install -y sxhkd" >&2
  exit 1
fi

if [[ ! -x "${TOGGLE_SCRIPT}" ]]; then
  chmod +x "${TOGGLE_SCRIPT}"
fi

mkdir -p "${SXHKD_DIR}"
if [[ ! -f "${SXHKD_RC}" ]]; then
  touch "${SXHKD_RC}"
fi

# Remove old managed block if present.
awk -v start="${START_MARK}" -v end="${END_MARK}" '
$0==start {skip=1; next}
$0==end {skip=0; next}
!skip {print}
' "${SXHKD_RC}" > "${SXHKD_RC}.tmp"
mv "${SXHKD_RC}.tmp" "${SXHKD_RC}"

if [[ -s "${SXHKD_RC}" ]]; then
  printf '\n%s\n' "${BLOCK}" >> "${SXHKD_RC}"
else
  printf '%s\n' "${BLOCK}" >> "${SXHKD_RC}"
fi

echo "installed sxhkd hotkey: Alt+X -> ${TOGGLE_SCRIPT}"

if pgrep -x sxhkd >/dev/null 2>&1; then
  pkill -USR1 -x sxhkd || true
  echo "reloaded running sxhkd"
else
  nohup sxhkd -c "${SXHKD_RC}" >/tmp/notype-sxhkd.log 2>&1 &
  echo "started sxhkd (log: /tmp/notype-sxhkd.log)"
fi
