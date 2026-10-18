import { test, expect, type Page } from "@playwright/test";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { parse, stringify } from "yaml";
import { ProjectService } from "../desktop/service";
import { encode, decode } from "../src/core/wire";
let service: ProjectService;
let calls: string[];
let dialogPath: string;
let page: Page;
let root: string;
let saveGate: Promise<void> | undefined;
test.beforeEach(async ({ browser }, info) => {
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
  if (info.title.startsWith("diagnostic")) {
    const file = path.join(root, "sources/announcements/announcement.yaml");
    const data = parse(await fs.readFile(file, "utf8"));
    data.records[0].id = "broken";
    await fs.writeFile(file, stringify(data));
  }
  saveGate = undefined;
  service = new ProjectService();
  calls = [];
  dialogPath = root;
  page = await browser.newPage({ viewport: { width: 1480, height: 960 } });
  await page.exposeFunction(
    "__native",
    async (method: string, payload: string) => {
      calls.push(method);
      if (method === "save") await saveGate;
      let args = decode<any[]>(payload);
      if (method === "initial")
        args = info.title.startsWith("welcome") ? [] : [root];
      if (method === "open") args = [dialogPath];
      if (method === "create") args = [args[0], dialogPath];
      if (method === "sample")
        args = [
          path.resolve("samples/Adventure"),
          path.join(root, ".user-data"),
        ];
      return encode(await service.request(method, args));
    },
  );
  await page.addInitScript(`(() => {
    const encode = ${encode.toString()};
    const decode = ${decode.toString()};
    const request = async (method, args = []) => decode(await window.__native(method, encode(args)));
    window.desktop = {
      initial: () => request("initial"), open: () => request("open"),
      create: name => request("create", [name]), sample: () => request("sample"),
      save: value => request("save", [value]), build: (...args) => request("build", args),
      publish: (...args) => request("publish", args), reveal: async () => {},
      dirty: () => {}, close: async () => {}, onCommand: callback => { window.__command = callback; return () => {}; },
    };
  })()`);
  await page.goto("http://127.0.0.1:5173");
  if (info.title.startsWith("welcome")) {
    await expect(
      page.getByRole("button", { name: /プロジェクトを開く/ }),
    ).toBeVisible();
    return;
  }
  await expect(
    page.getByText("冒険の道具箱", { exact: true }).first(),
  ).toBeVisible();
  await expect(
    page.getByRole("button", {
      name: info.title.startsWith("diagnostic") ? /1 件の問題/ : /0 件の問題/,
    }),
  ).toBeVisible();
});
test.afterEach(async () => {
  await page?.context().close();
  await fs.rm(root, { recursive: true, force: true });
});
async function saveAll() {
  await page.getByRole("button", { name: "保存方法", exact: true }).click();
  await page.getByRole("menuitem", { name: "すべて保存", exact: true }).click();
}
async function openNested(name: string) {
  await page.getByRole("button", { name, exact: true }).click();
  const panel = page.locator(".ant-drawer-content-wrapper");
  await expect(panel).toBeVisible();
  // rc-motion prepares the opening before a Web Animation exists. A finished-
  // animation promise alone returned early on Linux WebKit (CI trace 39083065919).
  await expect(panel).not.toHaveClass(
    /ant-drawer-panel-motion-.*-(?:appear|enter)(?:-|\s|$)/,
  );
}
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
  await page.getByRole("tab", { name: /ソース/ }).click();
  await expect(page.getByLabel("YAMLソース")).toHaveValue(
    await fs.readFile(path.join(root, "sources/types/rarity.yaml"), "utf8"),
  );
  await page.getByRole("tab", { name: "型の定義", exact: true }).click();
  await page.getByRole("button", { name: "候補を追加" }).click();
  const name = page.getByLabel("候補4の名前");
  await expect(name).toBeFocused();
  await name.fill("Legendary");
  await name.press("Enter");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  const saved = await fs.readFile(
    path.join(root, "sources/types/rarity.yaml"),
    "utf8",
  );
  expect(saved).toContain("Legendary");
  expect(saved).not.toContain("{");
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(name).toHaveValue("Member1");
  await expect(
    page.getByRole("status", { name: "未保存の変更", exact: true }),
  ).toBeVisible();
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
  await saveAll();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  expect(
    await fs.readFile(path.join(root, "sources/items/common.yaml"), "utf8"),
  ).toContain("luck");
  await page.getByRole("button", { name: "項目1の操作", exact: true }).click();
  await page.getByRole("menuitem", { name: /複製/ }).click();
  await expect(page.getByLabel("項目2の名前", { exact: true })).toBeFocused();
  await expect(page.getByLabel("項目2の名前", { exact: true })).toHaveValue(
    "attackCopy1",
  );
  await saveAll();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  const copied = parse(
    await fs.readFile(path.join(root, "sources/items/common.yaml"), "utf8"),
  );
  expect(
    copied.records.every((r: any) => r.stats.attackCopy1 === r.stats.attack),
  ).toBe(true);
});
test("top-level and nested bool share controls; nested nullable can be cleared", async () => {
  await selectSource("common");
  const top = page.getByRole("switch", { name: "enabled 行1", exact: true });
  await expect(top).toBeChecked();
  await openNested("stats 行1を開く");
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
test("nullable input bounds stay aligned across renderers, Undo and nested editing", async () => {
  await selectSource("common");
  async function bounds(frame: ReturnType<Page["locator"]>) {
    return frame.evaluate((element) => {
      const outer = element.getBoundingClientRect();
      const input = element
        .querySelector(".value-input > *")!
        .getBoundingClientRect();
      const action = element
        .querySelector(".nullable-slot")!
        .getBoundingClientRect();
      return {
        left: input.left - outer.left,
        width: input.width,
        action: action.left - outer.left,
      };
    });
  }
  function aligned(
    before: Awaited<ReturnType<typeof bounds>>,
    after: Awaited<ReturnType<typeof bounds>>,
  ) {
    for (const key of ["left", "width", "action"] as const)
      expect(Math.abs(before[key] - after[key]), key).toBeLessThan(1);
  }
  // All existing renderers go through the same frame. Compare the visible
  // renderer with its null button, rather than checking a particular CSS rule.
  for (const [name, column, row] of [
    ["code", 1, 1],
    ["rarity", 3, 1],
    ["price", 5, 1],
    ["stats", 6, 1],
    ["materials", 8, 1],
    ["note", 9, 2],
    ["tradable", 10, 1],
    ["tags", 11, 1],
    ["maxPower", 12, 2],
  ] as const) {
    const header = page.locator(".column-heading").filter({
      has: page.getByLabel(`列${column + 1}の名前`, { exact: true }),
    });
    const modifier = header.getByRole("button", {
      name: "nullを許可",
      exact: true,
    });
    if ((await modifier.getAttribute("aria-pressed")) !== "true")
      await modifier.click();
    const frame = page
      .locator(`[data-column-index="${column}"] .value-control`)
      .nth(row - 1);
    await frame.locator(".value-input > *").scrollIntoViewIfNeeded();
    const before = await bounds(frame);
    await page
      .getByRole("button", {
        name: `${name} 行${row}をnullにする`,
        exact: true,
      })
      .click();
    await expect(
      page.getByRole("button", {
        name: `${name} 行${row}に値を作成`,
        exact: true,
      }),
    ).toBeVisible();
    aligned(before, await bounds(frame));
    if (name === "maxPower")
      await page.screenshot({
        path: test.info().outputPath("table-null-width.png"),
      });
    await page.getByRole("button", { name: "元に戻す", exact: true }).click();
    await expect(
      page.getByRole("button", {
        name: `${name} 行${row}をnullにする`,
        exact: true,
      }),
    ).toBeVisible();
    aligned(before, await bounds(frame));
  }
  const switchWidth = (await page
    .getByRole("switch", {
      name: "tradable 行1",
      exact: true,
    })
    .boundingBox())!.width;
  await page.locator(".ant-table-tbody-virtual-holder").evaluate((element) => {
    element.dispatchEvent(
      new WheelEvent("wheel", {
        deltaX: 900 - document.querySelector(".ant-table-header")!.scrollLeft,
        bubbles: true,
        cancelable: true,
      }),
    );
  });
  await expect
    .poll(() =>
      page
        .locator(".ant-table-header")
        .evaluate((element) => element.scrollLeft),
    )
    .toBe(900);
  await openNested("stats 行1を開く");
  const nested = page
    .locator(".nested-field")
    .filter({
      has: page
        .locator(".nested-label strong")
        .filter({ hasText: /^critical$/ }),
    })
    .locator(".value-control");
  const before = await bounds(nested);
  expect(before.width).toBeGreaterThan(300);
  await page.screenshot({
    path: test.info().outputPath("nested-null-width.png"),
  });
  await page
    .getByRole("button", { name: "criticalに値を作成", exact: true })
    .click();
  aligned(before, await bounds(nested));
  expect(
    (await page
      .getByRole("switch", {
        name: "critical",
        exact: true,
      })
      .boundingBox())!.width,
  ).toBe(switchWidth);
  await page.screenshot({
    path: test.info().outputPath("nested-bool-width.png"),
  });
  await page
    .getByRole("button", { name: "criticalをnullにする", exact: true })
    .click();
  aligned(before, await bounds(nested));
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
  await saveAll();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
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
  await openNested("effects 行1を開く");
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
  await saveAll();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  expect(
    (
      await fs.stat(path.join(root, "sources/announcements/sandbox"))
    ).isDirectory(),
  ).toBe(true);
});
test("drag reorder retains visible rows and TSV paste is a single Undo", async () => {
  const handles = page.getByLabel("レコードを移動", { exact: true });
  await handles.first().dragTo(page.locator("[data-row-key]").nth(2));
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
  calls.length = 0;
  await folder.locator(".ant-tree-switcher").click();
  await expect(
    page.locator(".tree-name").filter({ hasText: /^type-0$/ }),
  ).toBeVisible();
  await folder.locator(".ant-tree-switcher").click();
  await folder.locator(".ant-tree-switcher").click();
  expect(calls).toEqual([]);
  expect(await page.locator("[data-row-key]").count()).toBeLessThan(100);
  await page.getByRole("button", { name: /件の問題/ }).click();
  await expect(page.locator(".problems-panel")).toBeVisible();
  const viewport = page.locator(".grid-viewport");
  const panel = page.locator(".problems-panel");
  const vp = await viewport.boundingBox();
  const pp = await panel.boundingBox();
  expect(vp!.y + vp!.height).toBeLessThanOrEqual(pp!.y + 1);
  const scroller = page.locator(".ant-table-tbody-virtual-holder");
  const start = (await page
    .getByLabel("レコードを移動", { exact: true })
    .first()
    .boundingBox())!;
  const bounds = (await scroller.boundingBox())!;
  const originalRow = await page
    .getByLabel("レコードを移動", { exact: true })
    .first()
    .locator("xpath=ancestor::*[@data-row-key]")
    .getAttribute("data-row-key");
  await page.mouse.move(start.x + start.width / 2, start.y + start.height / 2);
  await page.mouse.down();
  await page.mouse.move(
    start.x + start.width / 2,
    bounds.y + bounds.height - 5,
    { steps: 10 },
  );
  // Prove continued scrolling beyond the original viewport, not a fixed
  // pixels/second benchmark on a traced, software-rendered CI browser.
  await expect
    .poll(() => scroller.evaluate((element) => element.scrollTop), {
      timeout: 15000,
    })
    .toBeGreaterThan(bounds.height + 100);
  await expect(page.locator(`[data-row-key="${originalRow}"]`)).toHaveCount(0);
  await expect(page.locator(".sort-ghost")).toBeVisible();
  await page.mouse.up();
  await expect(page.locator(".sort-ghost")).toHaveCount(0);
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  const reordered = parse(
    await fs.readFile(
      path.join(root, "sources/announcements/announcement.yaml"),
      "utf8",
    ),
  );
  expect(
    reordered.records.findIndex((record: any) => record.id === 1),
  ).toBeGreaterThan(5);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await scroller.evaluate((element) => {
    element.scrollTop = 0;
  });
  await page.getByLabel("id 行1", { exact: true }).click();
  await page.getByLabel("id 行1", { exact: true }).press("Escape");
  await page.locator(".grid-editor").focus();
  await page.locator(".grid-editor").press("Control+End");
  await expect(page.getByLabel("id 行400", { exact: true })).toBeAttached();
});
test("paste extends records, preserves quoted TSV, and does not alter data from the search box", async () => {
  await page.getByLabel("id 行3", { exact: true }).click();
  await page.getByLabel("id 行3", { exact: true }).evaluate((element) => {
    const clipboardData = new DataTransfer();
    clipboardData.setData(
      "text/plain",
      '40\t"ひとつ\tの値"\n41\t次の行\n42\t最後の行',
    );
    element.dispatchEvent(
      new ClipboardEvent("paste", {
        clipboardData,
        bubbles: true,
        cancelable: true,
      }),
    );
  });
  await expect(page.getByLabel("id 行5", { exact: true })).toHaveValue("42");
  await expect(page.getByLabel("message 行3", { exact: true })).toHaveValue(
    "ひとつ\tの値",
  );
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("id 行5", { exact: true })).toHaveCount(0);
  await page.getByLabel("表を検索", { exact: true }).evaluate((element) => {
    const clipboardData = new DataTransfer();
    clipboardData.setData("text/plain", "99\t検索への貼り付け");
    element.dispatchEvent(
      new ClipboardEvent("paste", {
        clipboardData,
        bubbles: true,
        cancelable: true,
      }),
    );
  });
  await expect(page.getByLabel("id 行1", { exact: true })).toHaveValue("1");
});

test("welcome opens the first project without a phantom unsaved-change prompt", async () => {
  dialogPath = root;
  await page.getByRole("button", { name: /プロジェクトを開く/ }).click();
  await expect(
    page.getByText("冒険の道具箱", { exact: true }).first(),
  ).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
});

test("welcome creates a project, adds a table, edits and saves default records", async () => {
  const created = path.join(root, "新規作業");
  dialogPath = root;
  await page.getByRole("button", { name: /新しく作る/ }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.getByLabel("新しいプロジェクト名").fill("新規作業");
  await page.getByRole("button", { name: "保存先を選択", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    page.getByText("新規作業", { exact: true }).first(),
  ).toBeVisible();
  await page.getByRole("button", { name: /テーブルを作成/ }).click();
  await page.getByRole("button", { name: /レコードを追加/ }).click();
  await expect(page.getByLabel("id 行1", { exact: true })).toHaveValue("0");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  expect(
    await fs.readFile(path.join(created, "sources/new-table1.yaml"), "utf8"),
  ).toContain("id: 0");
});

test("column selection and duplication preserve values; search matches values rather than field names", async () => {
  const search = page.getByLabel("表を検索", { exact: true });
  await search.fill("message");
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveCount(0);
  await search.fill("");
  await page
    .getByRole("button", { name: "messageの操作", exact: true })
    .click();
  await page.getByRole("menuitem", { name: /列を選択/ }).click();
  await expect(page.locator(".selected-cell")).toHaveCount(3);
  await page.locator(".grid-editor").press("Delete");
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue("");
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "冒険へようこそ",
  );
  await page
    .getByRole("button", { name: "messageの操作", exact: true })
    .click();
  await page.getByRole("menuitem", { name: /列を複製/ }).click();
  await expect(page.getByLabel("列3の名前", { exact: true })).toBeFocused();
  await expect(
    page.getByLabel("messageCopy1 行1", { exact: true }),
  ).toHaveValue("冒険へようこそ");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  const saved = parse(
    await fs.readFile(
      path.join(root, "sources/announcements/announcement.yaml"),
      "utf8",
    ),
  );
  expect(saved.records.every((r: any) => r.messageCopy1 === r.message)).toBe(
    true,
  );
  expect(new Set(saved.fields.map((f: any) => f.key)).size).toBe(3);
});

test("nested navigation mounts each target independently and returns to its array", async () => {
  await selectSource("common");
  await openNested("effects 行1を開く");
  await page.getByRole("button", { name: "要素1を開く", exact: true }).click();
  const parent = await page
    .getByRole("switch", { name: "enabled", exact: true })
    .elementHandle();
  await expect(
    page.getByRole("switch", { name: "enabled", exact: true }),
  ).toBeChecked();
  await page.getByRole("button", { name: "statsを開く", exact: true }).click();
  await expect(
    page.getByRole("switch", { name: "enabled", exact: true }),
  ).not.toBeChecked();
  expect(await parent!.evaluate((element) => element.isConnected)).toBe(false);
  expect(
    await page
      .getByRole("switch", { name: "enabled", exact: true })
      .evaluate((element) =>
        element
          .getAnimations({ subtree: true })
          .some((a) => a instanceof CSSTransition),
      ),
  ).toBe(false);
  await page
    .getByRole("button", { name: "一つ上の値へ戻る", exact: true })
    .click();
  await expect(
    page.getByRole("switch", { name: "enabled", exact: true }),
  ).toBeChecked();
  await page
    .locator(".breadcrumb-scroll")
    .getByRole("button", { name: "effects", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "要素1を開く", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "一つ上の値へ戻る", exact: true }),
  ).toBeDisabled();
});

test("diagnostic navigation is consumed once and does not erase subsequent search input", async () => {
  await page.getByRole("button", { name: /1 件の問題/ }).click();
  await page.locator(".problem-item").first().click();
  const search = page.getByLabel("表を検索", { exact: true });
  await search.fill("収穫祭");
  await expect(page.getByLabel("message 行3", { exact: true })).toBeVisible();
  await expect(search).toHaveValue("収穫祭");
  await page.getByRole("button", { name: "問題を閉じる", exact: true }).click();
  await expect(search).toHaveValue("収穫祭");
  await search.press(process.platform === "darwin" ? "Meta+ArrowRight" : "End");
  await search.pressSequentially("test");
  await expect(search).toHaveValue("収穫祭test");
});

test("explorer menus match objects and folder titles toggle without disturbing the editor", async () => {
  const folder = page.locator(".tree-name").filter({ hasText: /^types$/ });
  await folder.click({ button: "right" });
  await expect(
    page.getByRole("menuitem", { name: /新しいEnum/ }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("menu")).toBeHidden();
  await folder.click();
  await expect(
    page.locator(".tree-name").filter({ hasText: /^rarity$/ }),
  ).toBeVisible();
  await folder.click();
  await expect(
    page.locator(".tree-name").filter({ hasText: /^rarity$/ }),
  ).toHaveCount(0);
  const file = page.locator(".tree-name").filter({ hasText: /^announcement$/ });
  await file.click({ button: "right" });
  await expect(
    page.getByRole("menuitem", { name: /名前を変更/ }),
  ).toBeVisible();
  await expect(page.getByRole("menuitem", { name: /新しい/ })).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(page.getByLabel("message 行1", { exact: true })).toBeVisible();
  await expect(page.getByText(/ファイル · メモリ上/)).toHaveCount(0);
  await page.getByRole("button", { name: /レコードを追加/ }).click();
  await expect(
    page.getByRole("status", { name: "未保存の変更", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".save-state")).toHaveText("");
  await page
    .getByRole("button", { name: "モーション: ON", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "モーション: OFF", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "モーション: OFF", exact: true })
    .click();
  await folder.click();
  await page.getByRole("button", { name: "新しい項目", exact: true }).click();
  await page.getByRole("menuitem", { name: /新しいEnum/ }).click();
  await page.getByLabel("項目の名前", { exact: true }).press("Escape");
  await saveAll();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  expect(
    await fs.readFile(path.join(root, "sources/types/NewType1.yaml"), "utf8"),
  ).toContain("kind: type");
});

test("horizontal viewport and column positions stay fixed through switch and nullable edits", async () => {
  await selectSource("common");
  const header = page.locator(".ant-table-header");
  await page.locator(".ant-table-tbody-virtual-holder").evaluate((element) => {
    element.dispatchEvent(
      new WheelEvent("wheel", {
        deltaX: 650,
        bubbles: true,
        cancelable: true,
      }),
    );
  });
  await header.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );
  await expect
    .poll(() => header.evaluate((element) => element.scrollLeft))
    .toBe(650);
  const control = page.getByRole("switch", {
    name: "enabled 行1",
    exact: true,
  });
  await expect(control).toBeVisible();
  const anchor = page.getByLabel("列5の名前", { exact: true });
  await anchor.evaluate((element) => {
    const win = window as any;
    win.columnPositions = [];
    const sample = () => {
      win.columnPositions.push({
        left: element.getBoundingClientRect().left,
        scroll: document.querySelector(".ant-table-header")!.scrollLeft,
      });
      if (win.columnPositions.length < 30) requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
  });
  await control.click();
  await expect
    .poll(() => page.evaluate(() => (window as any).columnPositions.length))
    .toBe(30);
  const points = await page.evaluate(
    () => (window as any).columnPositions as { left: number; scroll: number }[],
  );
  expect(
    Math.max(...points.map((p) => p.left)) -
      Math.min(...points.map((p) => p.left)),
  ).toBeLessThan(2);
  expect(points.every((p) => Math.abs(p.scroll - 650) < 2)).toBe(true);
  await page.locator(".ant-table-tbody-virtual-holder").evaluate((element) => {
    element.dispatchEvent(
      new WheelEvent("wheel", {
        deltaX: 1750 - document.querySelector(".ant-table-header")!.scrollLeft,
        bubbles: true,
        cancelable: true,
      }),
    );
  });
  await expect
    .poll(() => header.evaluate((element) => element.scrollLeft))
    .toBe(1750);
  const clear = page.getByRole("button", {
    name: "note 行2をnullにする",
    exact: true,
  });
  await expect(clear).toBeVisible();
  await clear.click();
  await expect(
    page.getByRole("button", { name: "note 行2に値を作成", exact: true }),
  ).toBeVisible();
  await expect
    .poll(() => header.evaluate((element) => element.scrollLeft))
    .toBe(1750);
  await page
    .getByRole("button", { name: "note 行2に値を作成", exact: true })
    .click();
  await expect
    .poll(() => header.evaluate((element) => element.scrollLeft))
    .toBe(1750);
});

test("offscreen nested buttons and header focus keep body cells aligned with their columns", async () => {
  await selectSource("common");
  const aligned = async (column: number) => {
    await expect
      .poll(() =>
        page.evaluate((column) => {
          const header = document
            .querySelector(`[aria-label="列${column + 1}の名前"]`)!
            .closest("th")!;
          const cell = document.querySelector(
            `.data-cell[data-column-index="${column}"]`,
          )!;
          return Math.abs(
            header.getBoundingClientRect().left -
              cell.getBoundingClientRect().left,
          );
        }, column),
      )
      .toBeLessThan(2);
  };
  await openNested("effects 行1を開く");
  await aligned(7);
  await page.locator(".ant-drawer-close").click();
  await page.getByLabel("列13の名前", { exact: true }).focus();
  await aligned(12);
  await page.getByLabel("maxPower 行1", { exact: true }).focus();
  await aligned(12);
});

test("native pointer drag reorders nested array elements without losing their editors", async () => {
  await selectSource("common");
  await openNested("effects 行1を開く");
  await page.getByRole("button", { name: "要素を複製", exact: true }).click();
  await page.getByRole("button", { name: "要素1を開く", exact: true }).click();
  await page.getByLabel("name", { exact: true }).fill("FirstEffect");
  await page.getByLabel("name", { exact: true }).press("Enter");
  await page.getByRole("button", { name: "一つ上の値へ戻る" }).click();
  await page
    .getByLabel("要素1を移動", { exact: true })
    .dragTo(page.getByLabel("要素2を移動", { exact: true }));
  await page.getByRole("button", { name: "要素2を開く", exact: true }).click();
  await expect(page.getByLabel("name", { exact: true })).toHaveValue(
    "FirstEffect",
  );
});

test("save scopes clear Explorer badges immediately and Escape deselects without closing the editor", async () => {
  await page.getByLabel("message 行1", { exact: true }).fill("保存する案内");
  await page.getByLabel("message 行1", { exact: true }).press("Enter");
  const announcement = page
    .locator(".tree-name")
    .filter({ hasText: /^announcement$/ });
  await expect(announcement.locator(".dirty-dot")).toHaveCount(1);
  await selectSource("rarity");
  await page.getByRole("button", { name: "候補を追加" }).click();
  const rarity = page.locator(".tree-name").filter({ hasText: /^rarity$/ });
  await expect(rarity.locator(".dirty-dot")).toHaveCount(1);
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(rarity.locator(".dirty-dot")).toHaveCount(0);
  await expect(announcement.locator(".dirty-dot")).toHaveCount(1);
  expect(
    await fs.readFile(
      path.join(root, "sources/announcements/announcement.yaml"),
      "utf8",
    ),
  ).not.toContain("保存する案内");
  await rarity.click();
  await page.keyboard.press("Escape");
  await expect(
    page.locator(".tree-scroll .ant-tree-node-selected"),
  ).toHaveCount(0);
  await expect(page.getByLabel("候補4の名前", { exact: true })).toBeVisible();
  await page.evaluate(() => (window as any).__command("save-all"));
  await expect(page.locator(".tree-scroll .dirty-dot")).toHaveCount(0);
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  expect(
    await fs.readFile(
      path.join(root, "sources/announcements/announcement.yaml"),
      "utf8",
    ),
  ).toContain("保存する案内");
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(rarity.locator(".dirty-dot")).toHaveCount(1);
});

test("Explorer truncates labels while preserving hierarchy and Flags None is fixed", async () => {
  await page.getByRole("button", { name: "新しい項目", exact: true }).click();
  await page.getByRole("menuitem", { name: /新しいフォルダ/ }).click();
  const name = page.getByLabel("項目の名前", { exact: true });
  const long = "非常に長いフォルダ名を表示して階層と省略を確認する";
  await name.fill(long);
  await name.press("Enter");
  const node = page.locator(".tree-name").filter({ hasText: long });
  expect(await node.getAttribute("title")).toContain(long);
  expect(
    await node.locator(".tree-label").evaluate((element) => {
      const style = getComputedStyle(element);
      return (
        style.textOverflow === "ellipsis" &&
        element.scrollWidth > element.clientWidth
      );
    }),
  ).toBe(true);
  const row = node.locator(
    "xpath=ancestor::*[contains(@class,'ant-tree-treenode')][1]",
  );
  const arrow = (await row.locator(".ant-tree-switcher").boundingBox())!;
  const icon = (await row.locator(".ant-tree-iconEle").boundingBox())!;
  expect(icon.x - arrow.x - arrow.width).toBeLessThan(8);
  expect((await row.boundingBox())!.height).toBeLessThanOrEqual(29);
  await expect(page.getByRole("menu")).toBeHidden();
  await page.screenshot({
    path: test.info().outputPath("explorer-density.png"),
  });
  await selectSource("item-flags");
  await expect(page.getByLabel("候補1の名前", { exact: true })).toBeDisabled();
  await expect(page.getByLabel("候補1の値", { exact: true })).toBeDisabled();
  await expect(page.getByLabel("候補1の値", { exact: true })).toHaveValue("0");
  await page.getByRole("button", { name: "候補1の操作", exact: true }).click();
  await expect(page.getByRole("menuitem", { name: /削除/ })).toHaveAttribute(
    "aria-disabled",
    "true",
  );
});

async function pointerPreview(
  from: ReturnType<Page["locator"]>,
  to: ReturnType<Page["locator"]>,
) {
  const a = (await from.boundingBox())!,
    b = (await to.boundingBox())!;
  await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
  await page.mouse.down();
  await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2, { steps: 12 });
  await expect(
    page.locator(".sort-dragging, .sort-ghost").first(),
  ).toBeVisible();
}

test("structural motion previews pointer reorder, cancels safely and commits one Undo", async () => {
  await page.evaluate(() => {
    (window as any).__motions = [];
    const animate = Element.prototype.animate;
    Element.prototype.animate = function (frames, options) {
      if (
        (this as HTMLElement).dataset.rowKey ||
        (this as HTMLElement).dataset.motionKey
      )
        (window as any).__motions.push({
          key:
            (this as HTMLElement).dataset.rowKey ||
            (this as HTMLElement).dataset.motionKey,
          frames,
        });
      return animate.call(this, frames, options);
    };
  });
  await page.getByRole("button", { name: /レコードを追加/ }).click();
  expect(
    await page.evaluate(() =>
      (window as any).__motions.some((m: any) => m.frames[0].opacity === 0),
    ),
  ).toBe(true);
  await pointerPreview(
    page.getByLabel("レコードを移動", { exact: true }).first(),
    page.getByLabel("レコードを移動", { exact: true }).nth(2),
  );
  await expect(page.getByLabel("id 行1", { exact: true })).toHaveValue("1");
  const translated = await page
    .locator('[data-sort-group="rows"]')
    .nth(1)
    .evaluate((e) => parseFloat(getComputedStyle(e).translate.split(" ")[1]));
  expect(translated).toBeLessThan(0);
  await page.keyboard.press("Escape");
  await page.mouse.up();
  await expect(page.getByLabel("id 行1", { exact: true })).toHaveValue("1");
  await pointerPreview(
    page.getByLabel("レコードを移動", { exact: true }).first(),
    page.getByLabel("レコードを移動", { exact: true }).nth(2),
  );
  await page.screenshot({ path: test.info().outputPath("drag-preview.png") });
  await page.mouse.up();
  await expect(page.getByLabel("id 行3", { exact: true })).toHaveValue("1");
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("id 行1", { exact: true })).toHaveValue("1");
  await pointerPreview(
    page.getByLabel("列を移動", { exact: true }).first(),
    page.getByLabel("列を移動", { exact: true }).nth(1),
  );
  await expect(page.getByLabel("列1の名前", { exact: true })).toHaveValue("id");
  await page.mouse.up();
  await expect(page.getByLabel("列1の名前", { exact: true })).toHaveValue(
    "message",
  );
  await selectSource("rarity");
  await page.getByRole("button", { name: "候補を追加" }).click();
  await pointerPreview(
    page.getByLabel("候補を移動", { exact: true }).first(),
    page.getByLabel("候補を移動", { exact: true }).nth(2),
  );
  await page.mouse.up();
  await expect(page.getByLabel("候補3の名前", { exact: true })).toHaveValue(
    "Common",
  );
  await page.getByRole("button", { name: "候補3の操作", exact: true }).click();
  await page.getByRole("menuitem", { name: /上へ/ }).click();
  await expect(page.getByLabel("候補2の名前", { exact: true })).toHaveValue(
    "Common",
  );
  await selectSource("combat-stats");
  await page.getByRole("button", { name: "フィールドを追加" }).click();
  await page.getByLabel("項目1の名前", { exact: true }).fill("AttackEdited");
  await pointerPreview(
    page.getByLabel("項目を移動", { exact: true }).first(),
    page.getByLabel("項目を移動", { exact: true }).nth(2),
  );
  await page.mouse.up();
  await expect(page.getByLabel("項目3の名前", { exact: true })).toHaveValue(
    "AttackEdited",
  );
  const motions = await page.evaluate(() => (window as any).__motions.length);
  await page
    .getByRole("button", { name: "モーション: ON", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "モーション: OFF", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "フィールドを追加" }).click();
  expect(await page.evaluate(() => (window as any).__motions.length)).toBe(
    motions,
  );
});

test("generation results reopen without repeating Build or Publish", async () => {
  await page.getByRole("button", { name: "Build" }).click();
  await expect(
    page.getByText("成果物を生成しました", { exact: true }),
  ).toBeVisible({ timeout: 30000 });
  await page.getByRole("button", { name: "Publishする" }).click();
  await page.getByRole("button", { name: "配置する", exact: true }).click();
  await expect(
    page.getByText("Publish完了 · rev.0 · すべてのデータ", { exact: true }),
  ).toBeVisible();
  const count = calls.filter((c) => c === "build" || c === "publish").length;
  const drawer = page
    .locator(".ant-drawer")
    .filter({ hasText: "Build / Publish" });
  await drawer.locator(".ant-drawer-close").click();
  await page
    .getByRole("button", { name: "生成結果を表示", exact: true })
    .click();
  await expect(
    page.getByText("成果物を生成しました", { exact: true }),
  ).toBeVisible();
  await drawer.locator(".ant-drawer-close").click();
  await page.evaluate(() => (window as any).__command("build-results"));
  await expect(
    page.getByText("成果物を生成しました", { exact: true }),
  ).toBeVisible();
  expect(calls.filter((c) => c === "build" || c === "publish").length).toBe(
    count,
  );
});

test("save-and-continue waits for an in-flight file save, then saves newer drafts too", async () => {
  let release!: () => void;
  saveGate = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.getByLabel("message 行1", { exact: true }).fill("最初の保存");
  await page.getByLabel("message 行1", { exact: true }).press("Enter");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await selectSource("rarity");
  await page.getByRole("button", { name: "候補を追加" }).click();
  await page.evaluate(() => {
    (window as any).__closed = false;
    window.desktop.close = async () => {
      (window as any).__closed = true;
    };
    (window as any).__command("close");
  });
  // Like Drawer preparation, Modal's paused scale(.2) can look stable for two
  // frames before its entry starts. A click there can miss the moving button.
  await expect(
    page.getByRole("dialog", { name: "変更を保存しますか？", exact: true }),
  ).not.toHaveClass(/ant-zoom-(?:appear|enter)(?:-|\s|$)/);
  await page
    .getByRole("button", { name: "保存して続ける", exact: true })
    .click();
  expect(await page.evaluate(() => (window as any).__closed)).toBe(false);
  release();
  await expect
    .poll(() => page.evaluate(() => (window as any).__closed))
    .toBe(true);
  expect(
    await fs.readFile(path.join(root, "sources/types/rarity.yaml"), "utf8"),
  ).toContain("Member1");
  expect(
    await fs.readFile(
      path.join(root, "sources/announcements/announcement.yaml"),
      "utf8",
    ),
  ).toContain("最初の保存");
  expect(calls.filter((c) => c === "save")).toHaveLength(2);
});

test("current-file shortcut does nothing after the displayed file is deleted; Save All persists deletion", async () => {
  const file = page.locator(".tree-name").filter({ hasText: /^announcement$/ });
  await expect(file).toBeVisible();
  await expect(
    page.locator(".tree-scroll .ant-tree-treenode-motion"),
  ).toHaveCount(0);
  await file.click({ button: "right" });
  await page.getByRole("menuitem", { name: /削除/ }).click();
  await expect(
    page.getByRole("button", { name: "保存", exact: true }),
  ).toBeDisabled();
  const count = await page.evaluate(() => {
    (window as any).__saveCalls = 0;
    const save = window.desktop.save;
    window.desktop.save = (request) => {
      (window as any).__saveCalls++;
      return save(request);
    };
    (window as any).__command("save");
    return (window as any).__saveCalls;
  });
  expect(count).toBe(0);
  expect(
    await fs.stat(path.join(root, "sources/announcements/announcement.yaml")),
  ).toBeTruthy();
  await page.evaluate(() => (window as any).__command("save-all"));
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  await expect(
    fs.stat(path.join(root, "sources/announcements/announcement.yaml")),
  ).rejects.toThrow();
});

async function closeFile() {
  await page.evaluate(() => (window as any).__command("close-file"));
}
async function closePrompt() {
  const dialog = page.getByRole("dialog", {
    name: "ファイルの変更を保存しますか？",
    exact: true,
  });
  await expect(dialog).toBeVisible();
  await expect(dialog).not.toHaveClass(/ant-zoom-(?:appear|enter)(?:-|\s|$)/);
  return dialog;
}
test("preview tabs replace clean previews, double click pins and editing before navigation auto-pins", async () => {
  const tabs = page.getByRole("tablist", { name: "開いているファイル" });
  await expect(tabs.getByRole("tab")).toHaveCount(1);
  await expect(page.locator(".preview-tab .file-tab-name")).toHaveCSS(
    "font-style",
    "italic",
  );
  await selectSource("rarity");
  await expect(tabs.getByRole("tab")).toHaveCount(1);
  await tabs.getByRole("tab").dblclick();
  await expect(page.locator(".preview-tab")).toHaveCount(0);
  await selectSource("combat-stats");
  await selectSource("announcement");
  await expect(tabs.getByRole("tab")).toHaveCount(2);
  await page.getByLabel("message 行1", { exact: true }).fill("入力途中も保持");
  await selectSource("combat-stats");
  await expect(tabs.getByRole("tab")).toHaveCount(3);
  await tabs.getByRole("tab", { name: /^announcement/ }).click();
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "入力途中も保持",
  );
  const search = page.getByLabel("表・型を検索");
  await search.fill("rarity");
  await page
    .locator(".tree-name")
    .filter({ hasText: /^rarity$/ })
    .dblclick();
  await search.fill("");
  await expect(tabs.getByRole("tab", { name: /^rarity/ })).not.toHaveClass(
    /preview-tab/,
  );
});

