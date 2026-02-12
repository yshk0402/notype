function resolveInvoke() {
  const fromCore = window.__TAURI__?.core?.invoke;
  if (typeof fromCore === "function") {
    return fromCore;
  }

  const fromInternals = window.__TAURI_INTERNALS__?.invoke;
  if (typeof fromInternals === "function") {
    return fromInternals;
  }

  return null;
}

const invoke = async (cmd, args) => {
  const invokeFn = resolveInvoke();
  if (typeof invokeFn !== "function") {
    throw new Error("Tauri invoke API is unavailable");
  }
  return invokeFn(cmd, args);
};

const listen = window.__TAURI__?.event?.listen;

const stateText = document.getElementById("stateText");
const stateDot = document.getElementById("stateDot");
const latencyHint = document.getElementById("latencyHint");

const toggleRecordBtn = document.getElementById("toggleRecord");
const openSettingsBtn = document.getElementById("openSettings");
const saveSettingsBtn = document.getElementById("saveSettings");
const pill = document.getElementById("pill");
const panel = document.querySelector(".panel");
const settingsPane = document.getElementById("settingsPane");
const currentWindow = window.__TAURI__?.window?.getCurrentWindow?.();
const windowLabel = currentWindow?.label ?? "main";
const isSettingsWindow = windowLabel === "settings";

let runtimeState = "Idle";
let isDraggingPill = false;

function setState(state) {
  runtimeState = state;
  stateText.textContent = state;
  stateDot.className = `state-dot ${state.toLowerCase()}`;
}

async function loadConfig() {
  const cfg = await invoke("get_config");
  document.getElementById("maxRecord").value = cfg.maxRecordSeconds;
  document.getElementById("model").value = cfg.model;
  document.getElementById("textCleanup").checked = cfg.textCleanup;
  document.getElementById("llmEnabled").checked = cfg.llmPostprocessEnabled;
  document.getElementById("llmProvider").value = cfg.llmProvider;
  return cfg;
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

  const msg = `missing: ${missing.join(", ")}`;
  latencyHint.textContent = msg;
  stateText.textContent = "Missing deps";
  toggleRecordBtn.disabled = true;
}

async function saveConfig() {
  const cfg = {
    maxRecordSeconds: Number(document.getElementById("maxRecord").value || 60),
    model: document.getElementById("model").value,
    autoType: true,
    textCleanup: document.getElementById("textCleanup").checked,
    llmPostprocessEnabled: false,
    llmProvider: "",
    realtimeEnabled: true,
    partialAutotypeMode: "replace"
  };
  await invoke("update_config", { cfg });
}

async function ensureAutoTypeEnabled(cfg) {
  if (isSettingsWindow || cfg.autoType) {
    return;
  }
  const next = { ...cfg, autoType: true };
  await invoke("update_config", { cfg: next });
  latencyHint.textContent = "auto_type was OFF, enabled for direct input";
}

async function startOrStopRecording() {
  if (runtimeState === "Recording") {
    await invoke("stop_recording");
    return;
  }
  await invoke("start_recording");
}

toggleRecordBtn.addEventListener("click", async () => {
  try {
    await startOrStopRecording();
  } catch (e) {
    latencyHint.textContent = String(e);
  }
});

openSettingsBtn.addEventListener("click", async () => {
  try {
    await invoke("show_settings");
  } catch (e) {
    latencyHint.textContent = `settings open failed: ${String(e)}`;
  }
});

saveSettingsBtn.addEventListener("click", async () => {
  await saveConfig();
  latencyHint.textContent = "saved";
});

if (listen) {
  listen("notype://transcript", (event) => {
    const payload = event.payload;
    setState(payload.state);

    if (payload.latencyMs) {
      latencyHint.textContent = `latency ${payload.latencyMs}ms`;
    } else if (payload.state === "Ready") {
      latencyHint.textContent = "typed to focused app";
    }
  });

  listen("notype://error", (event) => {
    const payload = event.payload;
    latencyHint.textContent = `${payload.userMessage} / Copy で代替できます`;
    console.error(payload.details);
    setState("Idle");
  });

  listen("notype://model-download", (event) => {
    const payload = event.payload;
    latencyHint.textContent = `${payload.status} ${payload.progress}%: ${payload.message}`;
  });
}

setState("Idle");
if (isSettingsWindow) {
  if (pill) pill.style.display = "none";
} else {
  if (panel) panel.style.display = "none";
  if (settingsPane) settingsPane.style.display = "none";
}

loadConfig()
  .then((cfg) => {
    if (!isSettingsWindow) {
      return ensureAutoTypeEnabled(cfg).then(checkRuntimeDependencies);
    }
    return undefined;
  })
  .catch((e) => {
    latencyHint.textContent = String(e);
  });

pill.addEventListener("mousedown", async (event) => {
  if (isSettingsWindow) {
    return;
  }
  if (event.target instanceof HTMLButtonElement) {
    return;
  }
  isDraggingPill = true;
  const currentWindow = window.__TAURI__?.window?.getCurrentWindow?.();
  if (currentWindow?.startDragging) {
    try {
      await currentWindow.startDragging();
    } catch {
      // Ignore drag failures on unsupported environments.
    }
  }
});

window.addEventListener("mouseup", async () => {
  if (isSettingsWindow) {
    return;
  }
  if (!isDraggingPill) {
    return;
  }
  isDraggingPill = false;
  try {
    await persistPillPosition();
  } catch (e) {
    console.error("failed to persist pill position", e);
  }
});
