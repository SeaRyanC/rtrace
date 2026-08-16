import {
  app,
  BrowserWindow,
  dialog,
  ipcMain,
  Menu,
  shell,
  type MenuItemConstructorOptions,
  type WebContents,
} from "electron";
import { randomUUID } from "node:crypto";
import { promises as fs, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawn, type ChildProcess } from "node:child_process";
import type {
  MenuCommand,
  RenderFinished,
  RenderFrame,
  RenderProgress,
  RenderRequest,
  SceneOpenedPayload,
  SaveSceneRequest,
  SaveSceneResult,
} from "./shared";

const workspaceRoot = resolve(__dirname, "..", "..");
const schemaPath = join(__dirname, "scene.schema.json");
const isWindows = process.platform === "win32";

interface ActiveRender {
  cancelled: boolean;
  child: ChildProcess | null;
  sender: WebContents;
  request: RenderRequest;
}

class RenderCancelled extends Error {
  constructor() {
    super("Render cancelled");
    this.name = "RenderCancelled";
  }
}

let mainWindow: BrowserWindow | null = null;
let activeRender: ActiveRender | null = null;

function sendToRenderer<T>(sender: WebContents, channel: string, payload: T): void {
  if (!sender.isDestroyed()) {
    sender.send(channel, payload);
  }
}

function cliCandidates(): string[] {
  const executableName = isWindows ? "rtrace-cli.exe" : "rtrace-cli";
  const configured = process.env.RTRACE_CLI
    ? [resolve(process.env.RTRACE_CLI)]
    : [];

  return [
    ...configured,
    join(workspaceRoot, "target", "release", executableName),
    join(workspaceRoot, "target", "debug", executableName),
  ];
}

function findCli(): string {
  const candidate = cliCandidates().find((path) => existsSync(path));
  if (!candidate) {
    throw new Error(
      `Could not find rtrace-cli. Build it with "cargo build --release -p rtrace-cli" or set RTRACE_CLI.`,
    );
  }
  return candidate;
}

function renderSizes(targetWidth: number, targetHeight: number): number[] {
  const targetDiagonal = Math.max(
    64,
    Math.ceil(Math.hypot(targetWidth, targetHeight)),
  );
  const sizes: number[] = [];
  let size = Math.min(128, targetDiagonal);

  while (true) {
    sizes.push(size);
    if (size >= targetDiagonal) {
      return sizes;
    }
    size = Math.min(targetDiagonal, size * 2);
  }
}

function runCli(
  executable: string,
  args: string[],
  render: ActiveRender,
): Promise<void> {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(executable, args, {
      cwd: workspaceRoot,
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
    render.child = child;

    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", () => undefined);
    child.stderr.on("data", (chunk: string) => {
      stderr += chunk;
    });
    child.once("error", (error) => {
      rejectPromise(error);
    });
    child.once("close", (code, signal) => {
      render.child = null;
      if (render.cancelled) {
        rejectPromise(new RenderCancelled());
      } else if (code === 0) {
        resolvePromise();
      } else {
        const detail = stderr.trim() || `process exited with ${signal ?? code}`;
        rejectPromise(new Error(`rtrace-cli failed: ${detail}`));
      }
    });
  });
}

async function removeIfPresent(path: string): Promise<void> {
  try {
    await fs.unlink(path);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
      throw error;
    }
  }
}

async function runProgressiveRender(render: ActiveRender): Promise<void> {
  const { request, sender } = render;
  const previewPath = join(
    request.baseDirectory,
    `.rtrace-editor-preview-${process.pid}-${randomUUID()}.json`,
  );
  const outputPath = join(
    tmpdir(),
    `rtrace-editor-render-${process.pid}-${randomUUID()}.png`,
  );
  const sizes = renderSizes(request.targetWidth, request.targetHeight);

  try {
    if (!existsSync(request.baseDirectory)) {
      throw new Error(`Scene directory does not exist: ${request.baseDirectory}`);
    }

    await fs.writeFile(previewPath, request.sceneJson, "utf8");
    const executable = findCli();

    for (const [index, diagonal] of sizes.entries()) {
      if (render.cancelled) {
        throw new RenderCancelled();
      }

      const progress: RenderProgress = {
        mode: request.mode,
        step: index + 1,
        totalSteps: sizes.length,
        diagonal,
      };
      sendToRenderer(sender, "render:progress", progress);

      const args = [
        "--input",
        previewPath,
        "--output",
        outputPath,
        "--size",
        String(diagonal),
      ];
      if (request.mode === "rasterize") {
        args.push("--rasterize");
      } else {
        args.push("--samples", "1");
      }

      await runCli(executable, args, render);
      if (render.cancelled) {
        throw new RenderCancelled();
      }

      const image = await fs.readFile(outputPath);
      const frame: RenderFrame = {
        ...progress,
        width: request.targetWidth,
        height: request.targetHeight,
        dataUrl: `data:image/png;base64,${image.toString("base64")}`,
      };
      sendToRenderer(sender, "render:frame", frame);
    }

    const finished: RenderFinished = {
      mode: request.mode,
      cancelled: false,
    };
    sendToRenderer(sender, "render:finished", finished);
  } catch (error) {
    if (error instanceof RenderCancelled || render.cancelled) {
      const finished: RenderFinished = {
        mode: request.mode,
        cancelled: true,
      };
      sendToRenderer(sender, "render:finished", finished);
    } else {
      const message = error instanceof Error ? error.message : String(error);
      sendToRenderer(sender, "render:error", message);
    }
  } finally {
    await removeIfPresent(previewPath);
    await removeIfPresent(outputPath);
    if (activeRender === render) {
      activeRender = null;
    }
  }
}

