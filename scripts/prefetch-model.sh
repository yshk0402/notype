#!/usr/bin/env bash
set -euo pipefail

MODEL_DIR="${NOTYPE_MODEL_DIR:-$HOME/.cache/notype/models}"
MODEL_PATH="${MODEL_DIR}/ggml-small.bin"
MODEL_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"

mkdir -p "${MODEL_DIR}"

if [[ -f "${MODEL_PATH}" ]]; then
  echo "model already exists: ${MODEL_PATH}"
  exit 0
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "missing curl. install with: sudo apt-get install -y curl" >&2
  exit 1
fi

echo "downloading model to ${MODEL_PATH}"
curl -fL -C - --connect-timeout 20 --retry 3 --retry-delay 2 -o "${MODEL_PATH}.part" "${MODEL_URL}"
mv "${MODEL_PATH}.part" "${MODEL_PATH}"

echo "model ready: ${MODEL_PATH}"
