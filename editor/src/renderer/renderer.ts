type MonacoApi = typeof import("monaco-editor");
type RenderMode = "rasterize" | "raytracer";
type MenuCommand =
  | "new"
  | "open"
  | "save"
  | "save-as"
  | "close"
  | "format"
  | "render"
  | "cancel-render"
  | "rasterize"
  | "raytracer";
type SceneOpenedPayload = {
  path: string;
  directory: string;
  content: string;
};
type SaveSceneResult = {
  path: string;
  directory: string;
};
type RenderProgress = {
  mode: RenderMode;
  step: number;
  totalSteps: number;
  diagonal: number;
};
type RenderFrame = RenderProgress & {
  width: number;
  height: number;
  dataUrl: string;
};
type RenderFinished = {
  mode: RenderMode;
  cancelled: boolean;
};

const SCHEMA_URI = "inmemory://rtrace/scene.schema.json";
const SCENE_URI_PREFIX = "inmemory://rtrace/scene-";

type SceneTab = {
  id: number;
  path: string | null;
  directory: string;
  dirty: boolean;
  model: import("monaco-editor").editor.ITextModel;
};

const starterScene = {
  camera: {
    kind: "perspective",
    position: [5, -8, 5],
    target: [0, 0, 1],
    up: [0, 0, 1],
    width: 6,
    height: 4,
    fov: 42,
  },
  objects: [
    {
      kind: "sphere",
      center: [0, 0, 1],
      radius: 1,
      material: {
        color: "#4F9DDE",
        ambient: 0.12,
        diffuse: 0.78,
        specular: 0.25,
        shininess: 40,
      },
    },
    {
      kind: "plane",
      point: [0, 0, 0],
      normal: [0, 0, 1],
      material: {
        color: "#3A414B",
        ambient: 0.18,
        diffuse: 0.7,
        specular: 0.08,
        shininess: 20,
      },
    },
  ],
  lights: [
    {
      position: [4, -5, 8],
      color: "#FFF2D5",
      intensity: 1.1,
      diameter: 1.5,
    },
    {
      position: [-4, -1, 4],
      color: "#BBD8FF",
      intensity: 0.35,
    },
  ],
  scene_settings: {
    ambient_illumination: {
      color: "#FFFFFF",
      intensity: 0.14,
    },
    background_color: "#11151B",
  },
};

let monacoApi: MonacoApi;
let editor: import("monaco-editor").editor.IStandaloneCodeEditor;
let model: import("monaco-editor").editor.ITextModel;
let appInfo: { workspaceRoot: string };
let sceneSchema: unknown;
let tabs: SceneTab[] = [];
let activeTab: SceneTab | null = null;
let nextTabId = 1;
let currentPath: string | null = null;
let baseDirectory = "";
let currentMode: RenderMode = "raytracer";
let renderInProgress = false;
let pendingRender = false;
let renderTimer: number | undefined;

const $ = <T extends HTMLElement>(id: string): T => {
  const element = document.getElementById(id);
  if (!element) {
    throw new Error(`Missing editor element #${id}`);
  }
  return element as T;
};

function starterText(): string {
  return JSON.stringify(starterScene, null, 2);
}

function fileName(path: string | null): string {
  if (!path) {
    return "Untitled scene";
  }
  return path.split(/[\\/]/).pop() ?? path;
}

function tabLabel(tab: SceneTab): string {
  return tab.path ? fileName(tab.path) : `Untitled-${tab.id}`;
}

function setFileStatus(message: string): void {
  $("file-status").textContent = message;
}

function setRenderStatus(message: string): void {
  $("render-status").textContent = message;
}

function updateTitle(): void {
  const dirty = $("dirty-indicator").classList.contains("visible");
  document.title = `${dirty ? "* " : ""}${fileName(currentPath)} - rtrace Scene Editor`;
}

function setDirty(dirty: boolean): void {
  if (activeTab) {
    activeTab.dirty = dirty;
  }
  $("dirty-indicator").classList.toggle("visible", dirty);
  renderTabBar();
  updateTitle();
}

