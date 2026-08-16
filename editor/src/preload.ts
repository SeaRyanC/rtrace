import { contextBridge, ipcRenderer, type IpcRendererEvent } from "electron";
import type {
  AppInfo,
  EditorBridge,
  MenuCommand,
  RenderFinished,
  RenderFrame,
  RenderProgress,
  SceneOpenedPayload,
  SaveSceneRequest,
} from "./shared";

function subscribe<T>(
  channel: string,
  handler: (payload: T) => void,
): () => void {
  const listener = (_event: IpcRendererEvent, payload: T) => {
    handler(payload);
  };
  ipcRenderer.on(channel, listener);
  return () => ipcRenderer.removeListener(channel, listener);
}

const bridge: EditorBridge = {
  getSchema: () => ipcRenderer.invoke("schema:get"),
  getAppInfo: () => ipcRenderer.invoke("app:info") as Promise<AppInfo>,
  openScene: () => ipcRenderer.invoke("scene:open"),
  saveScene: (request: SaveSceneRequest) =>
    ipcRenderer.invoke("scene:save", request),
  startRender: (request) => ipcRenderer.invoke("render:start", request),
  cancelRender: () => ipcRenderer.invoke("render:cancel"),
  onSceneOpened: (handler) =>
    subscribe<SceneOpenedPayload>("scene:opened", handler),
  onMenuCommand: (handler) =>
    subscribe<MenuCommand>("menu:command", handler),
  onRenderProgress: (handler) =>
    subscribe<RenderProgress>("render:progress", handler),
  onRenderFrame: (handler) => subscribe<RenderFrame>("render:frame", handler),
  onRenderFinished: (handler) =>
    subscribe<RenderFinished>("render:finished", handler),
  onRenderError: (handler) => subscribe<string>("render:error", handler),
};

contextBridge.exposeInMainWorld("rtrace", bridge);
