#!/usr/bin/env bash
set -euo pipefail

LOG_FILE="/tmp/notype-hotkey.log"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DBUS_DEST="dev.notype.app"
DBUS_PATH="/dev/notype/app"
DBUS_METHOD="dev.notype.app.ToggleRecording"

exec >>"${LOG_FILE}" 2>&1
printf '[%s] toggle request\n' "$(date '+%Y-%m-%d %H:%M:%S')"

if ! command -v gdbus >/dev/null 2>&1; then
  printf '[%s] gdbus command missing\n' "$(date '+%Y-%m-%d %H:%M:%S')"
  exit 1
fi

# Probe whether notype D-Bus service exists.
if gdbus introspect --session --dest "${DBUS_DEST}" --object-path "${DBUS_PATH}" >/dev/null 2>&1; then
  if gdbus call \
    --session \
    --timeout 3 \
    --dest "${DBUS_DEST}" \
    --object-path "${DBUS_PATH}" \
    --method "${DBUS_METHOD}" >/dev/null 2>&1; then
    printf '[%s] gdbus toggle ok\n' "$(date '+%Y-%m-%d %H:%M:%S')"
    exit 0
  fi

  # Important: if app is already running, never fallback-launch --toggle here.
  # Otherwise we create duplicate/looped toggle behavior while processing.
  printf '[%s] gdbus toggle failed while app is running; no fallback launch\n' "$(date '+%Y-%m-%d %H:%M:%S')"
  exit 1
fi

# App is not running; only this case may fallback-launch.
printf '[%s] dbus app missing, fallback to scripts/notype --toggle\n' "$(date '+%Y-%m-%d %H:%M:%S')"
if "${SCRIPT_DIR}/notype" --toggle; then
  printf '[%s] fallback toggle ok\n' "$(date '+%Y-%m-%d %H:%M:%S')"
  exit 0
fi

rc=$?
printf '[%s] fallback toggle failed rc=%s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "${rc}"
exit "${rc}"
