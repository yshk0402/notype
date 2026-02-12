#!/usr/bin/env bash
set -euo pipefail

MODEL_DIR="${NOTYPE_MODEL_DIR:-$HOME/.cache/notype/models}"
MODEL_PATH="${MODEL_DIR}/ggml-small.bin"
WAV_PATH="/tmp/notype-stt-check.wav"
OUT_TXT="${WAV_PATH}.txt"

mkdir -p "${MODEL_DIR}"

echo "[1/4] checking dependencies"
for cmd in arecord whisper-cli timeout; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "missing command: $cmd" >&2
    exit 1
  fi
done

echo "[2/4] checking model: ${MODEL_PATH}"
if [[ ! -f "${MODEL_PATH}" ]]; then
  echo "model missing: ${MODEL_PATH}" >&2
  echo "hint: set NOTYPE_MODEL_DIR or run app once to download model" >&2
  exit 1
fi

echo "[3/4] recording 3 sec sample to ${WAV_PATH}"
rm -f "${WAV_PATH}" "${OUT_TXT}"
arecord -q -f S16_LE -r 16000 -c1 -d 3 "${WAV_PATH}"

echo "[4/4] running whisper-cli (timeout 45s)"
timeout 45s whisper-cli -m "${MODEL_PATH}" -f "${WAV_PATH}" -otxt -nt -l ja >/tmp/notype-stt-check.stdout 2>/tmp/notype-stt-check.stderr || {
  rc=$?
  echo "whisper-cli failed rc=${rc}" >&2
  echo "stderr:" >&2
  cat /tmp/notype-stt-check.stderr >&2
  exit $rc
}

if [[ -f "${OUT_TXT}" ]]; then
  echo "transcript file: ${OUT_TXT}"
  cat "${OUT_TXT}"
else
  echo "warning: transcript file not generated; stdout follows"
  cat /tmp/notype-stt-check.stdout
fi
