import { test, expect, type Page } from "@playwright/test";
import fs from "node:fs/promises";
import path from "node:path";

let seed: { path: string; text: string }[];
test.beforeAll(async () => {
  seed = [];
  async function walk(relative: string) {
    for (const entry of await fs.readdir(
      path.join("samples/Adventure", relative),
      { withFileTypes: true },
    )) {
      const name = [relative, entry.name].filter(Boolean).join("/");
      if (entry.isDirectory()) await walk(name);
      else if (name === "masterdata.toml" || /\.ya?ml$/.test(name))
        seed.push({
          path: name,
          text: await fs.readFile(path.join("samples/Adventure", name), "utf8"),
        });
    }
  }
  await walk("");
});
test.beforeEach(async ({ page, browserName, context }, info) => {
  if (info.title.startsWith("browser unsupported")) {
    await context.addInitScript(() => {
      delete window.showDirectoryPicker;
    });
    await page.goto("http://127.0.0.1:5173");
    return;
  }
  test.skip(
    browserName !== "chromium",
    "Writing requires File System Access; unsupported UI is checked separately.",
  );
  // Only the picker is substituted. IO uses Chromium's real persistent directory,
  // file handles and writable streams; no DesktopAPI / native backend bridge.
  await context.addInitScript(
    ({ files }) => {
      const control = ((window as any).__browserTest = {
        calls: 0,
        denied: false,
        mode: "normal",
        closed: 0,
        crashed: false,
        release: null as null | (() => void),
      });
      for (const name of ["queryPermission", "requestPermission"]) {
        const original = (FileSystemHandle.prototype as any)[name];
        (FileSystemHandle.prototype as any)[name] = function (...args: any[]) {
          return control.denied
            ? Promise.resolve("denied")
            : original.apply(this, args);
        };
      }
      const record = () => {
        control.calls++;
        if (control.crashed)
          throw new DOMException("permission revoked", "NotAllowedError");
      };
      for (const [prototype, names] of [
        [
          FileSystemDirectoryHandle.prototype,
          ["getDirectoryHandle", "getFileHandle", "removeEntry", "values"],
        ],
        [FileSystemFileHandle.prototype, ["getFile", "createWritable"]],
      ] as const)
        for (const name of names) {
          const original = (prototype as any)[name];
          (prototype as any)[name] = function (...args: any[]) {
            record();
            return original.apply(this, args);
          };
        }
      const original = FileSystemFileHandle.prototype.createWritable;
      FileSystemFileHandle.prototype.createWritable = async function (...args) {
        const stream = await original.apply(this, args);
        const close = stream.close.bind(stream);
        const filename = this.name;
        stream.close = async () => {
          if (!filename.endsWith(".json")) {
            if (control.mode === "gate")
              await new Promise<void>((resolve) => {
                control.release = resolve;
              });
            if (control.mode === "crash" && control.closed++ >= 1) {
              control.crashed = true;
              throw new DOMException("permission revoked", "NotAllowedError");
            }
          }
          await close();
        };
        return stream;
      };
      (window as any).showDirectoryPicker = async () => {
        if (control.mode === "cancel")
          throw new DOMException("cancel", "AbortError");
        if (control.mode === "deny")
          throw new DOMException("denied", "NotAllowedError");
        const root = await navigator.storage.getDirectory();
        if (control.mode === "other")
          return root.getDirectoryHandle("another-project", { create: true });
        if (control.mode === "grant") control.denied = false;
        try {
          await root.getDirectoryHandle("sources");
          return root;
        } catch {}
        for (const file of files) {
          const names = file.path.split("/");
          const name = names.pop()!;
          let dir = root;
          for (const folder of names)
            dir = await dir.getDirectoryHandle(folder, { create: true });
          const stream = await (
            await dir.getFileHandle(name, { create: true })
          ).createWritable();
          await stream.write(file.text);
          await stream.close();
        }
        return root;
      };
    },
    { files: seed },
  );
  await page.goto("http://127.0.0.1:5173");
});
async function open(page: Page) {
  const table = page.getByLabel("message 行1", { exact: true });
  const button = page.getByRole("button", { name: /プロジェクトを開く/ });
  const first = await Promise.race([
    table
      .waitFor({ state: "visible", timeout: 10000 })
      .then(() => "table" as const),
    button
      .waitFor({ state: "visible", timeout: 10000 })
      .then(() => "button" as const),
  ]);
  if (first === "button") {
    await button.click();
  }
  await expect(table).toBeVisible();
}
async function read(page: Page, name: string) {
  return page.evaluate(async (path) => {
    const names = path.split("/"),
      name = names.pop()!;
    let dir = await navigator.storage.getDirectory();
    for (const folder of names) dir = await dir.getDirectoryHandle(folder);
    return (await (await dir.getFileHandle(name)).getFile()).text();
  }, name);
}
async function saveAll(page: Page) {
  await page.getByRole("button", { name: "保存方法", exact: true }).click();
  await page.getByRole("menuitem", { name: "すべて保存", exact: true }).click();
}
async function select(page: Page, name: string) {
  await page.getByLabel("表・型を検索").fill(name);
  await page
    .locator(".tree-name")
    .filter({ hasText: new RegExp(`^${name}$`) })
    .click();
  await page.getByLabel("表・型を検索").fill("");
}
test("browser static UI opens without native bridge, edits in memory, hides generation and saves compatible text", async ({
  page,
}) => {
  await expect(
    page.getByRole("button", { name: "新しく作る", exact: true }),
  ).toHaveCount(0);
  await open(page);
  await expect(
    page.getByRole("button", { name: "Build", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByLabel("出力プロファイル", { exact: true }),
  ).toHaveCount(0);
  const config = await read(page, "masterdata.toml");
  const before = await read(page, "sources/announcements/announcement.yaml");
  const calls = await page.evaluate(() => (window as any).__browserTest.calls);
  await page.getByLabel("message 行1", { exact: true }).fill("Browser保存");
  await page.getByLabel("message 行1", { exact: true }).press("Tab");
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await page.getByRole("button", { name: "やり直す", exact: true }).click();
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "Browser保存",
  );
  expect(await page.evaluate(() => (window as any).__browserTest.calls)).toBe(
    calls,
  );
  expect(await read(page, "sources/announcements/announcement.yaml")).toBe(
    before,
  );
  await page
    .getByRole("button", { name: "プロジェクト設定", exact: true })
    .click();
  await expect(page.getByText("Publishの出力先", { exact: true })).toHaveCount(
    0,
  );
  await expect(page.getByText("出力プロファイル", { exact: true })).toHaveCount(
    0,
  );
  await page
    .getByRole("textbox", { name: "プロジェクト名", exact: true })
    .fill("Browser Project");
  await page
    .getByRole("textbox", { name: "プロジェクト名", exact: true })
    .press("Tab");
  await page.getByRole("button", { name: "閉じる", exact: true }).click();
  await page.keyboard.press("ControlOrMeta+s");
  await expect
    .poll(() => read(page, "sources/announcements/announcement.yaml"))
    .toContain("Browser保存");
  expect(await read(page, "masterdata.toml")).toBe(config);
  await expect(page.locator(".save-state")).toHaveAttribute(
    "aria-label",
    "未保存の変更",
  );
  await saveAll(page);
  await expect(page.locator(".save-state")).toHaveAttribute(
    "aria-label",
    "保存済み",
  );
  expect(await read(page, "masterdata.toml")).toContain("Browser Project");
  await page.screenshot({ path: ".build/browser-editor.png" });
});
test("browser picker cancellation and rejected opens preserve the existing draft", async ({
  page,
}) => {
  await open(page);
  await page.getByLabel("message 行1", { exact: true }).fill("保持するdraft");
  await page.getByLabel("message 行1", { exact: true }).press("Tab");
  for (const mode of ["cancel", "deny"]) {
    await page.evaluate((mode) => {
      (window as any).__browserTest.mode = mode;
    }, mode);
    await page.getByRole("button", { name: /開く$/, exact: false }).click();
    await page
      .getByRole("button", { name: "破棄して続ける", exact: true })
      .click();
    await expect(page.locator(".ant-modal-wrap")).toBeHidden();
    await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
      "保持するdraft",
    );
    await expect(page.locator(".save-state")).toHaveAttribute(
      "aria-label",
      "未保存の変更",
    );
  }
});
test("browser edit during Save retains newer dirty input", async ({ page }) => {
  await open(page);
  await page.getByLabel("message 行1", { exact: true }).fill("captured");
  await page.getByLabel("message 行1", { exact: true }).press("Tab");
  await page.evaluate(() => {
    (window as any).__browserTest.mode = "gate";
  });
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect
    .poll(() => page.evaluate(() => !!(window as any).__browserTest.release))
    .toBe(true);
  await page.getByLabel("message 行1", { exact: true }).fill("newer");
  await page.getByLabel("message 行1", { exact: true }).press("Tab");
  await page.evaluate(() => {
    (window as any).__browserTest.mode = "normal";
    (window as any).__browserTest.release();
  });
  await expect
    .poll(() => read(page, "sources/announcements/announcement.yaml"))
    .toContain("captured");
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "newer",
  );
  await expect(page.locator(".save-state")).toHaveAttribute(
    "aria-label",
    "未保存の変更",
  );
  await saveAll(page);
  await expect(page.locator(".save-state")).toHaveAttribute(
    "aria-label",
    "保存済み",
  );
});
test("browser multi-tab conflict does not overwrite a newer saved file", async ({
  page,
  context,
}) => {
  await open(page);
  const other = await context.newPage();
  await other.goto("http://127.0.0.1:5173");
  await open(other);
  await page.getByLabel("message 行1", { exact: true }).fill("tab one");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.locator(".save-state")).toHaveAttribute(
    "aria-label",
    "保存済み",
  );
  await other.getByLabel("message 行1", { exact: true }).fill("tab two");
  await other.getByRole("button", { name: "保存", exact: true }).click();
  await expect(
    other.getByText(/内容を確認できません: sources\/announcements/),
  ).toBeVisible();
  await expect(other.locator(".save-state")).toHaveAttribute(
    "aria-label",
    "未保存の変更",
  );
  expect(await read(page, "sources/announcements/announcement.yaml")).toContain(
    "tab one",
  );
});
test("browser interrupted multi-file save survives page reload and restores verified originals through recovery UI", async ({
  page,
}) => {
  await open(page);
  const original = await read(page, "sources/announcements/announcement.yaml");
  await page.getByLabel("message 行1", { exact: true }).fill("partial save");
  await page.getByLabel("message 行1", { exact: true }).press("Tab");
  await select(page, "rarity");
  await page.getByRole("button", { name: /候補を追加$/, exact: false }).click();
  await page.evaluate(() => {
    (window as any).__browserTest.mode = "crash";
  });
  await saveAll(page);
  await expect(
    page.getByRole("dialog", { name: "中断した保存の復旧", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "保存前へ復旧", exact: true }),
  ).toBeVisible();
  let warned = false;
  page.once("dialog", async (dialog) => {
    warned = dialog.type() === "beforeunload";
    await dialog.accept();
  });
  await page.reload();
  expect(warned).toBe(true);
  const dialog = page.getByRole("dialog", {
    name: "中断した保存の復旧",
    exact: true,
  });
  const openButton = page.getByRole("button", { name: /プロジェクトを開く/ });
  const first = await Promise.race([
    dialog
      .waitFor({ state: "visible", timeout: 10000 })
      .then(() => "dialog" as const),
    openButton
      .waitFor({ state: "visible", timeout: 10000 })
      .then(() => "button" as const),
  ]);
  if (first === "button") {
    await openButton.click();
  }
  await expect(dialog).toBeVisible();
  await expect(
    page.getByText("保存予定", { exact: false }).first(),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "保存済み内容を採用", exact: true }),
  ).toHaveCount(0);
  const download = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "復旧JSONをダウンロード", exact: true })
    .click();
  const downloadPath = await (await download).path();
  expect(JSON.parse(await fs.readFile(downloadPath!, "utf8"))).toBeTruthy();
  await page.getByRole("button", { name: "保存前へ復旧", exact: true }).click();
  await expect(
    page.getByRole("dialog", { name: "中断した保存の復旧", exact: true }),
  ).toHaveCount(0);
  expect(await read(page, "sources/announcements/announcement.yaml")).toBe(
    original,
  );
  await open(page);
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "冒険へようこそ",
  );
});

