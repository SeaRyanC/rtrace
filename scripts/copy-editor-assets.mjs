import { cpSync, copyFileSync, mkdirSync } from "node:fs";

mkdirSync("editor/dist/renderer", { recursive: true });
mkdirSync("dist/editor/renderer", { recursive: true });
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

for (const file of ["main.js", "preload.js", "shared.js"]) {
  copyFileSync(`editor/dist/${file}`, `dist/editor/${file}`);
}
copyFileSync("editor/scene.schema.json", "dist/editor/scene.schema.json");
copyFileSync("scripts/rtrace-editor.js", "dist/rtrace-editor.js");
copyFileSync(
  "editor/src/renderer/index.html",
  "dist/editor/renderer/index.html",
);
copyFileSync(
  "editor/src/renderer/styles.css",
  "dist/editor/renderer/styles.css",
);
cpSync("node_modules/monaco-editor/min/vs", "dist/editor/renderer/vs", {
  recursive: true,
});
