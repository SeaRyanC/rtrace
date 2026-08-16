#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const path = require("node:path");

const electron = require("electron");
const editorEntry = path.join(__dirname, "editor", "main.js");

const result = spawnSync(electron, [editorEntry, ...process.argv.slice(2)], {
  stdio: "inherit",
  windowsHide: false,
});

if (result.error) {
  throw result.error;
}

process.exit(result.status ?? 1);
