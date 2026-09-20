import { copyFileSync, mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const gui = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(gui, "../..");
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const npm = process.platform === "win32" ? "npm.cmd" : "npm";

function run(command, args, cwd, env = process.env) {
  const result = spawnSync(command, args, { cwd, env, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

run(cargo, ["build", "-p", "masterdata-web", "--target", "wasm32-unknown-unknown", "--release"], root);
mkdirSync(resolve(gui, "public"), { recursive: true });
copyFileSync(resolve(root, "target/wasm32-unknown-unknown/release/masterdata_web.wasm"), resolve(gui, "public/masterdata_web.wasm"));
run(process.execPath, ["--test", "tests/web-wasm-smoke.mjs"], gui);
run(npm, ["run", "lint"], gui);
run(npm, ["exec", "--", "vite", "build", "--outDir", "dist-web", "--base", "./"], gui, {
  ...process.env,
  VITE_MASTERDATA_WEB: "1",
});