test("file close cancels, saves only that file, discards with Undo and keeps the last window open", async () => {
  await page.getByLabel("message 行1", { exact: true }).fill("保存対象");
  await selectSource("rarity");
  await page.getByRole("button", { name: "候補を追加" }).click();
  await closeFile();
  let dialog = await closePrompt();
  await dialog.getByRole("button", { name: "キャンセル", exact: true }).click();
  await expect(page.getByLabel("候補4の名前")).toBeVisible();
  await closeFile();
  dialog = await closePrompt();
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByRole("tab", { name: /^rarity/ })).toHaveCount(0);
  expect(
    await fs.readFile(path.join(root, "sources/types/rarity.yaml"), "utf8"),
  ).toContain("Member1");
  expect(
    await fs.readFile(
      path.join(root, "sources/announcements/announcement.yaml"),
      "utf8",
    ),
  ).not.toContain("保存対象");
  await closeFile();
  dialog = await closePrompt();
  await dialog.getByRole("button", { name: "保存しない", exact: true }).click();
  await expect(page.locator(".file-tab")).toHaveCount(0);
  await expect(page.locator(".workspace")).toBeVisible();
  await page.evaluate(() => (window as any).__command("undo"));
  await selectSource("announcement");
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "保存対象",
  );
});

test("file close save conflict retains the tab and drafts", async () => {
  await page.getByLabel("message 行1", { exact: true }).fill("失わない入力");
  const file = path.join(root, "sources/announcements/announcement.yaml");
  await fs.appendFile(file, "\n# external edit\n");
  await closeFile();
  const dialog = await closePrompt();
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText(/外部/).first()).toBeVisible();
  await expect(page.locator(".file-tab")).toHaveCount(1);
  await dialog.getByRole("button", { name: "キャンセル", exact: true }).click();
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "失わない入力",
  );
  expect(await fs.readFile(file, "utf8")).toContain("# external edit");
});

