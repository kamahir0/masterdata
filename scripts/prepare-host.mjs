import {
  mkdir,
  copyFile,
  readFile,
  chmod,
  rm,
} from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { resolve, join } from "node:path";

const root = resolve(import.meta.dirname, "..");

export function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    timeout: 600000,
    ...options,
  });
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error(`${command} failed (${result.status ?? result.signal})`);
  return result;
}

await mkdir(join(root, "src-tauri/binaries"), { recursive: true });
await mkdir(join(root, ".build"), { recursive: true });

const rust = spawnSync("rustc", ["-vV"], { encoding: "utf8" });
if (rust.status !== 0) throw new Error("Rust toolchain is required.");
const triple = rust.stdout.match(/^host: (.+)$/m)[1].trim();
const ext = process.platform === "win32" ? ".exe" : "";

run("cargo", [
  "build",
  "--manifest-path",
  join(root, "src-tauri/Cargo.toml"),
  "--bin",
  "masterdata",
  "--locked",
]);

const builtCli = join(root, "src-tauri/target/debug", `masterdata${ext}`);
const binary = join(root, "src-tauri/binaries", `masterdata-host-${triple}${ext}`);

const nodeVersion = JSON.parse(
  await readFile(join(root, "node_modules/node/package.json"), "utf8"),
).version;
await copyFile(
  join(root, "assets", `node-${nodeVersion}-LICENSE.txt`),
  join(root, ".build/node-license.txt"),
);

await rm(binary, { force: true });
await copyFile(builtCli, binary);
await chmod(binary, 0o755);

// cargo run/dev expects the unsuffixed sibling, bundle takes the target-suffixed input.
for (const profile of ["debug", "release"]) {
  await mkdir(join(root, "src-tauri/target", profile), { recursive: true });
  await copyFile(
    binary,
    join(root, "src-tauri/target", profile, `masterdata-host${ext}`),
  );
}
console.log(`Bundled Rust CLI / host: ${binary}`);
