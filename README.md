# notype

Local-first realtime dictation pill for Linux GNOME (Wayland-first).

## MVP highlights
- Floating always-on-top pill (settings + mic)
- Push-to-talk with realtime partial updates (300-700ms target)
- Local STT (whisper.cpp) with `small` default model
- Active app typing via `wtype`
- CLI contract
  - `notype`
  - `notype --settings`
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

## UI behavior
- Pill has only 2 icons (settings / mic).
- Transcript panel is not shown in main pill window.
- Recognized text is typed directly into the currently focused app.

## First model download
- Default model is `small` (`ggml-small.bin`).
- On first transcription, if the model file does not exist, notype downloads it to:
  - `$NOTYPE_MODEL_DIR` if set
  - otherwise `/tmp/notype-models`
- UI shows `notype://model-download` progress.

## Development
```bash
pnpm install
pnpm tauri dev
```

## Autostart setup (GNOME)
```bash
./scripts/install-autostart.sh
```

This installs `desktop/notype-autostart.desktop` to:
- `~/.config/autostart/notype.desktop`

## Security and privacy
- Audio never leaves local machine by default.
- LLM postprocess is disabled by default and considered extension scope.
