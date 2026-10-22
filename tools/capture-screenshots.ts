import { spawn } from "node:child_process";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { chromium } from "@playwright/test";
import { ProjectService } from "../desktop/service";
import { encode, decode } from "../src/core/wire";

const root = path.resolve(import.meta.dirname, "..");
const tmp = await fs.mkdtemp(path.join(os.tmpdir(), "masterdata-screen-"));
await fs.cp(path.join(root, "samples/Adventure"), tmp, { recursive: true });

// Start preview server
const preview = spawn(
  "npx",
  ["vite", "preview", "--host", "127.0.0.1", "--port", "5174", "--strictPort"],
  { cwd: root, stdio: "inherit" },
);

// Wait for preview server to be ready
await new Promise((resolve) => setTimeout(resolve, 2500));

try {
  const service = new ProjectService();
  const dialogPath = path.join(tmp, "masterdata.toml");

  const browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 2,
  });

  const page = await context.newPage();

  await page.exposeFunction("__native", async (method: string, payload: string) => {
    let args = decode<any[]>(payload);
    if (method === "initial") args = [dialogPath];
    if (method === "open") args = [dialogPath];
    if (method === "create") args = [args[0], tmp];
    if (method === "sample") args = [path.join(root, "samples/Adventure"), path.join(tmp, ".user-data")];
    return encode(await service.request(method, args));
  });

  await page.addInitScript(`(() => {
    const encode = ${encode.toString()};
    const decode = ${decode.toString()};
    const request = async (method, args = []) => decode(await window.__native(method, encode(args)));
    window.desktop = {
      initial: () => request("initial"),
      open: () => request("open"),
      create: name => request("create", [name]),
      sample: () => request("sample"),
      save: value => request("save", [value]),
      build: (...args) => request("build", args),
      publish: (...args) => request("publish", args),
      reveal: async () => {},
      dirty: () => {},
      close: async () => {},
      onCommand: callback => { window.__command = callback; return () => {}; },
    };
  })()`);

  // 1. Light theme
  await page.goto("http://127.0.0.1:5174");
  await page.waitForSelector("text=冒険の道具箱");

  // Select Item table if available
  const itemNav = page.getByRole("treeitem", { name: /Item/i }).or(page.getByText(/Item/i)).first();
  if (await itemNav.isVisible()) {
    await itemNav.click();
    await page.waitForTimeout(500);
  }

  // Ensure light theme
  await page.evaluate(() => {
    localStorage.setItem("masterdata.editor-preferences", JSON.stringify({ theme: "light" }));
    document.documentElement.dataset.theme = "light";
  });
  await page.waitForTimeout(600);

  const lightPath = path.join(root, "docs/images/screenshot-light.png");
  await page.screenshot({ path: lightPath });
  console.log(`Saved: ${lightPath}`);

  // 2. Dark theme
  await page.evaluate(() => {
    localStorage.setItem("masterdata.editor-preferences", JSON.stringify({ theme: "dark" }));
    document.documentElement.dataset.theme = "dark";
    window.dispatchEvent(new Event("storage"));
  });
  // Also reload with dark preference in storage
  await page.goto("http://127.0.0.1:5174");
  await page.waitForSelector("text=冒険の道具箱");
  if (await itemNav.isVisible()) {
    await itemNav.click();
    await page.waitForTimeout(500);
  }
  await page.evaluate(() => {
    document.documentElement.dataset.theme = "dark";
  });
  await page.waitForTimeout(600);

  const darkPath = path.join(root, "docs/images/screenshot-dark.png");
  await page.screenshot({ path: darkPath });
  console.log(`Saved: ${darkPath}`);

  await browser.close();
} finally {
  preview.kill("SIGTERM");
  await fs.rm(tmp, { recursive: true, force: true });
}