function renderTabBar(): void {
  const tabBar = $("tab-bar");
  tabBar.replaceChildren();

  for (const tab of tabs) {
    const tabElement = document.createElement("div");
    tabElement.className = "editor-tab";
    tabElement.setAttribute("role", "tab");
    tabElement.setAttribute("aria-selected", String(tab === activeTab));
    tabElement.tabIndex = tab === activeTab ? 0 : -1;
    if (tab === activeTab) {
      tabElement.classList.add("active");
    }
    tabElement.addEventListener("click", () => {
      activateTab(tab);
      requestRender();
    });
    tabElement.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        activateTab(tab);
        requestRender();
      }
    });

    const label = document.createElement("span");
    label.className = "tab-label";
    label.textContent = tabLabel(tab);
    label.title = tab.path ?? "Unsaved scene";
    tabElement.append(label);

    const dirty = document.createElement("span");
    dirty.className = "tab-dirty";
    dirty.textContent = "●";
    dirty.hidden = !tab.dirty;
    dirty.setAttribute("aria-label", "Unsaved changes");
    tabElement.append(dirty);

    const close = document.createElement("button");
    close.className = "tab-close";
    close.type = "button";
    close.title = `Close ${tabLabel(tab)}`;
    close.setAttribute("aria-label", `Close ${tabLabel(tab)}`);
    close.textContent = "×";
    close.addEventListener("click", (event) => {
      event.stopPropagation();
      closeTab(tab);
    });
    tabElement.append(close);
    tabBar.append(tabElement);
  }
}

function syncActiveTabState(): void {
  if (!activeTab) {
    currentPath = null;
    baseDirectory = appInfo.workspaceRoot;
    return;
  }

  model = activeTab.model;
  currentPath = activeTab.path;
  baseDirectory = activeTab.directory;
}

function activateTab(tab: SceneTab): void {
  if (activeTab === tab) {
    return;
  }

  activeTab = tab;
  syncActiveTabState();
  if (editor && editor.getModel() !== tab.model) {
    editor.setModel(tab.model);
  }
  $("file-name").textContent = tabLabel(tab);
  setDirty(tab.dirty);
  setFileStatus(tab.path ? `Loaded ${tab.path}` : "New scene");
  renderTabBar();
}

function createTab(
  content: string,
  path: string | null,
  directory: string,
): SceneTab {
  const id = nextTabId++;
  const tab: SceneTab = {
    id,
    path,
    directory,
    dirty: false,
    model: monacoApi.editor.createModel(
      content,
      "json",
      monacoApi.Uri.parse(`${SCENE_URI_PREFIX}${id}.json`),
    ),
  };
  tabs.push(tab);
  configureSchema();
  if (!activeTab) {
    activeTab = tab;
    syncActiveTabState();
    if (editor && editor.getModel() !== tab.model) {
      editor.setModel(tab.model);
    }
    $("file-name").textContent = tabLabel(tab);
    $("dirty-indicator").classList.remove("visible");
    updateTitle();
  }
  renderTabBar();
  return tab;
}

function closeTab(tab: SceneTab): void {
  if (tab.dirty && !window.confirm(`Discard unsaved changes in ${tabLabel(tab)}?`)) {
    return;
  }

  const index = tabs.indexOf(tab);
  if (index < 0) {
    return;
  }
  const wasActive = tab === activeTab;
  tabs.splice(index, 1);
  tab.model.dispose();
  configureSchema();

  if (!tabs.length) {
    activeTab = null;
    createTab(starterText(), null, appInfo.workspaceRoot);
  } else if (wasActive) {
    activateTab(tabs[Math.min(index, tabs.length - 1)]);
  }

  renderTabBar();
  if (wasActive) {
    requestRender();
  }
}

function setMode(mode: RenderMode): void {
  currentMode = mode;
  $("rasterize-mode").classList.toggle("active", mode === "rasterize");
  $("raytracer-mode").classList.toggle("active", mode === "raytracer");
  setRenderStatus(`${mode === "rasterize" ? "Rasterizer" : "Raytracer"} ready`);
}

function setRenderControls(): void {
  $("render-scene").toggleAttribute("disabled", renderInProgress);
  $("cancel-render").toggleAttribute("disabled", !renderInProgress);
}

function configureSchema(): void {
  if (sceneSchema === undefined) {
    return;
  }
  monacoApi.languages.json.jsonDefaults.setDiagnosticsOptions({
    validate: true,
    allowComments: false,
    schemas: [
      {
        uri: SCHEMA_URI,
        fileMatch: tabs.map((tab) => tab.model.uri.toString()),
        schema: sceneSchema,
      },
    ],
  });
}

function updateMarkerStatus(): void {
  if (!model) {
    return;
  }
  const markers = monacoApi.editor.getModelMarkers({ resource: model.uri });
  const errors = markers.filter(
    (marker) => marker.severity === monacoApi.MarkerSeverity.Error,
  ).length;
  if (errors > 0) {
    setFileStatus(`${errors} JSON/schema error${errors === 1 ? "" : "s"}`);
  } else if (currentPath) {
    setFileStatus(`Loaded ${currentPath}`);
  } else {
    setFileStatus("New scene");
  }
}

function targetSize(): { width: number; height: number } {
  const rect = $("preview-stage").getBoundingClientRect();
  const scale = Math.min(window.devicePixelRatio || 1, 2);
  return {
    width: Math.max(64, Math.round(rect.width * scale)),
    height: Math.max(64, Math.round(rect.height * scale)),
  };
}

