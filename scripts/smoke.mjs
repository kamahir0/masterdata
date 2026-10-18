import { resolve } from "node:path";
import { smoke } from "./local-app.mjs";
await smoke(
  resolve(
    `src-tauri/target/release/masterdata-desktop${process.platform === "win32" ? ".exe" : ""}`,
  ),
);
console.log("Tauri webview / frontend / bundled host startup: PASS");
