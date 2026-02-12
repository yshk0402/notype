import { getCurrentWindow, invoke, listen } from "./tauri-bridge.js";

const stateText = document.getElementById("stateText");
const stateDot = document.getElementById("stateDot");
const latencyHint = document.getElementById("latencyHint");
const toggleRecordBtn = document.getElementById("toggleRecord");
const openSettingsBtn = document.getElementById("openSettings");
const pill = document.getElementById("pill");

const currentWindow = getCurrentWindow();
let runtimeState = "Idle";
let isDraggingPill = false;

function setState(state) {
  runtimeState = state;
  stateText.textContent = state;
  stateDot.className = `state-dot ${state.toLowerCase()}`;
  toggleRecordBtn.classList.toggle("recording", state === "Recording");
}

async function persistPillPosition() {
  if (!currentWindow?.outerPosition) {
    return;
  }
  const pos = await currentWindow.outerPosition();
  if (typeof pos?.x !== "number" || typeof pos?.y !== "number") {
    return;
  }
  await invoke("set_pill_position", { position: { x: pos.x, y: pos.y } });
}

async function startOrStopRecording() {
  if (runtimeState === "Recording") {
    setState("Processing");
    const text = await invoke("stop_recording");
    setState("Ready");
    latencyHint.textContent = text ? "transcribed" : "no speech";
    return;
  }

  await invoke("start_recording");
  setState("Recording");
  latencyHint.textContent = "recording...";
}

async function checkRuntimeDependencies() {
  const missing = await invoke("check_runtime_dependencies");
  if (!Array.isArray(missing) || missing.length === 0) {
    toggleRecordBtn.disabled = false;
    return;
  }

  toggleRecordBtn.disabled = true;
  latencyHint.textContent = `missing: ${missing.join(", ")}`;
}

async function refreshState() {
  try {
    const next = await invoke("get_runtime_state");
    if (typeof next === "string") {
      setState(next);
    }
  } catch {
    // Fallback to optimistic UI state when command is unavailable.
  }
}

toggleRecordBtn.addEventListener("click", async () => {
  try {
    await startOrStopRecording();
  } catch (e) {
    latencyHint.textContent = String(e);
    setState("Idle");
  }
});

openSettingsBtn.addEventListener("click", async () => {
  try {
    await invoke("show_settings");
  } catch (e) {
    latencyHint.textContent = `settings open failed: ${String(e)}`;
  }
});

pill.addEventListener("mousedown", async (event) => {
  if (event.target instanceof HTMLButtonElement) {
    return;
  }
  isDraggingPill = true;
  if (currentWindow?.startDragging) {
    try {
      await currentWindow.startDragging();
    } catch {
      // Ignore drag failures on unsupported environments.
    }
  }
});

window.addEventListener("mouseup", async () => {
  if (!isDraggingPill) {
    return;
  }
  isDraggingPill = false;
  try {
    await persistPillPosition();
  } catch {
    // Ignore persisting errors to keep interaction smooth.
  }
});

listen("notype://transcript", (event) => {
  const payload = event.payload;
  setState(payload.state);

  if (payload.latencyMs) {
    latencyHint.textContent = `latency ${payload.latencyMs}ms`;
  } else if (payload.state === "Ready") {
    latencyHint.textContent = "typed to focused app";
  } else if (payload.state === "Recording") {
    latencyHint.textContent = "recording...";
  }
});

listen("notype://error", (event) => {
  const payload = event.payload;
  latencyHint.textContent = payload.userMessage;
  setState("Idle");
  console.error(payload.details);
});

listen("notype://model-download", (event) => {
  const payload = event.payload;
  latencyHint.textContent = `${payload.status} ${payload.progress}%`;
});

setState("Idle");
checkRuntimeDependencies();
setInterval(refreshState, 1200);
