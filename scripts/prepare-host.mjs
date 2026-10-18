import { build } from "esbuild";
import { inject } from "postject";
import {
  mkdir,
  writeFile,
  copyFile,
  readFile,
  chmod,
  rm,
  readdir,
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
const bundled = join(root, ".build/host.cjs");
await build({
  entryPoints: [join(root, "desktop/host.ts")],
  bundle: true,
  platform: "node",
  format: "cjs",
  outfile: bundled,
  target: "node22",
});
const rust = spawnSync("rustc", ["-vV"], { encoding: "utf8" });
if (rust.status !== 0) throw new Error("Rust toolchain is required.");
const triple = rust.stdout.match(/^host: (.+)$/m)[1].trim();
const binary = join(
  root,
  "src-tauri/binaries",
  `masterdata-host-${triple}${process.platform === "win32" ? ".exe" : ""}`,
);
const seaNode = join(
  root,
  "node_modules/node/bin",
  process.platform === "win32" ? "node.exe" : "node",
);
const runtimePackages = join(root, "node_modules/node/node_modules");
const runtimePackage = (await readdir(runtimePackages)).find(
  (name) => name.startsWith("node-") && name !== "node-bin-setup",
);
if (!runtimePackage) throw new Error("Bundled Node license package not found");
await copyFile(
  join(runtimePackages, runtimePackage, "LICENSE"),
  join(root, ".build/node-license.txt"),
);
const config = join(root, ".build/sea.json");
const blob = join(root, ".build/sea.blob");
await writeFile(
  config,
  JSON.stringify({
    main: bundled,
    output: blob,
    disableExperimentalSEAWarning: true,
    useSnapshot: false,
    useCodeCache: false,
    execArgvExtension: "none",
  }),
);
run(seaNode, ["--experimental-sea-config", config]);
await rm(binary, { force: true });
await copyFile(seaNode, binary);
await chmod(binary, 0o755);
if (process.platform === "darwin")
  run("codesign", ["--remove-signature", binary]);
await inject(binary, "NODE_SEA_BLOB", await readFile(blob), {
  sentinelFuse: "NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2",
  ...(process.platform === "darwin" ? { machoSegmentName: "NODE_SEA" } : {}),
});
await chmod(binary, 0o755);
if (process.platform === "darwin") run("codesign", ["--sign", "-", binary]);
// cargo run/dev expects the unsuffixed sibling, bundle takes the target-suffixed input.
for (const profile of ["debug", "release"]) {
  await mkdir(join(root, "src-tauri/target", profile), { recursive: true });
  await copyFile(
    binary,
    join(
      root,
      "src-tauri/target",
      profile,
      `masterdata-host${process.platform === "win32" ? ".exe" : ""}`,
    ),
  );
}
console.log(`Bundled private host: ${binary}`);
