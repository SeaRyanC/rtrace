import type { EditorBridge } from "./shared";

declare global {
  interface Window {
    rtrace: EditorBridge;
    monaco: typeof import("monaco-editor");
    startRtraceEditor: () => void;
    MonacoEnvironment: {
      getWorkerUrl: (moduleId: string, label: string) => string;
    };
  }
}

export {};
