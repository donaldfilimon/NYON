const ARTIFACTS = Object.freeze({
  webgpu: Object.freeze({
    key: "webgpu",
    label: "WEBGPU",
    moduleUrl: new URL("../dist/webgpu/nyon.js", import.meta.url).href,
  }),
  webgl2: Object.freeze({
    key: "webgl2",
    label: "WEBGL2 LOW",
    moduleUrl: new URL("../dist/webgl/nyon.js", import.meta.url).href,
  }),
});

export const GRAPHICS_READY_EVENT = "nyon:graphics-ready";
export const GRAPHICS_FAILED_EVENT = "nyon:graphics-failed";
const GRAPHICS_READY_TIMEOUT_MS = 20_000;

export class NyonStartupError extends Error {
  constructor(message, attempts, cause) {
    super(message, cause === undefined ? undefined : { cause });
    this.name = "NyonStartupError";
    this.attempts = Object.freeze([...attempts]);
  }
}

export function forcedBackend(search) {
  const params = new URLSearchParams(search ?? "");
  return params.get("backend") === "webgl2" ? "webgl2" : null;
}

function safeDetail(value) {
  const text = value instanceof Error ? value.message : String(value ?? "unknown failure");
  return text.replace(/[\u0000-\u001f\u007f]+/gu, " ").slice(0, 320);
}

function adapterDetail(adapter) {
  const info = adapter?.info;
  if (info === undefined || info === null) {
    return "WebGPU adapter available";
  }
  const parts = [info.vendor, info.architecture, info.device, info.description]
    .filter((part) => typeof part === "string" && part.length > 0);
  return parts.length > 0 ? parts.join(" / ").slice(0, 320) : "WebGPU adapter available";
}

export async function preflightWebGpu(navigatorObject) {
  if (navigatorObject?.gpu === undefined) {
    return Object.freeze({ ok: false, detail: "navigator.gpu is unavailable" });
  }

  try {
    const adapter = await navigatorObject.gpu.requestAdapter({
      powerPreference: "high-performance",
    });
    if (adapter === null) {
      return Object.freeze({ ok: false, detail: "navigator.gpu returned no adapter" });
    }
    return Object.freeze({ ok: true, detail: adapterDetail(adapter) });
  } catch (error) {
    return Object.freeze({
      ok: false,
      detail: `WebGPU adapter request failed: ${safeDetail(error)}`,
    });
  }
}

export function teardownIncompleteSurface(documentObject) {
  if (documentObject === undefined || documentObject === null) {
    return;
  }
  for (const canvas of documentObject.querySelectorAll("canvas")) {
    canvas.remove();
  }
}

export function normalizeGraphicsEventDetail(detail) {
  if (typeof detail === "string") {
    return Object.freeze({ backend: undefined, message: detail });
  }
  if (detail !== null && typeof detail === "object") {
    return Object.freeze({
      backend: typeof detail.backend === "string" ? detail.backend : undefined,
      message: typeof detail.message === "string" ? detail.message : undefined,
    });
  }
  return Object.freeze({ backend: undefined, message: undefined });
}

function renderStatus(documentObject, state, backend, message) {
  const status = documentObject?.getElementById("nyon-status");
  const backendText = documentObject?.getElementById("nyon-backend");
  const messageText = documentObject?.getElementById("nyon-message");
  if (status === null || status === undefined || backendText === null ||
      backendText === undefined || messageText === null || messageText === undefined) {
    return;
  }

  status.dataset.state = state;
  status.setAttribute("role", state === "error" ? "alert" : "status");
  backendText.textContent = backend;
  messageText.textContent = message;
}

function observeGraphicsInitialization(windowObject, expectedBackend, timeoutMs) {
  let timeout;
  let settled = false;
  let resolveSignal;
  let rejectSignal;

  const cleanup = () => {
    if (timeout !== undefined) {
      windowObject.clearTimeout(timeout);
    }
    windowObject.removeEventListener(GRAPHICS_READY_EVENT, onReady);
    windowObject.removeEventListener(GRAPHICS_FAILED_EVENT, onFailed);
  };

  const settle = (callback, value) => {
    if (settled) {
      return;
    }
    settled = true;
    cleanup();
    callback(value);
  };

  const onReady = (event) => {
    const detail = normalizeGraphicsEventDetail(event?.detail);
    if (detail.backend !== undefined && detail.backend !== expectedBackend.label) {
      return;
    }
    settle(resolveSignal, event?.detail ?? Object.freeze({ backend: expectedBackend.label }));
  };

  const onFailed = (event) => {
    const detail = normalizeGraphicsEventDetail(event?.detail);
    if (detail.backend !== undefined && detail.backend !== expectedBackend.label) {
      return;
    }
    settle(
      rejectSignal,
      new Error(detail.message ?? `${expectedBackend.label} initialization failed`),
    );
  };

  const promise = new Promise((resolve, reject) => {
    resolveSignal = resolve;
    rejectSignal = reject;
    windowObject.addEventListener(GRAPHICS_READY_EVENT, onReady);
    windowObject.addEventListener(GRAPHICS_FAILED_EVENT, onFailed);
    timeout = windowObject.setTimeout(() => {
      settle(reject, new Error(`${expectedBackend.label} initialization timed out`));
    }, timeoutMs);
  });

  return Object.freeze({
    promise,
    cancel: cleanup,
  });
}

