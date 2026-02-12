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

function resolveListen() {
  const fromEvent = window.__TAURI__?.event?.listen;
  if (typeof fromEvent === "function") {
    return fromEvent;
  }

  const fromInternals = window.__TAURI_INTERNALS__?.event?.listen;
  if (typeof fromInternals === "function") {
    return fromInternals;
  }

  return null;
}

function resolveCurrentWindow() {
  const fromWindowApi = window.__TAURI__?.window?.getCurrentWindow?.();
  if (fromWindowApi) {
    return fromWindowApi;
  }

  return window.__TAURI_INTERNALS__?.window;
}

export async function invoke(cmd, args) {
  const invokeFn = resolveInvoke();
  if (typeof invokeFn !== "function") {
    throw new Error("Tauri invoke API is unavailable");
  }
  return invokeFn(cmd, args);
}

export function listen(eventName, handler) {
  const listenFn = resolveListen();
  if (typeof listenFn !== "function") {
    return null;
  }
  return listenFn(eventName, handler);
}

export function getCurrentWindow() {
  return resolveCurrentWindow();
}
