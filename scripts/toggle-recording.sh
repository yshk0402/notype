#!/usr/bin/env bash
set -euo pipefail

LOG_FILE="/tmp/notype-toggle.log"
exec >>"$LOG_FILE" 2>&1
echo "[$(date '+%Y-%m-%d %H:%M:%S')] toggle invoked"
trap 'rc=$?; echo "[$(date "+%Y-%m-%d %H:%M:%S")] exit=${rc}"' EXIT

# First try existing running instance over D-Bus.
if command -v gdbus >/dev/null 2>&1; then
  if gdbus call \
    --session \
    --dest dev.notype.app \
    --object-path /dev/notype/app \
    --method dev.notype.app.ToggleRecording >/dev/null 2>&1; then
    echo "dbus toggle succeeded"
    exit 0
  fi
fi

# Fallback: launch local wrapper if available.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -x "${SCRIPT_DIR}/notype" ]]; then
  echo "dbus target missing, fallback launching scripts/notype --toggle"
  exec "${SCRIPT_DIR}/notype" --toggle
fi

echo "failed: no running notype instance and local launcher not found" >&2
exit 1
