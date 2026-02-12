# AGENTS.md - notype Implementation Guide

## 1. Purpose And Scope

### 1.1 Goal
- Deliver an always-available floating pill on Linux GNOME that supports:
  - Voice input
  - Local STT
  - Text injection into the currently focused app
- Keep interaction lightweight for short dictation workflows.

### 1.2 Non-Goals (MVP Exclusions)
- Long meeting transcription and speaker diarization
- Team sharing/collaboration workflows
- Advanced multi-language translation/rewriting
- Mobile integration

## 2. Product Principles
- Prioritize zero-cost operation for core features.
- Must work offline when LLM post-processing is disabled.
- Wayland-first behavior; Xorg is best effort.
- UI architecture is fixed:
  - Persistent floating pill window
  - Separate settings window

## 3. MVP Priority Order

### P0 (Must Build First)
- Recording start/stop
- Async STT execution
- Result display
- `Type` and `Copy` actions
- Auto-type toggle support
- Always-on-top floating pill
- Drag to move

### P1 (Core Operation Completeness)
- CLI behavior:
  - `notype`
  - `notype --settings`
  - `notype --quit`
- Single-instance behavior
- Login-time auto-start

### P2 (Post-MVP Expansion)
- LLM post-processing
- LLM provider extensions
- Input injection fallback expansion

## 4. Fixed Implementation Spec

### 4.1 Pill UI
- Pill contains exactly 2 icons:
  - Settings icon (open settings window)
  - Voice icon (start/stop recording)
- Must show state clearly with minimal visual cues.

### 4.2 Runtime States
- `Idle`
- `Recording`
- `Processing`
- `Ready`

### 4.3 MVP Settings
- `max_record_seconds`
- `model` (`small` or `medium`)
- `auto_type`
- `text_cleanup`

### 4.4 Text Injection
- Prioritize Wayland-compatible injection method.
- MVP primary path is `wtype`.

## 5. Resolved Defaults For Former TODOs
- Single-instance IPC: use `D-Bus`.
- Desktop entry (`.desktop`): provide it.
- Distribution strategy: AppImage first; deb/Flatpak later.
- Model delivery: download on first launch; default model is `medium`.
- Default close behavior: hide to tray/background (do not terminate).
- API key storage: GNOME Keyring (Secret Service).
- Injection fallback policy: `wtype` in MVP; `ydotool` is future extension.
- In-app global hotkey manager: out of MVP scope; rely on GNOME custom shortcut.

## 6. Contracts (Public Interfaces)

### 6.1 CLI Contract
- `notype`
  - If not running: start app.
  - If already running: bring main pill to front.
- `notype --settings`
  - If not running: start app and open settings.
  - If already running: bring settings window to front.
- `notype --quit`
  - Send termination request to running instance.

### 6.2 Single-Instance IPC Contract (D-Bus)
- `ShowMain`
- `ShowSettings`
- `Quit`

### 6.3 State Machine Contract
- Allowed values: `Idle | Recording | Processing | Ready`
- Any error path must transition back to `Idle`.

### 6.4 Minimal Settings Schema Contract
- `max_record_seconds: number`
- `model: "small" | "medium"`
- `auto_type: boolean`
- `text_cleanup: boolean`
- `llm_postprocess_enabled: boolean` (extension)
- `llm_provider: string` (extension)

## 7. Development Rules For Agents
- Never block UI thread:
  - Recording, STT, and post-processing run asynchronously.
- Do not send audio data externally.
  - If LLM post-processing is enabled, send text only.
- Always clean temp files on success and failure.
- On failure, recover to `Idle` and allow retry.
- Every change must map to acceptance criteria coverage.

## 8. Acceptance Criteria Checklist (MVP)
- [ ] AC-01: With cursor in VS Code, 10-second dictation and stop injects text when Auto-type is ON.
- [ ] AC-02: With Auto-type OFF, result appears in UI and `Type` injects text on demand.
- [ ] AC-03: UI remains responsive during recording and processing.
- [ ] AC-04: Window stays always-on-top and supports drag movement.
- [ ] AC-05: App can auto-start after login.
- [ ] AC-06: App works with network disabled when LLM is OFF.
- [ ] AC-07: `notype --settings` opens settings (or foregrounds existing one).
- [ ] AC-08: `notype --quit` terminates running instance safely.

## 9. Additional Failure-Path Test Scenarios
- [ ] STT failure shows clear error and returns to `Idle`.
- [ ] Input injection failure preserves result and supports `Copy` fallback.
- [ ] Crash during recording can recover cleanly on restart.
- [ ] Network-off behavior is stable and predictable.

## 10. Concrete Test Scenarios For Implementers
1. `notype` starts new instance when not running, foregrounds when already running.
2. `notype --settings` opens/foregrounds settings regardless of current state.
3. `notype --quit` shuts down safely and app can be relaunched.
4. 10-second record -> STT -> Auto-type ON injects into VS Code.
5. Auto-type OFF keeps result in UI until manual `Type`.
6. No UI hang during `Recording` and `Processing`.
7. With network disabled and LLM OFF, full flow still succeeds.
8. STT failure is recoverable and ends in `Idle`.
9. Injection failure still allows successful `Copy` usage.
10. After relogin, auto-start works and pill placement persists.

## 11. Future Extensions
- LLM post-processing rule templates
- Microphone device selection
- Pill opacity and always-on-top toggle
- Richer Xorg fallback injection options

## 12. Assumptions And Defaults
- Target environment is Linux GNOME, Wayland first.
- MVP must stay local-STT-centered and zero-cost for core flow.
- Unresolved TODOs are not deferred; this guide fixes defaults.
- This document is agent-first implementation guidance; human onboarding details stay minimal.
