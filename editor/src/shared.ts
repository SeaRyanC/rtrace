export type RenderMode = "rasterize" | "raytracer";

export type MenuCommand =
  | "new"
  | "open"
  | "save"
  | "save-as"
  | "render"
  | "cancel-render"
  | "rasterize"
  | "raytracer";

export interface SceneOpenedPayload {
  path: string;
  directory: string;
  content: string;
}

export interface SaveSceneRequest {
  filePath: string | null;
  content: string;
}

export interface SaveSceneResult {
  path: string;
  directory: string;
}

export interface RenderRequest {
  sceneJson: string;
  baseDirectory: string;
  mode: RenderMode;
  targetWidth: number;
  targetHeight: number;
}

export interface RenderProgress {
  mode: RenderMode;
  step: number;
  totalSteps: number;
  diagonal: number;
}

export interface RenderFrame extends RenderProgress {
  width: number;
  height: number;
  dataUrl: string;
}

export interface RenderFinished {
  mode: RenderMode;
  cancelled: boolean;
}

export interface AppInfo {
  workspaceRoot: string;
}

export interface EditorBridge {
  getSchema(): Promise<unknown>;
  getAppInfo(): Promise<AppInfo>;
  openScene(): Promise<void>;
  saveScene(request: SaveSceneRequest): Promise<SaveSceneResult>;
  startRender(request: RenderRequest): Promise<{ accepted: true }>;
  cancelRender(): Promise<{ cancelled: boolean }>;
  onSceneOpened(handler: (payload: SceneOpenedPayload) => void): () => void;
  onMenuCommand(handler: (command: MenuCommand) => void): () => void;
  onRenderProgress(handler: (payload: RenderProgress) => void): () => void;
  onRenderFrame(handler: (payload: RenderFrame) => void): () => void;
  onRenderFinished(handler: (payload: RenderFinished) => void): () => void;
  onRenderError(handler: (message: string) => void): () => void;
}
