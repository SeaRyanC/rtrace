#!/usr/bin/env node

const { arch, platform } = process;
const { spawnSync } = require("node:child_process");
const path = require("node:path");

const target = {
  "darwin-arm64": "aarch64-apple-darwin",
  "darwin-x64": "x86_64-apple-darwin",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "win32-arm64": "aarch64-pc-windows-msvc",
  "win32-x64": "x86_64-pc-windows-msvc",
}[`${platform}-${arch}`];

if (!target) {
  throw new Error(`Unsupported platform: ${platform}-${arch}`);
}

const executable = path.join(
  __dirname,
  "..",
  "dist",
  `rtrace-${target}${platform === "win32" ? ".exe" : ""}`,
);
const result = spawnSync(executable, process.argv.slice(2), {
  stdio: "inherit",
  windowsHide: false,
});

if (result.error) {
  throw result.error;
}
process.exit(result.status ?? 1);