function showFrame(frame: RenderFrame): void {
  const image = $("preview-image") as HTMLImageElement;
  image.src = frame.dataUrl;
  image.classList.add("visible");
  $("preview-placeholder").setAttribute("hidden", "");
  $("render-size").textContent = `${frame.diagonal}px`;
}

function showProgress(progress: RenderProgress): void {
  const modeName = progress.mode === "rasterize" ? "Rasterizer" : "Raytracer";
  const percentage = Math.round((progress.step / progress.totalSteps) * 100);
  $("progress-overlay").removeAttribute("hidden");
  $("progress-bar").style.width = `${percentage}%`;
  $("progress-label").textContent = `${modeName}: ${progress.diagonal}px (${progress.step}/${progress.totalSteps})`;
  setRenderStatus(`Rendering ${modeName.toLowerCase()}...`);
}

async function beginRender(): Promise<void> {
  if (renderInProgress) {
    pendingRender = true;
    await window.rtrace.cancelRender();
    return;
  }

  try {
    JSON.parse(model.getValue());
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    setFileStatus(`Cannot render invalid JSON: ${message}`);
    return;
  }

  const size = targetSize();
  renderInProgress = true;
  setRenderControls();
  pendingRender = false;
  $("progress-overlay").removeAttribute("hidden");
  $("progress-bar").style.width = "0%";
  setRenderStatus(`Starting ${currentMode === "rasterize" ? "rasterizer" : "raytracer"}...`);

  try {
    await window.rtrace.startRender({
      sceneJson: model.getValue(),
      baseDirectory: baseDirectory || appInfo.workspaceRoot,
      mode: currentMode,
      targetWidth: size.width,
      targetHeight: size.height,
    });
  } catch (error) {
    renderInProgress = false;
    setRenderControls();
    const message = error instanceof Error ? error.message : String(error);
    setRenderStatus(`Render failed: ${message}`);
  }
}

function requestRender(): void {
  if (renderTimer !== undefined) {
    window.clearTimeout(renderTimer);
  }
  void beginRender();
}

function scheduleRender(): void {
  if (renderTimer !== undefined) {
    window.clearTimeout(renderTimer);
  }
  renderTimer = window.setTimeout(() => {
    renderTimer = undefined;
    requestRender();
  }, 700);
}

async function saveScene(saveAs: boolean): Promise<void> {
  if (!activeTab) {
    return;
  }

  try {
    const result: SaveSceneResult = await window.rtrace.saveScene({
      filePath: saveAs ? null : activeTab.path,
      content: model.getValue(),
    });
    activeTab.path = result.path;
    activeTab.directory = result.directory;
    activeTab.dirty = false;
    syncActiveTabState();
    currentPath = result.path;
    baseDirectory = result.directory;
    $("file-name").textContent = tabLabel(activeTab);
    setDirty(false);
    setFileStatus(`Saved ${currentPath}`);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (message !== "Save cancelled") {
      setFileStatus(`Save failed: ${message}`);
    }
  }
}

function newScene(): void {
  const tab = createTab(starterText(), null, appInfo.workspaceRoot);
  activateTab(tab);
  requestRender();
}

function openScene(payload: SceneOpenedPayload): void {
  const existing = tabs.find((tab) => tab.path === payload.path);
  if (existing) {
    activateTab(existing);
  } else {
    const tab = createTab(payload.content, payload.path, payload.directory);
    activateTab(tab);
  }
  requestRender();
}

function formatJsonValue(value: unknown, indent: string): string {
  if (Array.isArray(value)) {
    if (
      value.length === 3 &&
      value.every(
        (item) => typeof item === "number" && Number.isFinite(item),
      )
    ) {
      return `[${value.map((item) => JSON.stringify(item)).join(", ")}]`;
    }
    if (value.length === 0) {
      return "[]";
    }
    const childIndent = `${indent}  `;
    return `[\n${value
      .map((item) => `${childIndent}${formatJsonValue(item, childIndent)}`)
      .join(",\n")}\n${indent}]`;
  }

  if (value !== null && typeof value === "object") {
    const entries = Object.entries(value);
    if (entries.length === 0) {
      return "{}";
    }
    const childIndent = `${indent}  `;
    return `{\n${entries
      .map(
        ([key, item]) =>
          `${childIndent}${JSON.stringify(key)}: ${formatJsonValue(item, childIndent)}`,
      )
      .join(",\n")}\n${indent}}`;
  }

  return JSON.stringify(value) ?? "null";
}