async function importAndInitializeArtifact(backend, environment) {
  const artifact = ARTIFACTS[backend];
  const observer = observeGraphicsInitialization(
    environment.windowObject,
    artifact,
    environment.graphicsReadyTimeoutMs,
  );

  try {
    const module = await import(artifact.moduleUrl);
    if (typeof module.default !== "function") {
      throw new TypeError(`${artifact.label} artifact does not export a default initializer`);
    }
    await module.default();
    return await observer.promise;
  } finally {
    observer.cancel();
  }
}

function result(backend, attempts, preflight) {
  return Object.freeze({
    backend,
    label: ARTIFACTS[backend].label,
    attempts: Object.freeze([...attempts]),
    preflight,
  });
}

export async function startNyon(options = {}) {
  const windowObject = options.windowObject ?? globalThis.window;
  const documentObject = options.documentObject ?? globalThis.document;
  const navigatorObject = options.navigatorObject ?? globalThis.navigator;
  const search = options.search ?? windowObject?.location?.search ?? "";
  const graphicsReadyTimeoutMs = options.graphicsReadyTimeoutMs ?? GRAPHICS_READY_TIMEOUT_MS;
  const report = options.report ?? ((state, backend, message) => {
    renderStatus(documentObject, state, backend, message);
  });
  const teardown = options.teardown ?? (() => teardownIncompleteSurface(documentObject));
  const runPreflight = options.preflight ?? (() => preflightWebGpu(navigatorObject));
  const loadArtifact = options.loadArtifact ?? ((backend) => importAndInitializeArtifact(backend, {
    windowObject,
    graphicsReadyTimeoutMs,
  }));
  const attempts = [];

  const startArtifact = async (backend, reason) => {
    const artifact = ARTIFACTS[backend];
    attempts.push(backend);
    report("loading", artifact.label, reason.detail);
    await loadArtifact(backend);
    const readyMessage = reason.ok
      ? `${artifact.label} initialized successfully.`
      : `${artifact.label} initialized successfully. ${reason.detail}`;
    report("ready", artifact.label, readyMessage);
    return result(backend, attempts, reason);
  };

  if (forcedBackend(search) === "webgl2") {
    const forced = Object.freeze({ ok: false, detail: "forced by ?backend=webgl2" });
    try {
      return await startArtifact("webgl2", forced);
    } catch (error) {
      const message = `Forced WEBGL2 LOW startup failed: ${safeDetail(error)}`;
      report("error", "WEBGL2 LOW FAILED", message);
      throw new NyonStartupError(message, attempts, error);
    }
  }

  const preflight = await runPreflight();
  if (!preflight.ok) {
    try {
      return await startArtifact("webgl2", preflight);
    } catch (error) {
      const message = `${preflight.detail}. WEBGL2 LOW startup failed: ${safeDetail(error)}`;
      report("error", "NO GRAPHICS BACKEND", message);
      throw new NyonStartupError(message, attempts, error);
    }
  }

  try {
    return await startArtifact("webgpu", preflight);
  } catch (webGpuError) {
    teardown();
    const fallbackReason = Object.freeze({
      ok: false,
      detail: `${preflight.detail}. WEBGPU initialization failed: ${safeDetail(webGpuError)}`,
    });
    try {
      return await startArtifact("webgl2", fallbackReason);
    } catch (webGlError) {
      const message = `${fallbackReason.detail}. WEBGL2 LOW startup failed: ${safeDetail(webGlError)}`;
      report("error", "NO GRAPHICS BACKEND", message);
      throw new NyonStartupError(message, attempts, webGlError);
    }
  }
}

if (typeof window !== "undefined" && typeof document !== "undefined") {
  startNyon().catch((error) => {
    console.error("NYON startup failed", error);
  });
}