test("browser live recovery can be dismissed, keeps newer input and Undo, and allows resaving without reopening", async ({
  page,
}) => {
  await open(page);
  await page.getByLabel("message 行1", { exact: true }).fill("captured");
  await page.getByLabel("message 行1", { exact: true }).press("Tab");
  await select(page, "rarity");
  await page.getByRole("button", { name: /候補を追加$/ }).click();
  await page.evaluate(() => {
    (window as any).__browserTest.mode = "crash";
  });
  await saveAll(page);
  const dialog = page.getByRole("dialog", {
    name: "中断した保存の復旧",
    exact: true,
  });
  await expect(dialog).toBeVisible();
  await dialog.locator(".ant-modal-close").click();
  await select(page, "announcement");
  await page
    .getByLabel("message 行1", { exact: true })
    .fill("newer retained draft");
  await page.getByLabel("message 行1", { exact: true }).press("Tab");
  await page.evaluate(() => {
    (window as any).__browserTest.mode = "normal";
    (window as any).__browserTest.crashed = false;
  });
  await page
    .getByRole("button", { name: "復旧情報を開く", exact: true })
    .click();
  await page.getByRole("button", { name: "保存前へ復旧", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "newer retained draft",
  );
  await expect(page.locator(".save-state")).toHaveAttribute(
    "aria-label",
    "未保存の変更",
  );
  await saveAll(page);
  await expect(page.locator(".save-state")).toHaveAttribute(
    "aria-label",
    "保存済み",
  );
  expect(await read(page, "sources/announcements/announcement.yaml")).toContain(
    "newer retained draft",
  );
  expect(await read(page, "sources/types/rarity.yaml")).toContain("Member1");
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "captured",
  );
});

