import { cpSync, copyFileSync, mkdirSync } from "node:fs";

mkdirSync("editor/dist/renderer", { recursive: true });
copyFileSync(
  "editor/src/renderer/index.html",
  "editor/dist/renderer/index.html",
);
copyFileSync(
  "editor/src/renderer/styles.css",
  "editor/dist/renderer/styles.css",
);
copyFileSync("editor/scene.schema.json", "editor/dist/scene.schema.json");
cpSync("node_modules/monaco-editor/min/vs", "editor/dist/renderer/vs", {
  recursive: true,
});