async function openScene(): Promise<void> {
  if (!mainWindow) {
    return;
  }

  const result = await dialog.showOpenDialog(mainWindow, {
    properties: ["openFile"],
    filters: [{ name: "rtrace scenes", extensions: ["json"] }],
  });
  if (result.canceled || result.filePaths.length === 0) {
    return;
  }

  const path = result.filePaths[0];
  try {
    const payload: SceneOpenedPayload = {
      path,
      directory: resolve(path, ".."),
      content: await fs.readFile(path, "utf8"),
    };
    sendToRenderer(mainWindow.webContents, "scene:opened", payload);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    await dialog.showMessageBox(mainWindow, {
      type: "error",
      title: "Open scene failed",
      message,
    });
  }
}

async function saveScene(request: SaveSceneRequest): Promise<SaveSceneResult> {
  if (!mainWindow) {
    throw new Error("The editor window is not available");
  }

  let path = request.filePath;
  if (!path) {
    const result = await dialog.showSaveDialog(mainWindow, {
      defaultPath: join(workspaceRoot, "scene.json"),
      filters: [{ name: "rtrace scenes", extensions: ["json"] }],
    });
    if (result.canceled || !result.filePath) {
      throw new Error("Save cancelled");
    }
    path = result.filePath;
  }

  await fs.writeFile(path, request.content, "utf8");
  return {
    path,
    directory: resolve(path, ".."),
  };
}

function sendMenuCommand(command: MenuCommand): void {
  if (mainWindow) {
    sendToRenderer(mainWindow.webContents, "menu:command", command);
  }
}

function createApplicationMenu(): void {
  const template: MenuItemConstructorOptions[] = [
    {
      label: "File",
      submenu: [
        { label: "New Scene", accelerator: "CmdOrCtrl+N", click: () => sendMenuCommand("new") },
        { label: "Open Scene...", accelerator: "CmdOrCtrl+O", click: () => void openScene() },
        { type: "separator" },
        { label: "Save", accelerator: "CmdOrCtrl+S", click: () => sendMenuCommand("save") },
        {
          label: "Save As...",
          accelerator: "CmdOrCtrl+Shift+S",
          click: () => sendMenuCommand("save-as"),
        },
        { type: "separator" },
        { role: "quit" },
      ],
    },
    {
      label: "Render",
      submenu: [
        { label: "Render Scene", accelerator: "F5", click: () => sendMenuCommand("render") },
        {
          label: "Cancel Render",
          accelerator: "Esc",
          click: () => sendMenuCommand("cancel-render"),
        },
        { type: "separator" },
        {
          label: "Rasterizer Preview",
          accelerator: "CmdOrCtrl+1",
          click: () => sendMenuCommand("rasterize"),
        },
        {
          label: "Raytracer Preview",
          accelerator: "CmdOrCtrl+2",
          click: () => sendMenuCommand("raytracer"),
        },
      ],
    },
    {
      label: "View",
      submenu: [
        { role: "toggleDevTools" },
        { role: "resetZoom" },
        { role: "zoomIn" },
        { role: "zoomOut" },
      ],
    },
    {
      label: "Help",
      submenu: [
        {
          label: "rtrace Documentation",
          click: () => void shell.openExternal("https://github.com/SeaRyanC/rtrace"),
        },
      ],
    },
  ];

  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}

function createWindow(): void {
  mainWindow = new BrowserWindow({
    width: 1440,
    height: 900,
    minWidth: 960,
    minHeight: 640,
    backgroundColor: "#1e1e1e",
    show: false,
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      preload: join(__dirname, "preload.js"),
    },
  });

  mainWindow.once("ready-to-show", () => mainWindow?.show());
  mainWindow.on("closed", () => {
    if (activeRender) {
      activeRender.cancelled = true;
      activeRender.child?.kill();
      activeRender = null;
    }
    mainWindow = null;
  });
  void mainWindow.loadFile(join(__dirname, "renderer", "index.html"));
  createApplicationMenu();
}

ipcMain.handle("schema:get", async () =>
  JSON.parse(await fs.readFile(schemaPath, "utf8")),
);

ipcMain.handle("app:info", () => ({
  workspaceRoot,
}));

ipcMain.handle("scene:save", (_event, request: SaveSceneRequest) =>
  saveScene(request),
);

ipcMain.handle("scene:open", () => openScene());

ipcMain.handle("render:start", (event, request: RenderRequest) => {
  if (activeRender) {
    throw new Error("A render is already in progress");
  }
  if (
    typeof request.sceneJson !== "string" ||
    typeof request.baseDirectory !== "string" ||
    (request.mode !== "rasterize" && request.mode !== "raytracer") ||
    !Number.isFinite(request.targetWidth) ||
    !Number.isFinite(request.targetHeight)
  ) {
    throw new Error("Invalid render request");
  }

  const render: ActiveRender = {
    cancelled: false,
    child: null,
    sender: event.sender,
    request: {
      ...request,
      targetWidth: Math.max(1, Math.round(request.targetWidth)),
      targetHeight: Math.max(1, Math.round(request.targetHeight)),
    },
  };
  activeRender = render;
  void runProgressiveRender(render);
  return { accepted: true as const };
});

ipcMain.handle("render:cancel", () => {
  if (!activeRender) {
    return { cancelled: false };
  }
  activeRender.cancelled = true;
  activeRender.child?.kill();
  return { cancelled: true };
});

void app.whenReady().then(() => {
  createWindow();
  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});