test("browser unsupported environment shows a clear explanation and no enabled picker", async ({
  page,
}) => {
  await expect(
    page.getByRole("button", { name: /プロジェクトを開く/ }),
  ).toBeDisabled();
  await expect(page.getByText(/この環境では編集できません/)).toBeVisible();
  await expect(
    page.getByRole("button", { name: /新しく作る|Build|Publish/ }),
  ).toHaveCount(0);
});

test("browser invalid or missing project config preserves prior edits and identifies the bad file", async ({
  page,
}) => {
  await open(page);
  const config = await read(page, "masterdata.toml");
  await page.getByLabel("message 行1", { exact: true }).fill("retained");
  await page.getByLabel("message 行1", { exact: true }).press("Tab");
  for (const text of [null, "name = [", 'name = "x"\nprofiles = []']) {
    await page.evaluate(async (text) => {
      const root = await navigator.storage.getDirectory();
      if (text === null) await root.removeEntry("masterdata.toml");
      else {
        const stream = await (
          await root.getFileHandle("masterdata.toml", { create: true })
        ).createWritable();
        await stream.write(text);
        await stream.close();
      }
    }, text);
    await page.getByRole("button", { name: /開く$/ }).click();
    await page
      .getByRole("button", { name: "破棄して続ける", exact: true })
      .click();
    await expect(page.locator(".ant-modal-wrap")).toBeHidden();
    await expect(
      page.getByText(/masterdata.toml.*(?:ありません|Error)/).last(),
    ).toBeVisible();
    await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
      "retained",
    );
  }
  await page.evaluate(async (text) => {
    const root = await navigator.storage.getDirectory();
    const stream = await (
      await root.getFileHandle("masterdata.toml")
    ).createWritable();
    await stream.write(text);
    await stream.close();
  }, config);
});

