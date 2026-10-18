import { chromium } from "@playwright/test";
import { readFile, mkdir, copyFile } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

// One scalable master supplies the in-app mark and every native icon size.
const root = fileURLToPath(new URL("../", import.meta.url));
const browser = await chromium.launch();
try {
  const page = await browser.newPage({
    viewport: { width: 2048, height: 2048 },
  });
  const svg = await readFile(path.join(root, "assets/icon.svg"), "utf8");
  await page.setContent(
    `<style>html,body{margin:0;width:2048px;height:2048px}svg{width:100%;height:100%;display:block}</style>${svg}`,
  );
  await page.screenshot({
    path: path.join(root, "assets/icon.png"),
    omitBackground: true,
  });
} finally {
  await browser.close();
}
const output = path.join(root, ".build/icons");
await mkdir(output, { recursive: true });
execFileSync(
  process.execPath,
  [
    path.join(root, "node_modules/@tauri-apps/cli/tauri.js"),
    "icon",
    path.join(root, "assets/icon.png"),
    "--output",
    output,
  ],
  { cwd: root, stdio: "inherit" },
);
for (const ext of ["icns", "ico"])
  await copyFile(
    path.join(output, `icon.${ext}`),
    path.join(root, `assets/icon.${ext}`),
  );
