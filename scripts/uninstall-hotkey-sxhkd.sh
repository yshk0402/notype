#!/usr/bin/env bash
set -euo pipefail

SXHKD_RC="${HOME}/.config/sxhkd/sxhkdrc"
START_MARK="# >>> notype hotkey >>>"
END_MARK="# <<< notype hotkey <<<"

if [[ -f "${SXHKD_RC}" ]]; then
  awk -v start="${START_MARK}" -v end="${END_MARK}" '
$0==start {skip=1; next}
$0==end {skip=0; next}
!skip {print}
' "${SXHKD_RC}" > "${SXHKD_RC}.tmp"
  mv "${SXHKD_RC}.tmp" "${SXHKD_RC}"
  echo "removed notype block from ${SXHKD_RC}"
else
  echo "sxhkd config not found: ${SXHKD_RC}"
fi

if pgrep -x sxhkd >/dev/null 2>&1; then
  pkill -USR1 -x sxhkd || true
  echo "reloaded running sxhkd"
fi