test("browser raw syntax errors save without a build gate and can be repaired through the shared source editor", async ({
  page,
}) => {
  await open(page);
  const original = await read(page, "sources/announcements/announcement.yaml");
  await page.getByRole("tab", { name: /ソース/ }).click();
  const raw = "kind: schema\ntable: Broken\nfields: [\n";
  await page.getByLabel("YAMLソース", { exact: true }).fill(raw);
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.locator(".save-state")).toHaveAttribute(
    "aria-label",
    "保存済み",
  );
  expect(await read(page, "sources/announcements/announcement.yaml")).toBe(raw);
  await page
    .getByLabel("YAMLソース", { exact: true })
    .fill(original + "\n# Browser repair\n");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.locator(".save-state")).toHaveAttribute(
    "aria-label",
    "保存済み",
  );
  await page.getByRole("tab", { name: "データ", exact: true }).click();
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "冒険へようこそ",
  );
  expect(await read(page, "sources/announcements/announcement.yaml")).toContain(
    "# Browser repair",
  );
});

test("browser explicit reauthorization preserves drafts, rejects another folder and does not implicitly save", async ({
  page,
}) => {
  await open(page);
  const original = await read(page, "sources/announcements/announcement.yaml");
  await page
    .getByLabel("message 行1", { exact: true })
    .fill("permission draft");
  await page.getByLabel("message 行1", { exact: true }).press("Tab");
  await page.evaluate(() => {
    (window as any).__browserTest.denied = true;
  });
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText(/読み書きが許可されていません/)).toBeVisible();
  expect(await read(page, "sources/announcements/announcement.yaml")).toBe(
    original,
  );
  for (const mode of ["cancel", "other", "grant"]) {
    await page.evaluate((mode) => {
      (window as any).__browserTest.mode = mode;
    }, mode);
    await page.getByRole("button", { name: "保存方法", exact: true }).click();
    await page
      .getByRole("menuitem", {
        name: "フォルダのアクセスを再許可",
        exact: true,
      })
      .click();
    if (mode === "other")
      await expect(page.getByText(/元と同じフォルダ/)).toBeVisible();
    if (mode === "grant")
      await expect(
        page.getByText(/フォルダへのアクセスを許可しました/),
      ).toBeVisible();
    await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
      "permission draft",
    );
    await expect(page.locator(".save-state")).toHaveAttribute(
      "aria-label",
      "未保存の変更",
    );
    expect(await read(page, "sources/announcements/announcement.yaml")).toBe(
      original,
    );
  }
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.locator(".save-state")).toHaveAttribute(
    "aria-label",
    "保存済み",
  );
  expect(await read(page, "sources/announcements/announcement.yaml")).toContain(
    "permission draft",
  );
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "冒険へようこそ",
  );
});

test("browser reopens recent project automatically on reload when permission is granted", async ({
  page,
}) => {
  await open(page);
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "冒険へようこそ",
  );
  await page.reload();
  await expect(page.getByLabel("message 行1", { exact: true })).toBeVisible();
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "冒険へようこそ",
  );
});

test("browser unsupported welcome identifies the build version and serves its favicon", async ({ page }) => {
  const { version } = JSON.parse(await fs.readFile("package.json", "utf8"));
  const label = page.getByLabel("アプリのバージョン", { exact: true });
  await expect(label).toBeVisible();
  await expect(label).toHaveText(`v${version}`);
  expect(await label.evaluate(el => parseFloat(getComputedStyle(el).fontSize))).toBeLessThan(14);
  const icon = await page.locator('link[rel="icon"]').evaluate(el => (el as HTMLLinkElement).href);
  const response = await page.request.get(icon);
  expect(response.ok()).toBe(true);
  expect(response.headers()["content-type"]).toContain("image/svg+xml");
  expect(await response.text()).toContain("<svg");
});
