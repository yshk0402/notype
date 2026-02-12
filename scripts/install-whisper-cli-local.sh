#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT_DIR}/.tmp/whisper.cpp"
INSTALL_DIR="${HOME}/.local/bin"

if ! command -v git >/dev/null 2>&1; then
  echo "git が必要です。先にインストールしてください。"
  exit 1
fi

if ! command -v cmake >/dev/null 2>&1; then
  echo "cmake が必要です。先に 'sudo apt-get install -y cmake' を実行してください。"
  exit 1
fi

mkdir -p "${ROOT_DIR}/.tmp"
rm -rf "${BUILD_DIR}"
git clone --depth 1 https://github.com/ggml-org/whisper.cpp.git "${BUILD_DIR}"

if command -v cmake >/dev/null 2>&1; then
  cmake -S "${BUILD_DIR}" -B "${BUILD_DIR}/build" -DWHISPER_BUILD_TESTS=OFF
  cmake --build "${BUILD_DIR}/build" -j"$(nproc)"
  BIN_PATH="${BUILD_DIR}/build/bin/whisper-cli"
fi

if [[ ! -x "${BIN_PATH}" ]]; then
  echo "whisper-cli のビルドに失敗しました。"
  exit 1
fi

mkdir -p "${INSTALL_DIR}"
cp "${BIN_PATH}" "${INSTALL_DIR}/whisper-cli"
chmod +x "${INSTALL_DIR}/whisper-cli"

if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
  echo "PATH に ${INSTALL_DIR} が含まれていません。"
  echo "次を ~/.bashrc に追加してください:"
  echo "export PATH=\"${INSTALL_DIR}:\$PATH\""
fi

echo "installed: ${INSTALL_DIR}/whisper-cli"
