import { invoke } from "./tauri-bridge.js";

const statusText = document.getElementById("statusText");
const sectionTitle = document.getElementById("sectionTitle");
const navButtons = Array.from(document.querySelectorAll(".nav-btn"));
const panels = Array.from(document.querySelectorAll(".panel"));

const form = {
  maxRecord: document.getElementById("maxRecord"),
  model: document.getElementById("model"),
  autoType: document.getElementById("autoType"),
  textCleanup: document.getElementById("textCleanup"),
  realtimeEnabled: document.getElementById("realtimeEnabled"),
  partialMode: document.getElementById("partialMode"),
  llmEnabled: document.getElementById("llmEnabled"),
  llmProvider: document.getElementById("llmProvider"),
  llmModel: document.getElementById("llmModel"),
  llmApiBaseUrl: document.getElementById("llmApiBaseUrl"),
  llmApiKey: document.getElementById("llmApiKey")
};

const saveSettingsBtn = document.getElementById("saveSettings");
const reloadSettingsBtn = document.getElementById("reloadSettings");
const checkDepsBtn = document.getElementById("checkDeps");
const depsList = document.getElementById("depsList");

let currentConfig = null;

function switchPanel(id) {
  navButtons.forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.target === id);
  });

  panels.forEach((panel) => {
    panel.classList.toggle("active", panel.id === id);
  });

  sectionTitle.textContent = id[0].toUpperCase() + id.slice(1);
}

function applyConfig(cfg) {
  form.maxRecord.value = cfg.maxRecordSeconds;
  form.model.value = cfg.model;
  form.autoType.checked = cfg.autoType;
  form.textCleanup.checked = cfg.textCleanup;
  form.realtimeEnabled.checked = cfg.realtimeEnabled;
  form.partialMode.value = cfg.partialAutotypeMode;
  form.llmEnabled.checked = cfg.llmPostprocessEnabled;
  form.llmProvider.value = cfg.llmProvider || "";
  form.llmModel.value = cfg.llmModel || "";
  form.llmApiBaseUrl.value = cfg.llmApiBaseUrl || "";
  form.llmApiKey.value = cfg.llmApiKey || "";
}

function buildConfig() {
  return {
    ...currentConfig,
    maxRecordSeconds: Number(form.maxRecord.value || 60),
    model: form.model.value,
    autoType: form.autoType.checked,
    textCleanup: form.textCleanup.checked,
    realtimeEnabled: form.realtimeEnabled.checked,
    partialAutotypeMode: form.partialMode.value,
    llmPostprocessEnabled: form.llmEnabled.checked,
    llmProvider: form.llmProvider.value.trim(),
    llmModel: form.llmModel.value.trim(),
    llmApiBaseUrl: form.llmApiBaseUrl.value.trim(),
    llmApiKey: form.llmApiKey.value.trim()
  };
}

async function loadConfig() {
  const cfg = await invoke("get_config");
  currentConfig = cfg;
  applyConfig(cfg);
}

async function saveConfig() {
  const next = buildConfig();
  await invoke("update_config", { cfg: next });
  currentConfig = next;
}

async function checkDependencies() {
  const missing = await invoke("check_runtime_dependencies");
  depsList.innerHTML = "";

  if (!Array.isArray(missing) || missing.length === 0) {
    const li = document.createElement("li");
    li.textContent = "All runtime dependencies are available.";
    depsList.appendChild(li);
    return;
  }

  missing.forEach((name) => {
    const li = document.createElement("li");
    li.textContent = `missing: ${name}`;
    depsList.appendChild(li);
  });
}

navButtons.forEach((btn) => {
  btn.addEventListener("click", () => switchPanel(btn.dataset.target));
});

reloadSettingsBtn.addEventListener("click", async () => {
  try {
    await loadConfig();
    statusText.textContent = "loaded";
  } catch (e) {
    statusText.textContent = String(e);
  }
});

saveSettingsBtn.addEventListener("click", async () => {
  try {
    await saveConfig();
    statusText.textContent = "saved";
  } catch (e) {
    statusText.textContent = String(e);
  }
});

checkDepsBtn.addEventListener("click", async () => {
  try {
    await checkDependencies();
    statusText.textContent = "dependency check completed";
  } catch (e) {
    statusText.textContent = String(e);
  }
});

switchPanel("general");
loadConfig()
  .then(checkDependencies)
  .catch((e) => {
    statusText.textContent = String(e);
  });