test("editor preferences preserve drafts, follow system theme and restore Explorer state", async () => {
  await page.getByLabel("message 行1", { exact: true }).fill("設定中のdraft");
  await page
    .getByRole("button", { name: "エクスプローラーの表示切替" })
    .click();
  await expect(page.locator(".explorer-container")).toHaveClass(
    /explorer-collapsed/,
  );
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "設定中のdraft",
  );
  await page.evaluate(() => (window as any).__command("explorer"));
  await expect(page.getByLabel("表・型を検索")).toBeVisible();
  await page
    .getByRole("button", { name: "エディター設定", exact: true })
    .click();
  const drawer = page.getByRole("dialog", {
    name: "エディター設定",
    exact: true,
  });
  await expect(page.locator(".ant-drawer-content-wrapper")).not.toHaveClass(
    /ant-drawer-panel-motion-.*-(?:appear|enter)(?:-|\s|$)/,
  );
  await drawer.getByText("ダーク", { exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await drawer.getByText("複数行に折り返す", { exact: true }).click();
  await drawer.getByText("システム", { exact: true }).click();
  await page.emulateMedia({ colorScheme: "light" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await drawer.locator(".ant-drawer-close").click();
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "設定中のdraft",
  );
  expect(calls.filter((c) => c === "initial")).toHaveLength(1);
  expect(calls.filter((c) => c === "save")).toHaveLength(0);
  await page.reload();
  await expect(page.locator(".file-tabs")).toHaveClass(/wrap-tabs/);
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
});

test("compact layout keeps commands and wrapped tabs reachable at minimum width in both themes", async () => {
  await page.setViewportSize({ width: 1000, height: 680 });
  const firstRow = await page
    .locator(".data-grid .ant-table-row")
    .first()
    .boundingBox();
  expect(firstRow!.y).toBeLessThan(300);
  for (const name of [
    "announcement",
    "schema",
    "common",
    "quest",
    "rarity",
    "combat-stats",
    "effect",
    "item-code",
    "item-flags",
  ]) {
    await selectSource(name);
    await page.locator('.file-tab[aria-selected="true"]').dblclick();
  }
  await expect(page.locator(".file-tab")).toHaveCount(9);
  const rowHeight = (await page.locator(".file-tabs").boundingBox())!.height;
  await page.evaluate(() => (window as any).__command("editor-settings"));
  const drawer = page.getByRole("dialog", {
    name: "エディター設定",
    exact: true,
  });
  await expect(page.locator(".ant-drawer-content-wrapper")).not.toHaveClass(
    /ant-drawer-panel-motion-.*-(?:appear|enter)(?:-|\s|$)/,
  );
  await drawer.getByText("複数行に折り返す", { exact: true }).click();
  await drawer.getByText("ダーク", { exact: true }).click();
  await drawer.locator(".ant-drawer-close").click();
  await expect(page.locator(".ant-drawer-content-wrapper:visible")).toHaveCount(
    0,
  );
  const wrapped = (await page.locator(".file-tabs").boundingBox())!.height;
  expect(wrapped).toBeGreaterThan(rowHeight + 20);
  const tabBoxes = await page.locator(".file-tab").evaluateAll((tabs) =>
    tabs.map((tab) => {
      const box = tab.getBoundingClientRect();
      return { left: box.left, right: box.right, top: box.top };
    }),
  );
  expect(tabBoxes.every((box) => box.right <= 1001)).toBe(true);
  await page.locator('.file-tab[aria-selected="true"]').focus();
  await page.keyboard.press("Home");
  await expect(
    page.getByRole("tab", { name: "announcement", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  const commands = await page
    .locator(".app-toolbar button, .app-toolbar .ant-select")
    .evaluateAll((items) =>
      items.map((item) => {
        const box = item.getBoundingClientRect();
        return { left: box.left, right: box.right };
      }),
    );
  expect(commands.every((box) => box.left >= 0 && box.right <= 1000)).toBe(
    true,
  );
  await expect(page.locator(".ant-drawer-open")).toHaveCount(0);
  await expect(page.locator(".ant-drawer-content-wrapper:visible")).toHaveCount(
    0,
  );
  await page.screenshot({ path: test.info().outputPath("compact-dark.png") });
  await page.evaluate(() => (window as any).__command("editor-settings"));
  await expect(page.locator(".ant-drawer-content-wrapper")).not.toHaveClass(
    /ant-drawer-panel-motion-.*-(?:appear|enter)(?:-|\s|$)/,
  );
  await drawer.getByText("ライト", { exact: true }).click();
  await drawer.locator(".ant-drawer-close").click();
  await expect(page.locator(".ant-drawer-content-wrapper:visible")).toHaveCount(
    0,
  );
  await page.screenshot({ path: test.info().outputPath("compact-light.png") });
});

test("new file close discards only the new draft and Undo can recover it", async () => {
  await page.getByRole("button", { name: "新しい項目", exact: true }).click();
  await page.getByRole("menuitem", { name: /新しいEnum/ }).click();
  await page.getByLabel("項目の名前", { exact: true }).press("Enter");
  const name = await page
    .locator('.file-tab[aria-selected="true"] .file-tab-name')
    .innerText();
  await closeFile();
  const dialog = await closePrompt();
  await dialog.getByRole("button", { name: "保存しない", exact: true }).click();
  await expect(page.getByRole("tab", { name, exact: true })).toHaveCount(0);
  await page.evaluate(() => (window as any).__command("undo"));
  await selectSource(name);
  await expect(page.getByRole("tab", { name, exact: true })).toBeVisible();
  expect(calls.filter((c) => c === "save")).toHaveLength(0);
});

test("file save-and-close keeps newer edits made during save and waits for earlier saves", async () => {
  let release!: () => void;
  saveGate = new Promise((resolve) => {
    release = resolve;
  });
  await page.getByLabel("message 行1", { exact: true }).fill("保存開始時点");
  await closeFile();
  const dialog = await closePrompt();
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect.poll(() => calls.filter((c) => c === "save").length).toBe(1);
  // Native Undo is still available while the save request is pending.
  await page.evaluate(() => (window as any).__command("undo"));
  release();
  await expect(
    page.getByText(
      "保存中に変更が追加されました。変更を確認して再度保存してください。",
      { exact: true },
    ),
  ).toBeVisible();
  await expect(page.locator(".file-tab")).toHaveCount(1);
  await dialog.getByRole("button", { name: "キャンセル", exact: true }).click();
  await expect(page.getByLabel("message 行1", { exact: true })).toHaveValue(
    "冒険へようこそ",
  );
  expect(
    await fs.readFile(
      path.join(root, "sources/announcements/announcement.yaml"),
      "utf8",
    ),
  ).toContain("保存開始時点");
  saveGate = new Promise((resolve) => {
    release = resolve;
  });
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect.poll(() => calls.filter((c) => c === "save").length).toBe(2);
  await closeFile();
  await expect(page.locator(".file-tab")).toHaveCount(1);
  await selectSource("rarity");
  release();
  await expect(
    page.getByRole("tab", { name: "announcement", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("tab", { name: "rarity", exact: true }),
  ).toBeVisible();
});

test("discard restores a saved file even if Undo removed it while close confirmation was open", async () => {
  await page.getByRole("button", { name: "新しい項目", exact: true }).click();
  await page.getByRole("menuitem", { name: /新しいEnum/ }).click();
  await page.getByLabel("項目の名前", { exact: true }).press("Enter");
  const name = await page
    .locator('.file-tab[aria-selected="true"] .file-tab-name')
    .innerText();
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "候補を追加" }).click();
  await closeFile();
  const dialog = await closePrompt();
  await page.evaluate(() => {
    (window as any).__command("undo");
    (window as any).__command("undo");
  });
  await expect(page.locator(".file-tab")).toHaveCount(0);
  await expect(dialog.getByText(new RegExp(name))).toBeVisible();
  await dialog.getByRole("button", { name: "保存しない", exact: true }).click();
  await selectSource(name);
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
});
