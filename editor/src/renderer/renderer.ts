type MonacoApi = typeof import("monaco-editor");
type RenderMode = "rasterize" | "raytracer";
type MenuCommand =
  | "new"
  | "open"
  | "save"
  | "save-as"
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
const SCENE_URI = "inmemory://rtrace/scene.json";

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
let currentPath: string | null = null;
let baseDirectory = "";
let currentMode: RenderMode = "raytracer";
let renderInProgress = false;
let pendingRender = false;
let applyingContent = false;
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
  $("dirty-indicator").classList.toggle("visible", dirty);
  updateTitle();
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

function setContent(content: string, path: string | null, directory: string): void {
  applyingContent = true;
  model.setValue(content);
  applyingContent = false;
  currentPath = path;
  baseDirectory = directory;
  $("file-name").textContent = fileName(path);
  setDirty(false);
  setFileStatus(path ? `Loaded ${path}` : "New scene");
}

function configureSchema(schema: unknown): void {
  monacoApi.languages.json.jsonDefaults.setDiagnosticsOptions({
    validate: true,
    allowComments: false,
    schemas: [
      {
        uri: SCHEMA_URI,
        fileMatch: [SCENE_URI],
        schema,
      },
    ],
  });
}

function updateMarkerStatus(): void {
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
  try {
    const result: SaveSceneResult = await window.rtrace.saveScene({
      filePath: saveAs ? null : currentPath,
      content: model.getValue(),
    });
    currentPath = result.path;
    baseDirectory = result.directory;
    $("file-name").textContent = fileName(currentPath);
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
  if (
    $("dirty-indicator").classList.contains("visible") &&
    !window.confirm("Discard unsaved scene changes?")
  ) {
    return;
  }
  setContent(starterText(), null, appInfo.workspaceRoot);
  requestRender();
}

function openScene(payload: SceneOpenedPayload): void {
  if (
    $("dirty-indicator").classList.contains("visible") &&
    !window.confirm("Discard unsaved scene changes?")
  ) {
    return;
  }
  setContent(payload.content, payload.path, payload.directory);
  requestRender();
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
    configureSchema(await window.rtrace.getSchema());
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
  model = monacoApi.editor.createModel(
    starterText(),
    "json",
    monacoApi.Uri.parse(SCENE_URI),
  );
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
    run: () => editor.getAction("editor.action.formatDocument")?.run(),
  });
  editor.onDidChangeModelContent(() => {
    if (!applyingContent) {
      setDirty(true);
      scheduleRender();
    }
  });
  monacoApi.editor.onDidChangeMarkers(updateMarkerStatus);
  updateMarkerStatus();
  setMode("raytracer");
  setRenderControls();
  window.setTimeout(requestRender, 250);
}

window.startRtraceEditor = () => {
  void initialize(window.monaco).catch((error: unknown) => {
    const message = error instanceof Error ? error.message : String(error);
    setFileStatus(`Editor initialization failed: ${message}`);
  });
};