function formatJsonDocument(): void {
  try {
    const formatted = `${formatJsonValue(JSON.parse(model.getValue()), "")}\n`;
    editor.executeEdits("rtrace.format-json", [
      {
        range: model.getFullModelRange(),
        text: formatted,
      },
    ]);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    setFileStatus(`Cannot format invalid JSON: ${message}`);
  }
}

function handleMenuCommand(command: MenuCommand): void {
  switch (command) {
    case "new":
      newScene();
      break;
    case "open":
      break;
    case "save":
      void saveScene(false);
      break;
    case "save-as":
      void saveScene(true);
      break;
    case "close":
      if (activeTab) {
        closeTab(activeTab);
      }
      break;
    case "format":
      formatJsonDocument();
      break;
    case "render":
      requestRender();
      break;
    case "cancel-render":
      void window.rtrace.cancelRender();
      break;
    case "rasterize":
      setMode("rasterize");
      requestRender();
      break;
    case "raytracer":
      setMode("raytracer");
      requestRender();
      break;
  }
}

function registerUi(): void {
  $("new-scene").addEventListener("click", newScene);
  $("open-scene").addEventListener("click", () => {
    void window.rtrace.openScene();
  });
  $("save-scene").addEventListener("click", () => void saveScene(false));
  $("save-as-scene").addEventListener("click", () => void saveScene(true));
  $("render-scene").addEventListener("click", requestRender);
  $("cancel-render").addEventListener("click", () => {
    void window.rtrace.cancelRender();
  });
  $("rasterize-mode").addEventListener("click", () => {
    setMode("rasterize");
    requestRender();
  });
  $("raytracer-mode").addEventListener("click", () => {
    setMode("raytracer");
    requestRender();
  });

  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && renderInProgress) {
      event.preventDefault();
      void window.rtrace.cancelRender();
    } else if (
      event.key.toLowerCase() === "w" &&
      (event.ctrlKey || event.metaKey) &&
      activeTab
    ) {
      event.preventDefault();
      closeTab(activeTab);
    }
  });

  window.rtrace.onSceneOpened(openScene);
  window.rtrace.onMenuCommand(handleMenuCommand);
  window.rtrace.onRenderProgress(showProgress);
  window.rtrace.onRenderFrame(showFrame);
  window.rtrace.onRenderFinished((finished: RenderFinished) => {
    renderInProgress = false;
    $("progress-overlay").setAttribute("hidden", "");
    setRenderControls();
    setRenderStatus(
      finished.cancelled
        ? "Render cancelled"
        : `${finished.mode === "rasterize" ? "Rasterizer" : "Raytracer"} complete`,
    );
    if (pendingRender) {
      pendingRender = false;
      requestRender();
    }
  });
  window.rtrace.onRenderError((message) => {
    renderInProgress = false;
    $("progress-overlay").setAttribute("hidden", "");
    setRenderControls();
    setRenderStatus(`Render failed: ${message}`);
  });
}

async function initialize(api: MonacoApi): Promise<void> {
  monacoApi = api;
  registerUi();
  appInfo = await window.rtrace.getAppInfo();

  try {
    sceneSchema = await window.rtrace.getSchema();
    configureSchema();
    setFileStatus("Scene schema ready");
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    setFileStatus(`Scene schema unavailable: ${message}`);
  }

  monacoApi.editor.defineTheme("rtrace-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [],
    colors: {
      "editor.background": "#1e1e1e",
      "editorLineNumber.foreground": "#5a5d61",
      "editorCursor.foreground": "#75beff",
      "editor.selectionBackground": "#264f78",
    },
  });
  monacoApi.editor.setTheme("rtrace-dark");
  createTab(starterText(), null, appInfo.workspaceRoot);
  editor = monacoApi.editor.create($("editor"), {
    model,
    automaticLayout: true,
    fontSize: 13,
    minimap: { enabled: true },
    folding: true,
    scrollBeyondLastLine: false,
    renderWhitespace: "selection",
    tabSize: 2,
    wordWrap: "on",
  });
  editor.addAction({
    id: "rtrace.format-document",
    label: "Format Scene JSON",
    keybindings: [monacoApi.KeyMod.Shift | monacoApi.KeyMod.Alt | monacoApi.KeyCode.KeyF],
    run: () => formatJsonDocument(),
  });
  editor.onDidChangeModelContent(() => {
    setDirty(true);
    scheduleRender();
  });
  monacoApi.editor.onDidChangeMarkers(updateMarkerStatus);
  updateMarkerStatus();
  setMode("raytracer");
  setRenderControls();
  renderTabBar();
  window.setTimeout(requestRender, 250);
}

window.startRtraceEditor = () => {
  void initialize(window.monaco).catch((error: unknown) => {
    const message = error instanceof Error ? error.message : String(error);
    setFileStatus(`Editor initialization failed: ${message}`);
  });
};
