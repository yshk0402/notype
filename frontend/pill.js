import { getCurrentWindow, invoke, listen } from "./tauri-bridge.js";

const stateText = document.getElementById("stateText");
const stateDot = document.getElementById("stateDot");
const latencyHint = document.getElementById("latencyHint");
const openSettingsBtn = document.getElementById("openSettings");
const pill = document.getElementById("pill");

const currentWindow = getCurrentWindow();
const unlistenFns = [];
let isDraggingPill = false;

function normalizeError(err) {
  if (typeof err === "string") {
    return err;
  }
  if (err instanceof Error) {
    return err.message;
  }
  return String(err);
}

function setState(state) {
  const lower = String(state || "Idle").toLowerCase();
  stateText.textContent = state || "Idle";
  stateDot.className = `state-dot ${lower}`;
}

function setErrorState(message) {
  stateText.textContent = "Error";
  stateDot.className = "state-dot error";
  latencyHint.textContent = `${message} / Alt+X で再試行`;
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

async function checkRuntimeDependencies() {
  const missing = await invoke("check_runtime_dependencies");
  if (!Array.isArray(missing) || missing.length === 0) {
    return;
  }
  latencyHint.textContent = `missing: ${missing.join(", ")}`;
}

async function refreshState() {
  try {
    const next = await invoke("get_runtime_state");
    if (typeof next === "string") {
      setState(next);
    }
  } catch {
    // Realtime events are authoritative; ignore polling failures.
  }
}

async function onOpenSettingsClick() {
  try {
    await invoke("show_settings");
  } catch (e) {
    setErrorState(`settings open failed: ${normalizeError(e)}`);
  }
}

function onPillMouseDown(event) {
  if (event.target instanceof HTMLButtonElement) {
    return;
  }
  isDraggingPill = true;
  if (currentWindow?.startDragging) {
    currentWindow.startDragging().catch(() => {
      // Ignore drag failures.
    });
  }
}

async function onMouseUp() {
  if (!isDraggingPill) {
    return;
  }
  isDraggingPill = false;
  try {
    await persistPillPosition();
  } catch {
    // Ignore persist errors.
  }
}

async function subscribeEvents() {
  const transcriptUnlisten = await listen("notype://transcript", (event) => {
    const payload = event.payload;
    setState(payload.state);

    if (payload.state === "Ready") {
      latencyHint.textContent = payload.finalText
        ? "typed to focused app / Alt+X: start"
        : "no speech / Alt+X: retry";
      return;
    }

    if (payload.state === "Recording") {
      latencyHint.textContent = "recording... / Alt+X: stop";
      return;
    }

    if (payload.state === "Processing") {
      latencyHint.textContent = "processing...";
      return;
    }

    latencyHint.textContent = "Alt+X: start/stop";
  });

  const errorUnlisten = await listen("notype://error", (event) => {
    const payload = event.payload;
    console.error(payload.details);
    setErrorState(payload.userMessage);
  });

  const modelUnlisten = await listen("notype://model-download", (event) => {
    const payload = event.payload;
    latencyHint.textContent = `${payload.status} ${payload.progress}%`;
  });

  const dependencyUnlisten = await listen("notype://dependency-warning", (event) => {
    const payload = event.payload;
    latencyHint.textContent = `missing: ${payload.missing.join(", ")}`;
  });

  [transcriptUnlisten, errorUnlisten, modelUnlisten, dependencyUnlisten]
    .filter((fn) => typeof fn === "function")
    .forEach((fn) => unlistenFns.push(fn));
}

function destroy() {
  for (const unlisten of unlistenFns.splice(0)) {
    try {
      unlisten();
    } catch {
      // Ignore unlisten errors.
    }
  }
  openSettingsBtn.removeEventListener("click", onOpenSettingsClick);
  pill.removeEventListener("mousedown", onPillMouseDown);
  window.removeEventListener("mouseup", onMouseUp);
}

setState("Idle");
openSettingsBtn.addEventListener("click", onOpenSettingsClick);
pill.addEventListener("mousedown", onPillMouseDown);
window.addEventListener("mouseup", onMouseUp);
window.addEventListener("beforeunload", destroy);

subscribeEvents()
  .then(checkRuntimeDependencies)
  .catch((e) => {
    setErrorState(normalizeError(e));
  });

setInterval(refreshState, 1200);
