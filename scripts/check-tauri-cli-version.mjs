import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const packageJson = JSON.parse(
  readFileSync(new URL("../package.json", import.meta.url), "utf8"),
);
const expectedVersion = packageJson.devDependencies?.["@tauri-apps/cli"];

if (!/^\d+\.\d+\.\d+$/.test(expectedVersion)) {
  throw new Error(
    `@tauri-apps/cli must use an exact version, received "${expectedVersion}"`,
  );
}

if (!process.env.npm_execpath) {
  throw new Error(
    "Run this check through pnpm so the local CLI can be resolved",
  );
}

const cliOutput = execFileSync(
  process.execPath,
  [process.env.npm_execpath, "exec", "tauri", "--version"],
  { encoding: "utf8" },
).trim();
const installedVersion = /^tauri-cli (\d+\.\d+\.\d+)$/.exec(cliOutput)?.[1];

if (!installedVersion) {
  throw new Error(`Unexpected Tauri CLI version output: "${cliOutput}"`);
}

if (installedVersion !== expectedVersion) {
  throw new Error(
    `Tauri CLI version mismatch: expected ${expectedVersion}, received ${installedVersion}`,
  );
}

console.log(`Tauri CLI version verified: ${installedVersion}`);
