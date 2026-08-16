import { copyFileSync, mkdirSync } from "node:fs";

mkdirSync("editor", { recursive: true });
copyFileSync("schema.json", "editor/scene.schema.json");
