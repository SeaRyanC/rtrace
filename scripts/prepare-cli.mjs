import { copyFileSync, chmodSync, mkdirSync } from "node:fs";
import { join } from "node:path";

const target = process.argv[2];
if (!target) {
  throw new Error("Usage: node scripts/prepare-cli.mjs <target>");
}

const executable = target.includes("windows") ? "rtrace-cli.exe" : "rtrace-cli";
const source = join("target", target, "release", executable);
const destination = join("dist", `rtrace-${target}${target.includes("windows") ? ".exe" : ""}`);

mkdirSync("dist", { recursive: true });
copyFileSync(source, destination);
if (!target.includes("windows")) {
  chmodSync(destination, 0o755);
}

console.log(`Prepared ${destination}`);
