import {
  test,
  expect,
  _electron as electron,
  type ElectronApplication,
  type Page,
} from "@playwright/test";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { parse, stringify } from "yaml";
let app: ElectronApplication;
let page: Page;
let root: string;
test.beforeEach(async ({}, info) => {
  root = await fs.mkdtemp(path.join(os.tmpdir(), "masterdata-e2e-"));
  await fs.cp(path.resolve("samples/Adventure"), root, { recursive: true });
  if (info.title.startsWith("large")) {
    const file = path.join(root, "sources/announcements/announcement.yaml");
    const data = parse(await fs.readFile(file, "utf8"));
    for (let i = 0; i < 60; i++)
      data.fields.push({ key: i + 2, name: `field${i}`, type: "string" });
    data.records = Array.from({ length: 400 }, (_, i) => ({
      id: i + 1,
      message: `レコード${i + 1}`,
      ...Object.fromEntries(
        Array.from({ length: 60 }, (_, n) => [`field${n}`, `value-${i}-${n}`]),
      ),
    }));
    await fs.writeFile(file, stringify(data));
    await fs.mkdir(path.join(root, "sources/many"));
    for (let i = 0; i < 600; i++)
      await fs.writeFile(
        path.join(root, `sources/many/type-${i}.yaml`),
        `kind: type\nname: Extra${i}\ncustom:\n  fields: []\n`,
      );
  }
  app = await electron.launch({
    args: [path.resolve("desktop/main.cjs"), "--project", root],
    env: { ...process.env, ELECTRON_DISABLE_SECURITY_WARNINGS: "true" },
  });
  page = await app.firstWindow();
  await expect(
    page.getByText("冒険の道具箱", { exact: true }).first(),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: /0 件の問題/ })).toBeVisible();
});
test.afterEach(async () => {
  if (app) {
    await page.evaluate(() => window.desktop.dirty(false)).catch(() => {});
    await app.close();
  }
  await fs.rm(root, { recursive: true, force: true });
});
async function selectSource(name: string) {
  const search = page.getByLabel("表・型を検索");
  await search.fill(name);
  await page
    .locator(".tree-name")
    .filter({ hasText: new RegExp(`^${name}$`) })
    .click();
  await search.fill("");
}
test("Enum adds inline with focused name; Save keeps Undo and values persist", async () => {
  await selectSource("rarity");
  await page.getByRole("button", { name: "候補を追加" }).click();
  const name = page.getByLabel("候補4の名前");
  await expect(name).toBeFocused();
  await name.fill("Legendary");
  await name.press("Enter");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("保存済み", { exact: true })).toBeVisible();
  const saved = await fs.readFile(
    path.join(root, "sources/types/rarity.yaml"),
    "utf8",
  );
  expect(saved).toContain("Legendary");
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(name).toHaveValue("Member1");
  await expect(page.getByText("未保存の変更", { exact: true })).toBeVisible();
});
test("Custom Type adds fields inline and uses the same modifiers", async () => {
  await selectSource("combat-stats");
  await page.getByRole("button", { name: "フィールドを追加" }).click();
  const name = page.getByLabel("項目5の名前");
  await expect(name).toBeFocused();
  await name.fill("luck");
  await name.press("Enter");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "配列", exact: true }),
  ).toHaveCount(5);
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("保存済み", { exact: true })).toBeVisible();
  expect(
    await fs.readFile(path.join(root, "sources/items/common.yaml"), "utf8"),
  ).toContain("luck");
});
test("top-level and nested bool share controls; nested nullable can be cleared", async () => {
  await selectSource("common");
  const top = page.getByRole("switch", { name: "enabled 行1", exact: true });
  await expect(top).toBeChecked();
  await page
    .getByRole("button", { name: "stats 行1を開く", exact: true })
    .click();
  const nested = page.getByRole("switch", { name: "enabled", exact: true });
  await expect(nested).toBeChecked();
  expect(await top.getAttribute("class")).toBe(
    await nested.getAttribute("class"),
  );
  await page
    .getByRole("button", { name: "criticalに値を作成", exact: true })
    .click();
  await expect(
    page.getByRole("switch", { name: "critical", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "criticalをnullにする", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "criticalに値を作成", exact: true }),
  ).toBeVisible();
});
test("type mismatch retains original value, does not block edits or Save, and can Undo", async () => {
  await selectSource("common");
  const select = page.locator(".column-type").nth(4).getByRole("combobox");
  await select.fill("int");
  await select.press("Enter");
  await expect(page.getByLabel("enabled 行1", { exact: true })).toHaveValue(
    "true",
  );
  await page.getByLabel("name 行1", { exact: true }).fill("不整合中も編集可能");
  await page.getByLabel("name 行1", { exact: true }).press("Enter");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("保存済み", { exact: true })).toBeVisible();
  expect(
    await fs.readFile(path.join(root, "sources/items/common.yaml"), "utf8"),
  ).toContain("enabled: true");
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(
    page.getByRole("switch", { name: "enabled 行1", exact: true }),
  ).toBeChecked();
});
test("deep array navigation retains the whole breadcrumb and allows value repair", async () => {
  await selectSource("common");
  await page
    .getByRole("button", { name: "effects 行1を開く", exact: true })
    .click();
  await page.getByRole("button", { name: "要素1を開く", exact: true }).click();
  await page.getByRole("button", { name: "statsを開く", exact: true }).click();
  await expect(page.locator(".breadcrumb-scroll")).toContainText("effects");
  await expect(page.locator(".breadcrumb-scroll")).toContainText("[0]");
  await expect(page.locator(".breadcrumb-scroll")).toContainText("stats");
  await expect(
    page.getByRole("switch", { name: "enabled", exact: true }),
  ).toBeVisible();
});
test("folder creation, rename and deletion are inline, reversible, and saved", async () => {
  await page.getByRole("button", { name: "新しい項目", exact: true }).click();
  await page.getByRole("menuitem", { name: /新しいフォルダ/ }).click();
  const name = page.getByLabel("項目の名前");
  await name.fill("sandbox");
  await name.press("Enter");
  const folder = page.locator(".tree-name").filter({ hasText: /^sandbox$/ });
  await expect(folder).toBeVisible();
  await folder.click({ button: "right" });
  await page.getByRole("menuitem", { name: /削除/ }).click();
  await expect(folder).toHaveCount(0);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(folder).toBeVisible();
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("保存済み", { exact: true })).toBeVisible();
  expect(
    (
      await fs.stat(path.join(root, "sources/announcements/sandbox"))
    ).isDirectory(),
  ).toBe(true);
});
test("drag reorder retains visible rows and TSV paste is a single Undo", async () => {
  const handles = page.getByLabel("レコードを移動", { exact: true });
  const dataTransfer = await page.evaluateHandle(() => new DataTransfer());
  await handles.first().dispatchEvent("dragstart", { dataTransfer });
  await page
    .locator("[data-row-key]")
    .nth(2)
    .dispatchEvent("drop", { dataTransfer });
  await expect(page.getByLabel("id 行3", { exact: true })).toHaveValue("1");
  await expect(page.locator("[data-row-key]")).toHaveCount(3);
  await page.getByLabel("id 行1", { exact: true }).click();
  await page.getByLabel("id 行1", { exact: true }).evaluate((element) => {
    const clipboardData = new DataTransfer();
    clipboardData.setData("text/plain", "42\t貼り付けた値");
    element.dispatchEvent(
      new ClipboardEvent("paste", {
        clipboardData,
        bubbles: true,
        cancelable: true,
      }),
    );
  });
  await expect(page.getByLabel("id 行1", { exact: true })).toHaveValue("42");
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "貼り付けた値",
  );
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("id 行1", { exact: true })).toHaveValue("2");
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "開発環境のお知らせ",
  );
});
test("large folder toggles perform no discovery I/O; large table remains virtual with Problems open", async () => {
  const folder = page
    .locator(".tree-name")
    .filter({ hasText: /^many$/ })
    .locator('xpath=ancestor::*[@role="treeitem"]');
  await app.evaluate(() => {
    const fsp = process.getBuiltinModule("fs/promises") as any;
    (globalThis as any).readCalls = 0;
    for (const key of ["readdir", "readFile"]) {
      const original = fsp[key];
      fsp[key] = (...args: any[]) => {
        (globalThis as any).readCalls++;
        return original(...args);
      };
    }
  });
  await folder.locator(".ant-tree-switcher").click();
  await expect(
    page.locator(".tree-name").filter({ hasText: /^type-0$/ }),
  ).toBeVisible();
  await folder.locator(".ant-tree-switcher").click();
  await folder.locator(".ant-tree-switcher").click();
  expect(await app.evaluate(() => (globalThis as any).readCalls)).toBe(0);
  expect(await page.locator("[data-row-key]").count()).toBeLessThan(100);
  await page.getByRole("button", { name: /件の問題/ }).click();
  await expect(page.locator(".problems-panel")).toBeVisible();
  const viewport = page.locator(".grid-viewport");
  const panel = page.locator(".problems-panel");
  const vp = await viewport.boundingBox();
  const pp = await panel.boundingBox();
  expect(vp!.y + vp!.height).toBeLessThanOrEqual(pp!.y + 1);
  await page.getByLabel("id 行1", { exact: true }).click();
  await page.getByLabel("id 行1", { exact: true }).press("Escape");
  await page.locator(".grid-editor").focus();
  await page.locator(".grid-editor").press("Control+End");
  await expect(page.getByLabel("id 行400", { exact: true })).toBeAttached();
});
