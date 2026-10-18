import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
const rust = spawnSync("rustc", ["-vV"], { encoding: "utf8" });
if (rust.status !== 0) throw new Error("rustc is required");
const triple = rust.stdout.match(/^host: (.+)$/m)[1].trim();
const result = spawnSync(
  "cargo",
  ["test", "--manifest-path", "src-tauri/Cargo.toml", "--locked"],
  {
    stdio: "inherit",
    timeout: 600000,
    env: {
      ...process.env,
      MASTERDATA_HOST_TEST: resolve(
        `src-tauri/binaries/masterdata-host-${triple}${process.platform === "win32" ? ".exe" : ""}`,
      ),
    },
  },
);
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
