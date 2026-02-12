# notype

Local-first realtime dictation pill for Linux (Wayland-first).

## MVP highlights
- Floating always-on-top pill (settings + mic)
- Push-to-talk with realtime partial updates (300-700ms target)
- Local STT (whisper.cpp) with `small` default model
- Active app typing via `wtype`
- CLI contract
  - `notype`
  - `notype --settings`
  - `notype --toggle`
  - `notype --quit`

## Runtime dependencies
- `arecord` (ALSA)
- `wtype`
- `wl-copy`
- `whisper-cli` (from whisper.cpp)

Quick setup:
```bash
./scripts/setup-runtime.sh
```
`setup-runtime.sh` は `sudo` パスワード入力が必要です。

Verify dependencies:
```bash
which arecord wtype wl-copy whisper-cli
```
不足時の一括インストール例:
```bash
sudo apt-get install -y alsa-utils wtype wl-clipboard
./scripts/install-whisper-cli-local.sh
```

## UI behavior
- Pill has settings icon + recording status indicator.
- Transcript panel is not shown in main pill window.
- Recognized text is typed directly into the currently focused app.
- Recording starts/stops by `notype --toggle`.
- Recovery default is `final-only` (`realtimeEnabled=false`) for stability.

## First model download
- Default model is `small` (`ggml-small.bin`).
- On first transcription, if the model file does not exist, notype downloads it to:
  - `$NOTYPE_MODEL_DIR` if set
  - otherwise `~/.cache/notype/models`
- UI shows `notype://model-download` progress.

## Development
```bash
pnpm install
pnpm tauri dev
```

## Autostart setup
```bash
./scripts/install-autostart.sh
```

This installs these desktop entries:
- `~/.config/autostart/notype.desktop`
- `~/.config/autostart/notype-hotkey-sxhkd.desktop`

## Hotkey setup (non-GNOME recommended)
Install `sxhkd` first:
```bash
sudo apt-get install -y sxhkd
```

Install notype hotkey (`Alt+X`):
```bash
./scripts/install-hotkey-sxhkd.sh
```

Uninstall:
```bash
./scripts/uninstall-hotkey-sxhkd.sh
```

This writes a managed block to `~/.config/sxhkd/sxhkdrc` and logs toggle activity to:
- `/tmp/notype-hotkey.log`

## STT quick health check (recommended)
Before debugging app behavior, validate mic + whisper directly:
```bash
./scripts/prefetch-model.sh
./scripts/verify-stt.sh
```
This script records 3 seconds and runs `whisper-cli` with a 45s timeout.

## GNOME-only fallback
If you have full GNOME Shell environment:
```bash
./scripts/install-gnome-extension.sh
```

Legacy custom-keybinding path:
```bash
./scripts/install-gnome-shortcut.sh
```

## Quick troubleshooting
When hotkey does not work:
```bash
tail -n 80 /tmp/notype-hotkey.log
gdbus call --session --dest dev.notype.app --object-path /dev/notype/app --method dev.notype.app.ToggleRecording
journalctl --user -f | grep -E "notype|sxhkd"
```

When app is stuck on `Processing`:
```bash
./scripts/verify-stt.sh
```
If this fails, root cause is outside notype runtime loop (`whisper-cli` / model / CPU).

## Phase B: re-enable low latency partial
After final-only flow is stable, re-enable partial in config:
1. Start app with `NOTYPE_ALLOW_REALTIME=1` and set realtime on.
   - Example: `NOTYPE_ALLOW_REALTIME=1 pnpm tauri dev`
2. Verify with short 1-2s utterances first.

## Security and privacy
- Audio never leaves local machine by default.
- LLM postprocess is disabled by default and considered extension scope.
