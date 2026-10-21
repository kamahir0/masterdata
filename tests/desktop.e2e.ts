import { test, expect, type Page } from "@playwright/test";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { parse, stringify } from "yaml";
import { readConfig } from "../desktop/config";
import { ProjectService } from "../desktop/service";
import { encode, decode } from "../src/core/wire";
let service: ProjectService;
let calls: string[];
let dialogPath: string | null;
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
  if (info.title.startsWith("hierarchy")) {
    await fs.mkdir(
      path.join(
        root,
        "sources/types/outer/inner/deep-folder-with-a-very-long-name",
      ),
      { recursive: true },
    );
  }
  if (info.title.startsWith("hierarchy guides")) {
    await fs.writeFile(
      path.join(root, "sources/types/outer/inner/deep-folder-with-a-very-long-name/leaf.yaml"),
      "kind: type\nname: Leaf\ncustom:\n  fields: []\n",
    );
    await fs.writeFile(
      path.join(root, "sources/types/outer/inner/z-last.yaml"),
      "kind: type\nname: Last\ncustom:\n  fields: []\n",
    );
    for (let i = 0; i < 65; i++)
      await fs.writeFile(
        path.join(root, `sources/types/z-virtual-${String(i).padStart(2, "0")}.yaml`),
        `kind: type\nname: Virtual${i}\ncustom:\n  fields: []\n`,
      );
  }
  if (info.title.startsWith("hierarchy drop reasons")) {
    for (const folder of ["incoming/clash-folder", "target/clash-folder"])
      await fs.mkdir(path.join(root, "sources/types", folder), { recursive: true });
    for (const [file, name] of [
      ["sources/types/target/rarity.yaml", "TargetRarity"],
      ["sources/types/target/other.yaml", "OtherType"],
      ["sources/rarity.yaml", "RootRarity"],
    ]) await fs.writeFile(path.join(root, file), `kind: type\nname: ${name}\ncustom:\n  fields: []\n`);
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
  dialogPath = path.join(root, "masterdata.toml");
  page = await browser.newPage({ viewport: { width: 1480, height: 960 } });
  await page.exposeFunction(
    "__native",
    async (method: string, payload: string) => {
      calls.push(method);
      if (method === "save") await saveGate;
      let args = decode<any[]>(payload);
      if (method === "initial")
        args = info.title.startsWith("welcome") ? [] : [path.join(root, "masterdata.toml")];
      if (method === "open") {
        if (dialogPath === null) return encode(null);
        args = [dialogPath];
      }
      if (method === "create") args = [args[0], root];
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
  await expect(page.locator(".nested-page:not([inert]) .breadcrumb-scroll")).toContainText("effects");
  await expect(page.locator(".nested-page:not([inert]) .breadcrumb-scroll")).toContainText("[0]");
  await expect(page.locator(".nested-page:not([inert]) .breadcrumb-scroll")).toContainText("stats");
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
test("hierarchy inline folder creation keeps focus through expansion and commits on outside click", async () => {
  const name = page.getByRole("textbox", { name: "項目の名前", exact: true });
  const node = (path: string) => page.locator(`.tree-name[data-path="${path}"]`);
  const settleTree = () => expect.poll(() => page.locator(".tree-scroll").evaluate(
    node => node.getAnimations({ subtree: true }).filter(a => a.playState === "running").length,
  )).toBe(0);
  const createIn = async (folder: string, value: string) => {
    await expect(page.locator(".ant-tree-treenode-motion")).toHaveCount(0);
    await settleTree();
    await node(folder).click({ button: "right" });
    await page.getByRole("menuitem", { name: /新しいフォルダ/ }).click();
    await expect(name).toBeFocused();
    await page.keyboard.type(value);
    // A transient expansion row is replaced by the final virtual row. Focus
    // must belong to the editor after that replacement as well as before it.
    await expect(page.locator(".ant-tree-treenode-motion")).toHaveCount(0);
    await settleTree();
    await expect(name).toBeFocused();
    await expect(name).toHaveValue(value);
    await page.getByLabel("表・型を検索").click();
    await expect(name).toHaveCount(0);
    await expect(node(`${folder}/${value}`)).toBeVisible();
    await expect(page.getByLabel("表・型を検索")).toBeFocused();
  };
  await node("types").click();
  await createIn("types/outer", "created-child");
  await createIn("types/outer/created-child", "created-grandchild");
  const grandchildPath = "types/outer/created-child/created-grandchild";
  const grandchild = node(grandchildPath);
  await grandchild.click({ button: "right" });
  await page.getByRole("menuitem", { name: /名前を変更/ }).click();
  await expect(name).toBeFocused();
  await name.fill("cancelled");
  await name.press("Escape");
  await expect(name).toHaveCount(0);
  await expect(grandchild).toBeVisible();
  await grandchild.click({ button: "right" });
  await page.getByRole("menuitem", { name: /新しいEnum/ }).click();
  await expect(name).toBeFocused();
  await page.keyboard.type("inside-enum");
  await settleTree();
  await expect(name).toBeFocused();
  await expect(name).toHaveValue("inside-enum");
  // A non-focusable outside surface must commit too, not just another input.
  await page.locator(".explorer-heading > span").click();
  await expect(name).toHaveCount(0);
  const created = node(`${grandchildPath}/inside-enum.yaml`);
  await expect(created).toBeVisible();
  await created.click({ button: "right" });
  await page.getByRole("menuitem", { name: /名前を変更/ }).click();
  await expect(name).toBeFocused();
  await name.fill("renamed-enum");
  await page.getByLabel("表・型を検索").click();
  await expect(name).toHaveCount(0);
  await expect(node(`${grandchildPath}/renamed-enum.yaml`)).toBeVisible();
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(created).toBeVisible();
  await grandchild.click();
  await expect(created).toBeHidden();
  await grandchild.click();
  await expect(created).toBeVisible();
  await saveAll();
  await expect(page.getByRole("status", { name: "保存済み", exact: true })).toBeVisible();
  expect((await fs.stat(path.join(root, "sources", grandchildPath))).isDirectory()).toBe(true);
  expect(await fs.readFile(path.join(root, "sources", grandchildPath, "inside-enum.yaml"), "utf8"))
    .toContain("kind: type");
  await page.screenshot({ path: test.info().outputPath("nested-folder-outside-commit.png") });
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
  dialogPath = path.join(root, "masterdata.toml");
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
  dialogPath = path.join(root, "masterdata.toml");
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
  expect(await parent!.evaluate((element) => !element.isConnected || !!element.closest<HTMLElement>(".nested-page")?.inert)).toBe(true);
  await expect.poll(() => parent!.evaluate((element) => element.isConnected)).toBe(false);
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
    .locator(".nested-page:not([inert]) .breadcrumb-scroll")
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
          options,
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
  expect(
    await page.evaluate(() =>
      (window as any).__motions
        .filter((m: any) => m.frames[0].opacity === 0)
        .every((m: any) => m.options.duration === 280),
    ),
  ).toBe(true);
  await expect(page.getByLabel("id 行4", { exact: true })).toBeFocused();
  await pointerPreview(
    page.getByLabel("レコードを移動", { exact: true }).first(),
    page.getByLabel("レコードを移動", { exact: true }).nth(2),
  );
  await expect(page.getByLabel("id 行1", { exact: true })).toHaveValue("1");
  await expect
    .poll(() =>
      page
        .locator('[data-sort-group="rows"]')
        .nth(1)
        .evaluate((e) =>
          parseFloat(getComputedStyle(e).translate.split(" ")[1]),
        ),
    )
    .toBeLessThan(0);
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
    page.getByText("Publish完了 · rev.0 · プロファイル未選択", { exact: true }),
  ).toBeVisible();
  const count = calls.filter((c) => c === "build" || c === "publish").length;
  const drawer = page
    .locator(".ant-drawer")
    .filter({ hasText: "Build / Publish" });
  await drawer.locator(".ant-drawer-close").click();
  await page.getByRole("button", { name: "ビルドの操作", exact: true }).click();
  await page
    .getByRole("menuitem", { name: "実行結果を表示", exact: true })
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

test("hierarchy aligns each child arrow with its parent icon and source kinds have coherent icons", async () => {
  await page.getByLabel("表・型を検索").fill("deep-folder");
  const node = (title: string) =>
    page
      .locator(`.tree-name[title="${title}"]`)
      .locator("xpath=ancestor::*[contains(@class,'ant-tree-treenode')][1]");
  for (const [parent, child] of [
    ["types", "types/outer"],
    ["types/outer", "types/outer/inner"],
    [
      "types/outer/inner",
      "types/outer/inner/deep-folder-with-a-very-long-name",
    ],
  ]) {
    const icon = (await node(parent)
      .locator(".ant-tree-iconEle")
      .boundingBox())!;
    const arrow = (await node(child)
      .locator(".ant-tree-switcher")
      .boundingBox())!;
    expect(
      Math.abs(icon.x + icon.width / 2 - arrow.x - arrow.width / 2),
    ).toBeLessThan(1);
  }
  await page.screenshot({
    path: test.info().outputPath("hierarchy-centers.png"),
  });
  const kindColors = new Set<string>();
  for (const [name, kind, label] of [
    ["announcement", "table", "テーブル"],
    ["common", "data", "データ"],
    ["rarity", "enum", "Enum（単一選択）"],
    ["item-flags", "flags", "Flags（複数選択）"],
    ["combat-stats", "custom", "Custom Type（構造）"],
    ["item-code", "valueObject", "Value Object（値の包み）"],
  ]) {
    await selectSource(name);
    const icon = page.locator('.file-tab[aria-selected="true"] .source-icon');
    await expect(icon).toHaveAttribute("data-source-kind", kind);
    kindColors.add(await icon.evaluate((el) => getComputedStyle(el).color));
    await expect(icon).toHaveAccessibleName(label);
    await expect(
      page
        .locator(`.tree-name[title$="/${name}.yaml"]`)
        .locator("xpath=ancestor::*[contains(@class,'ant-tree-treenode')][1]")
        .locator(".source-icon"),
    ).toHaveAttribute("data-source-kind", kind);
  }
  await page.getByRole("button", { name: "新しい項目", exact: true }).click();
  expect(kindColors.size).toBe(6);
  for (const kind of ["table", "enum", "flags", "custom", "valueObject"])
    await expect(
      page.getByRole("menu").locator(`[data-source-kind="${kind}"]`),
    ).toBeVisible();
  await expect(page.locator(".ant-dropdown:visible")).not.toHaveClass(
    /ant-slide-up-(?:appear|enter)(?:-|\s|$)/,
  );
  for (const kind of ["table", "enum", "flags", "custom", "valueObject"]) {
    const gap = await page
      .getByRole("menu")
      .locator(`[data-source-kind="${kind}"]`)
      .evaluate((icon) => {
        const item = icon.closest('[role="menuitem"]')!;
        const walker = document.createTreeWalker(item, NodeFilter.SHOW_TEXT);
        let node: Node | null;
        while ((node = walker.nextNode()))
          if (node.textContent?.includes("新しい")) {
            const range = document.createRange();
            range.selectNodeContents(node);
            return (
              range.getBoundingClientRect().left -
              icon.getBoundingClientRect().right
            );
          }
        return 0;
      });
    expect(gap).toBeGreaterThanOrEqual(7);
  }
  await page.screenshot({
    path: test.info().outputPath("source-kind-icons.png"),
  });
});

test("closing a pinned tab exits only that tab while edits and folder reveals do not replay additions", async () => {
  await page.evaluate(() => {
    (window as any).__motions = [];
    const animate = Element.prototype.animate;
    Element.prototype.animate = function (frames, options) {
      const element = this as HTMLElement;
      if (element.dataset.motionKey || element.dataset.rowKey)
        (window as any).__motions.push({
          key: element.dataset.motionKey || element.dataset.rowKey,
          frames,
          options,
        });
      return animate.call(this, frames, options);
    };
  });
  for (const name of ["announcement", "rarity", "combat-stats"]) {
    await selectSource(name);
    await page.locator('.file-tab[aria-selected="true"]').dblclick();
  }
  await page.evaluate(() => ((window as any).__motions = []));
  await page
    .locator(".file-tab")
    .nth(1)
    .getByRole("button", { name: /閉じる/ })
    .click();
  await expect(page.locator(".file-tab-slot:not([inert])")).toHaveCount(2);
  await expect(page.locator(".file-tab-slot[inert]")).toHaveCount(0);
  expect(
    await page.evaluate(() =>
      (window as any).__motions.filter(
        (m: any) => m.key.startsWith("tab-") && m.frames[0].translate,
      ),
    ),
  ).toHaveLength(0);
  await page.getByRole("button", { name: /フィールドを追加/ }).click();
  await expect(page.getByLabel("項目5の名前", { exact: true })).toBeFocused();
  await page.getByLabel("項目5の名前", { exact: true }).fill("newFieldEdited");
  await page.getByLabel("項目5の名前", { exact: true }).press("Enter");
  await page.evaluate(() => ((window as any).__motions = []));
  await page.getByLabel("項目1の名前", { exact: true }).fill("AttackEdited");
  await page.getByLabel("項目1の名前", { exact: true }).press("Enter");
  expect(
    await page.evaluate(() =>
      (window as any).__motions.filter((m: any) => m.frames[0].opacity === 0),
    ),
  ).toHaveLength(0);
  await selectSource("rarity");
  await page.getByRole("button", { name: /候補を追加/ }).click();
  await expect(page.getByLabel("候補4の名前", { exact: true })).toBeFocused();
  await page.getByLabel("候補4の名前", { exact: true }).fill("NewRarity");
  await page.getByLabel("候補4の名前", { exact: true }).press("Enter");
  await selectSource("item-flags");
  await page.getByRole("button", { name: /候補を追加/ }).click();
  await expect(page.getByLabel("候補5の名前", { exact: true })).toBeFocused();
  await page.getByLabel("候補5の名前", { exact: true }).fill("NewFlag");
  await page.getByLabel("候補5の名前", { exact: true }).press("Enter");
  await page.evaluate(() => ((window as any).__motions = []));
  await page.getByLabel("表・型を検索").fill("types");
  await page.getByLabel("表・型を検索").fill("");
  expect(
    await page.evaluate(() =>
      (window as any).__motions.filter(
        (m: any) => m.key.startsWith("node-") && m.frames[0].opacity === 0,
      ),
    ),
  ).toHaveLength(0);
});

test("large additions reveal a virtual new record and a new column without delaying input", async () => {
  await page.evaluate(() => {
    (window as any).__motions = [];
    const animate = Element.prototype.animate;
    Element.prototype.animate = function (frames, options) {
      const element = this as HTMLElement;
      if (element.dataset.motionKey || element.dataset.rowKey)
        (window as any).__motions.push({
          key: element.dataset.motionKey || element.dataset.rowKey,
          frames,
          options,
        });
      return animate.call(this, frames, options);
    };
  });
  // Hold animation-frame delivery after Add, as if a busy main thread delayed
  // the virtual row's first paint. Feedback must survive; later user input ends it.
  await page.evaluate(() => {
    const request = window.requestAnimationFrame.bind(window);
    const cancel = window.cancelAnimationFrame.bind(window);
    const queued = new Map<
      number,
      { frame: number; timer?: ReturnType<typeof setTimeout> }
    >();
    let resumeAt = 0;
    const arm = (event: Event) => {
      if (
        !(event.target as HTMLElement)
          .closest("button")
          ?.textContent?.includes("レコードを追加")
      )
        return;
      resumeAt = performance.now() + 1800;
      document.removeEventListener("click", arm, true);
    };
    document.addEventListener("click", arm, true);
    window.requestAnimationFrame = (callback) => {
      let id = 0;
      id = request((time) => {
        const remaining = resumeAt - performance.now();
        if (remaining > 0) {
          const entry = {
            frame: id,
            timer: setTimeout(() => {
              entry.frame = request((next) => {
                queued.delete(id);
                callback(next);
              });
            }, remaining),
          };
          queued.set(id, entry);
        } else {
          queued.delete(id);
          callback(time);
        }
      });
      return id;
    };
    window.cancelAnimationFrame = (id) => {
      const entry = queued.get(id);
      if (entry) {
        clearTimeout(entry.timer);
        cancel(entry.frame);
        queued.delete(id);
      } else cancel(id);
    };
  });
  await page
    .locator(".ant-table-header")
    .evaluate((element) => (element.scrollLeft = 800));
  await page
    .getByRole("textbox", { name: "表を検索", exact: true })
    .fill("レコード1");
  await page.getByRole("button", { name: /レコードを追加/ }).click();
  const added = page.getByLabel("id 行401", { exact: true });
  await expect(added).toBeFocused();
  await expect(
    page.getByRole("textbox", { name: "表を検索", exact: true }),
  ).toHaveValue("");
  await expect(added).toHaveValue("0");
  const rowKey = await added
    .locator("xpath=ancestor::*[@data-row-key][1]")
    .getAttribute("data-row-key");
  await expect
    .poll(() =>
      page.evaluate(
        (key) =>
          (window as any).__motions.some(
            (m: any) =>
              m.key === key &&
              m.frames[0].opacity === 0 &&
              m.options.duration === 280,
          ),
        rowKey,
      ),
    )
    .toBe(true);
  await expect
    .poll(() =>
      page
        .locator(".ant-table-header")
        .evaluate((element) => element.scrollLeft),
    )
    .toBe(0);
  await added.fill("9001");
  await added.press("Enter");
  await expect(added).toHaveValue("9001");
  await page.getByRole("button", { name: /列を追加/ }).click();
  const column = page.getByLabel("列63の名前", { exact: true });
  await expect(column).toBeFocused();
  await column.fill("lastField");
  await column.press("Enter");
  await expect(column).toHaveValue("lastField");
  const columnKey = await column
    .locator("xpath=ancestor::*[@data-motion-key][1]")
    .getAttribute("data-motion-key");
  await expect
    .poll(() =>
      page.evaluate(
        (key) =>
          (window as any).__motions.some(
            (m: any) =>
              m.key === key &&
              m.frames[0].opacity === 0 &&
              m.options.duration === 280,
          ),
        columnKey,
      ),
    )
    .toBe(true);
  await page.screenshot({
    path: test.info().outputPath("virtual-additions.png"),
  });
});

test("Build profile starts unselected, can be cleared, and Publish menu only opens the review panel", async () => {
  const profile = page.getByRole("combobox", {
    name: "出力プロファイル",
    exact: true,
  });
  await expect(page.locator(".profile-select")).toContainText(
    "プロファイルを選択",
  );
  await profile.click();
  await expect(
    page.getByRole("option", { name: "すべてのデータ", exact: true }),
  ).toHaveCount(0);
  await page.locator('.ant-select-item-option[title="production"]').click();
  await page.getByRole("button", { name: "ビルドの操作", exact: true }).click();
  await page.getByRole("menuitem", { name: "Publish…", exact: true }).click();
  const drawer = page
    .locator(".ant-drawer")
    .filter({ hasText: "Build / Publish" });
  await expect(drawer.getByText("production", { exact: true })).toBeVisible();
  expect(calls.filter((c) => c === "build" || c === "publish")).toHaveLength(0);
  await drawer.locator(".ant-drawer-close").click();
  await page.locator(".profile-select .ant-select-clear").click();
  await expect(page.locator(".profile-select")).toContainText(
    "プロファイルを選択",
  );
  await page.getByRole("button", { name: "ビルドの操作", exact: true }).click();
  await page
    .getByRole("menuitem", { name: "実行結果を表示", exact: true })
    .click();
  await expect(
    drawer.getByText("プロファイル未選択", { exact: true }),
  ).toBeVisible();
  await expect(
    drawer.getByText("タグの絞り込みなし", { exact: true }),
  ).toBeVisible();
  expect(calls.filter((c) => c === "build" || c === "publish")).toHaveLength(0);
  await expect(drawer.locator(".ant-drawer-content-wrapper")).not.toHaveClass(
    /ant-drawer-panel-motion-.*-(?:appear|enter)(?:-|\s|$)/,
  );
  await expect(page.locator(".ant-dropdown:visible")).toHaveCount(0);
  await page.screenshot({
    path: test.info().outputPath("build-panel-unselected.png"),
  });
  await drawer.locator(".ant-drawer-close").click();
  await profile.click();
  await page.locator('.ant-select-item-option[title="production"]').click();
  await page
    .getByRole("button", { name: "プロジェクト設定", exact: true })
    .click();
  const settings = page.getByRole("dialog", {
    name: "プロジェクト設定",
    exact: true,
  });
  await page
    .getByRole("button", { name: "productionを削除", exact: true })
    .click();
  await settings.locator(".ant-drawer-close").click();
  await expect(page.locator(".profile-select")).toContainText(
    "プロファイルを選択",
  );
});

test("Build commands coalesce while busy, preserve editing, and release the guard after failure", async () => {
  await page.evaluate(() => {
    const state = window as any;
    state.__outputCalls = [];
    const gate = new Promise<void>(
      (resolve) => (state.__releaseBuild = resolve),
    );
    window.desktop.build = async (_token, _snapshot, profile, revision) => {
      state.__outputCalls.push({ profile, revision });
      if (state.__rejectBuild) throw new Error("試験用のBuild失敗");
      await gate;
      return {
        directory: "test",
        revision,
        profile,
        tables: 3,
        records: 7,
        log: "",
        files: [],
        hashes: {},
      };
    };
    state.__command("build");
    state.__command("build");
  });
  expect(await page.evaluate(() => (window as any).__outputCalls)).toEqual([
    { profile: "", revision: 0 },
  ]);
  const drawer = page
    .locator(".ant-drawer")
    .filter({ hasText: "Build / Publish" });
  await drawer.locator(".ant-drawer-close").click();
  await page.getByLabel("message 行1", { exact: true }).fill("Build中も編集");
  await page.getByLabel("message 行1", { exact: true }).press("Enter");
  await page.evaluate(() => (window as any).__releaseBuild());
  await expect(
    page.locator(".build-controls").getByRole("button", { name: /Build/ }),
  ).not.toHaveClass(/ant-btn-loading/);
  await page.getByRole("button", { name: "ビルドの操作", exact: true }).click();
  await page
    .getByRole("menuitem", { name: "実行結果を表示", exact: true })
    .click();
  await expect(
    drawer.getByText("Build後に編集内容または出力条件が変わっています", {
      exact: true,
    }),
  ).toBeVisible();
  await page.evaluate(() => ((window as any).__rejectBuild = true));
  await page.getByRole("button", { name: "Buildする" }).click();
  await expect(
    drawer.getByText("試験用のBuild失敗", { exact: true }),
  ).toBeVisible();
  await expect(
    drawer.getByText("成果物を生成しました", { exact: true }),
  ).toBeVisible();
  await page.evaluate(() => ((window as any).__rejectBuild = false));
  await page.getByRole("button", { name: "Buildする" }).click();
  await expect(
    drawer.getByText("試験用のBuild失敗", { exact: true }),
  ).toHaveCount(0);
  await expect(
    drawer.getByText("Build後に編集内容または出力条件が変わっています", {
      exact: true,
    }),
  ).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).__outputCalls.length)).toBe(
    3,
  );
});

test("Explorer resize persists, supports keyboard, and creation reveals inside a closed filtered folder", async () => {
  const resize = page.getByRole("separator", { name: "エクスプローラーの幅" });
  const box = (await resize.boundingBox())!;
  await page.mouse.move(box.x, box.y + 80);
  await page.mouse.down();
  await page.mouse.move(box.x + 100, box.y + 80, { steps: 8 });
  await page.mouse.up();
  await expect(resize).toHaveAttribute("aria-valuenow", "336");
  await resize.focus();
  await page.keyboard.press("Home");
  await expect(resize).toHaveAttribute("aria-valuenow", "180");
  await page.keyboard.press("ArrowRight");
  await expect(resize).toHaveAttribute("aria-valuenow", "196");
  await page.reload();
  await expect(resize).toHaveAttribute("aria-valuenow", "196");
  const search = page.getByLabel("表・型を検索");
  await search.fill("types");
  const folder = page.locator('.tree-name[data-path="types"]');
  await folder.click({ button: "right" });
  await page.getByRole("menuitem", { name: /新しいフォルダ/ }).click();
  const name = page.getByLabel("項目の名前");
  await expect(search).toHaveValue("");
  await expect(name).toBeFocused();
  await expect(name).toBeInViewport();
  await name.fill("created-here");
  await name.press("Enter");
  await page.locator('.tree-name[data-path="types"]').click(); // collapse the parent
  await folder.click({ button: "right" });
  await page.getByRole("menuitem", { name: /新しいテーブル/ }).click();
  await expect(name).toBeFocused();
  await expect(name).toBeInViewport();
  await name.fill("new-inside");
  await name.press("Enter");
  await expect(
    page.locator('.tree-name[data-path="types/new-inside.yaml"]'),
  ).toBeVisible();
  await page.screenshot({
    path: test.info().outputPath("explorer-created-and-resized.png"),
  });
});

test("folder body drop highlights and moves into the folder; Undo restores the original path", async () => {
  await selectSource("rarity");
  const from = page.locator('.tree-name[data-path="types/rarity.yaml"]');
  const target = page.locator('.tree-name[data-path="announcements"]');
  const data = await page.evaluateHandle(() => new DataTransfer());
  await from.dispatchEvent("dragstart", { dataTransfer: data });
  const box = (await target.boundingBox())!;
  await target.dispatchEvent("dragenter", {
    dataTransfer: data,
    clientX: box.x + 12,
    clientY: box.y + box.height / 2,
  });
  await target.dispatchEvent("dragover", {
    dataTransfer: data,
    clientX: box.x + 12,
    clientY: box.y + box.height / 2,
  });
  await expect(
    target.locator(
      'xpath=ancestor::*[contains(@class,"ant-tree-treenode")][1]',
    ),
  ).toHaveClass(/folder-drop-target/);
  await target.dispatchEvent("drop", {
    dataTransfer: data,
    clientX: box.x + 12,
    clientY: box.y + box.height / 2,
  });
  await expect(
    page.locator('.tree-name[data-path="announcements/rarity.yaml"]'),
  ).toBeVisible();
  await expect(page.locator(".folder-drop-target")).toHaveCount(0);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(from).toBeVisible();
});

test("deleting collections removes editors immediately, shows inert exits, and Undo restores values", async () => {
  await page.evaluate(() => {
    (window as any).__exits = [];
    const animate = Element.prototype.animate;
    Element.prototype.animate = function (frames, options) {
      if ((this as HTMLElement).classList.contains("edit-exit")) {
        (window as any).__exits.push({
          inert: (this as HTMLElement).inert,
          hidden: this.getAttribute("aria-hidden"),
          frames,
          options,
        });
      }
      return animate.call(this, frames, options);
    };
  });
  await selectSource("rarity");
  await page.getByRole("button", { name: "候補3の操作", exact: true }).click();
  await page.getByRole("menuitem", { name: /削除/ }).click();
  await expect(page.getByLabel("候補3の名前", { exact: true })).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).__exits)).toEqual(
    expect.arrayContaining([
      expect.objectContaining({
        inert: true,
        hidden: "true",
        options: expect.objectContaining({ duration: 300 }),
      }),
    ]),
  );
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("候補3の名前", { exact: true })).toHaveValue(
    "Epic",
  );
  await expect(page.locator(".edit-exit")).toHaveCount(0);
  await selectSource("combat-stats");
  await page.getByRole("button", { name: "項目4の操作", exact: true }).click();
  await page.getByRole("menuitem", { name: /削除/ }).click();
  await expect(page.getByLabel("項目4の名前", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("項目4の名前", { exact: true })).toHaveValue(
    "critical",
  );
  await selectSource("announcement");
  await page.getByRole("button", { name: "行3の操作", exact: true }).click();
  await page.getByRole("menuitem", { name: /削除/ }).click();
  await expect(page.getByLabel("id 行3", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("id 行3", { exact: true })).toHaveValue("3");
  await expect(page.locator(".edit-exit")).toHaveCount(0);
});

test("deletion visibly collapses its space while the last-item Add control follows, without delaying Undo", async () => {
  await selectSource("rarity");
  const add = page.locator(".definition-add button");
  const before = (await add.boundingBox())!;
  const frame = page.locator('[data-motion-layout="definition-frame"]');
  await page.evaluate(() => {
    (window as any).__deletionAnimations = [];
    const animate = Element.prototype.animate;
    (window as any).__originalAnimate = animate;
    Element.prototype.animate = function (frames, options) {
      const animation = animate.call(this, frames, options);
      if (
        typeof options === "object" &&
        options.duration === 300 &&
        this.matches(
          ".edit-exit, [data-row-key], [data-motion-key], [data-motion-layout], .ant-tree-treenode",
        )
      ) {
        animation.pause();
        animation.currentTime = 0;
        (window as any).__deletionAnimations.push(animation);
      }
      return animation;
    };
  });
  await page.getByRole("button", { name: "候補3の操作", exact: true }).click();
  await page.getByRole("menuitem", { name: /削除/ }).click();
  const ghost = page.locator(".edit-exit");
  await expect(ghost).toHaveCount(1);
  await expect(page.getByLabel("候補3の名前", { exact: true })).toHaveCount(0);
  expect(Math.abs((await add.boundingBox())!.y - before.y)).toBeLessThan(2);
  expect(
    (await frame.boundingBox())!.y + (await frame.boundingBox())!.height,
  ).toBeGreaterThanOrEqual(before.y + before.height);
  const at = async (time: number) => {
    await page.evaluate((time) => {
      (window as any).__deletionAnimations.forEach((a: Animation) => {
        a.currentTime = time;
      });
    }, time);
    return {
      y: (await add.boundingBox())!.y,
      ...(await ghost.evaluate((el) => ({
        opacity: Number(getComputedStyle(el).opacity),
        clip: getComputedStyle(el).clipPath,
      }))),
    };
  };
  const early = await at(80);
  expect(early.opacity).toBeGreaterThan(0.65);
  expect(early.opacity).toBeLessThan(1);
  expect(early.clip).not.toBe("none");
  await page.screenshot({ path: test.info().outputPath("deletion-early.png") });
  const middle = await at(150);
  const late = await at(299);
  expect(before.y - middle.y).toBeGreaterThan(5);
  expect(middle.y - late.y).toBeGreaterThan(5);
  expect(early.y).toBeGreaterThan(middle.y);
  await at(150);
  await page.screenshot({
    path: test.info().outputPath("deletion-midpoint.png"),
  });
  // The visual exit is still held halfway; Undo never waits for it.
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("候補3の名前", { exact: true })).toHaveValue(
    "Epic",
  );
  await expect(ghost).toHaveCount(0);
});

test("deletion moves following fields, rows, columns, arrays and Explorer nodes; reduced motion is immediate", async () => {
  await page.evaluate(() => {
    (window as any).__deletionAnimations = [];
    const animate = Element.prototype.animate;
    (window as any).__originalAnimate = animate;
    Element.prototype.animate = function (frames, options) {
      const animation = animate.call(this, frames, options);
      if (
        typeof options === "object" &&
        options.duration === 300 &&
        this.matches(
          ".edit-exit, [data-row-key], [data-motion-key], [data-motion-layout], .ant-tree-treenode",
        )
      ) {
        animation.pause();
        animation.currentTime = 0;
        (window as any).__deletionAnimations.push(animation);
      }
      return animation;
    };
  });
  const seek = (time: number) =>
    page.evaluate((time) => {
      (window as any).__deletionAnimations.forEach((a: Animation) => {
        if (a.playState !== "idle") a.currentTime = time;
      });
    }, time);
  const check = async (
    selector: string,
    remove: () => Promise<void>,
    axis: "x" | "y" = "y",
  ) => {
    const following = page.locator(selector);
    const before = (await following.boundingBox())![axis];
    await page.evaluate(() => {
      (window as any).__deletionAnimations = [];
    });
    await remove();
    await expect(page.locator(".edit-exit").first()).toBeAttached();
    const start = (await following.boundingBox())![axis];
    expect(Math.abs(start - before)).toBeLessThan(2);
    await seek(80);
    expect(
      await page
        .locator(".edit-exit")
        .first()
        .evaluate((el) => Number(getComputedStyle(el).opacity)),
    ).toBeGreaterThan(0.65);
    await seek(150);
    const middle = (await following.boundingBox())![axis];
    await seek(299);
    const end = (await following.boundingBox())![axis];
    expect(before - middle).toBeGreaterThan(2);
    expect(middle - end).toBeGreaterThan(2);
    await page.getByRole("button", { name: "元に戻す", exact: true }).click();
    await expect(page.locator(".edit-exit")).toHaveCount(0);
    // Each next geometry assertion starts after Undo's real (unpaused) entry
    // and move feedback; menu/checkbox preparation must not change its baseline.
    await page.evaluate(() =>
      Promise.all(
        document
          .getAnimations()
          .filter(
            (a) =>
              a.playState === "running" &&
              (a.effect as KeyframeEffect).target?.matches(
                "[data-row-key], [data-motion-key], [data-motion-layout], .ant-tree-treenode",
              ),
          )
          .map((a) => a.finished.catch(() => {})),
      ),
    );
  };
  await selectSource("combat-stats");
  const fieldKey = await page
    .getByLabel("項目3の名前", { exact: true })
    .locator("xpath=ancestor::*[@data-row-key][1]")
    .getAttribute("data-row-key");
  await check(`[data-row-key="${fieldKey}"]`, async () => {
    await page
      .getByRole("button", { name: "項目2の操作", exact: true })
      .click();
    await page.getByRole("menuitem", { name: /削除/ }).click();
  });
  await selectSource("announcement");
  const rowKey = await page
    .getByLabel("id 行2", { exact: true })
    .locator("xpath=ancestor::*[@data-row-key][1]")
    .getAttribute("data-row-key");
  await check(`[data-row-key="${rowKey}"]`, async () => {
    await page.getByRole("button", { name: "行1の操作", exact: true }).click();
    await page.getByRole("menuitem", { name: /削除/ }).click();
  });
  const lastRowKey = await page
    .getByLabel("id 行3", { exact: true })
    .locator("xpath=ancestor::*[@data-row-key][1]")
    .getAttribute("data-row-key");
  await check(`[data-row-key="${lastRowKey}"]`, async () => {
    const selection = page.locator(
      '.grid-editor [data-row-key] input[type="checkbox"]',
    );
    await selection.nth(0).check();
    await selection.nth(1).check();
    await page.locator(".grid-editor").press("Delete");
  });
  const columnKey = await page
    .getByLabel("列2の名前", { exact: true })
    .locator("xpath=ancestor::*[@data-motion-key][1]")
    .getAttribute("data-motion-key");
  await check(
    `[data-motion-key="${columnKey}"]`,
    async () => {
      await page.getByRole("button", { name: "idの操作", exact: true }).click();
      await page.getByRole("menuitem", { name: /列を削除/ }).click();
    },
    "x",
  );
  await selectSource("common");
  await openNested("materials 行1を開く");
  const arrayKey = await page
    .locator(".nested-page:not([inert]) .array-item")
    .nth(1)
    .getAttribute("data-motion-key");
  await check(`[data-motion-key="${arrayKey}"]`, async () => {
    await page
      .locator(".nested-page:not([inert]) .array-item")
      .first()
      .getByRole("button", { name: "要素を削除", exact: true })
      .click();
  });
  await page.getByRole("button", { name: "閉じる", exact: true }).click();
  await selectSource("combat-stats");
  await check('.tree-name[data-path="types/effect.yaml"]', async () => {
    await page
      .locator('.tree-name[data-path="types/combat-stats.yaml"]')
      .click({ button: "right" });
    await page.getByRole("menuitem", { name: /削除/ }).click();
  });
  await page.evaluate(() => {
    Element.prototype.animate = (window as any).__originalAnimate;
    (window as any).__deletionAnimations.forEach((a: Animation) => {
      if (a.playState !== "idle") {
        a.finish();
        a.cancel();
      }
    });
  });
  await selectSource("rarity");
  await page
    .getByRole("button", { name: "モーション: ON", exact: true })
    .click();
  await page.getByRole("button", { name: "候補3の操作", exact: true }).click();
  await page.getByRole("menuitem", { name: /削除/ }).click();
  await expect(page.getByLabel("候補3の名前", { exact: true })).toHaveCount(0);
  await expect(page.locator(".edit-exit")).toHaveCount(0);
});

test("reduced-motion deletion keeps its commands usable and immediately removes the editor", async () => {
  await selectSource("rarity");
  await page
    .getByRole("button", { name: "モーション: ON", exact: true })
    .click();
  await page.getByRole("button", { name: "候補3の操作", exact: true }).click();
  await expect(page.getByRole("menuitem", { name: /削除/ })).toBeAttached();
  await page.getByRole("menuitem", { name: /削除/ }).click();
  await expect(page.getByLabel("候補3の名前", { exact: true })).toHaveCount(0);
  await expect(page.locator(".edit-exit")).toHaveCount(0);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("候補3の名前", { exact: true })).toHaveValue(
    "Epic",
  );
});

test("source heading distinguishes table data, key labels and references communicate actual values first", async () => {
  await selectSource("schema");
  await expect(page.locator(".editor-title > :first-child")).toHaveText(
    "TABLE",
  );
  await expect(page.locator(".column-keys").first()).toHaveText("PKKey 0");
  await expect(page.locator(".column-keys").nth(1)).toHaveText("SK 0Key 1");
  await expect(page.locator(".column-keys").nth(3)).toHaveText("SK 1Key 3");
  await expect(page.getByRole("combobox", { name: "追加先ファイル", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /レコードを追加/ })).toBeDisabled();
  await expect(
    page.getByRole("button", {
      name: "テーブルデータファイルを作成",
      exact: true,
    }),
  ).toBeVisible();
  await selectSource("common");
  await expect(page.locator(".editor-title > :first-child")).toHaveText(
    "TABLE DATA",
  );
  await selectSource("quest");
  const reference = page.locator(".reference-label").first();
  await expect(reference.locator("span").first()).toHaveText("2001");
  await expect(reference.locator(".reference-name")).toHaveText("氷晶の剣");
  const styles = await reference.evaluate((el) => {
    const a = getComputedStyle(el.children[0]),
      b = getComputedStyle(el.children[1]);
    return {
      main: a.color,
      name: b.color,
      smaller: parseFloat(b.fontSize) < parseFloat(a.fontSize),
    };
  });
  expect(styles.main).not.toBe(styles.name);
  expect(styles.smaller).toBe(true);
  await page.screenshot({
    path: test.info().outputPath("source-keys-reference.png"),
  });
  await page
    .getByRole("combobox", { name: "rewardItemId 行1", exact: true })
    .fill("銅");
  await page
    .locator(".ant-select-item-option-content")
    .filter({ hasText: /1001.*銅の剣/ })
    .click();
  await expect(reference.locator("span").first()).toHaveText("1001");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  expect(
    parse(
      await fs.readFile(path.join(root, "sources/quests/quest.yaml"), "utf8"),
    ).records[0].rewardItemId,
  ).toBe(1001);
});

test("primitive array edits and duplicate reorder retain item identity and a single-step Undo", async () => {
  await selectSource("common");
  await openNested("materials 行1を開く");
  const items = page.locator(".nested-page:not([inert]) .array-item");
  const keys = () =>
    items.evaluateAll((nodes) =>
      nodes.map((n) => (n as HTMLElement).dataset.motionKey),
    );
  const initial = await keys();
  await page.getByLabel("要素1", { exact: true }).fill("樫の木材");
  await page.getByLabel("要素1", { exact: true }).press("Enter");
  expect(await keys()).toEqual(initial);
  await items
    .first()
    .getByRole("button", { name: "要素を複製", exact: true })
    .click();
  await expect(page.getByLabel("要素2", { exact: true })).toBeFocused();
  const duplicated = await keys();
  expect(new Set(duplicated).size).toBe(3);
  expect(duplicated[0]).toBe(initial[0]);
  expect(duplicated[2]).toBe(initial[1]);
  await items
    .nth(1)
    .getByRole("button", { name: "要素を下へ", exact: true })
    .click();
  expect(await keys()).toEqual([initial[0], initial[1], duplicated[1]]);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  expect(await keys()).toEqual(duplicated);
  await items
    .nth(1)
    .getByRole("button", { name: "要素を削除", exact: true })
    .click();
  await expect(items).toHaveCount(2);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  expect(await keys()).toEqual(duplicated);
  await expect(page.locator(".edit-exit")).toHaveCount(0);
});

test("opening and closing a tab keeps the vertical position of every surviving tab fixed", async () => {
  await page.locator('.file-tab[aria-selected="true"]').dblclick();
  await page.evaluate(() => {
    const tab = document.querySelector(".file-tab")!;
    (window as any).__tabY = [];
    const until = performance.now() + 2000;
    const sample = () => {
      if (!tab.isConnected) return;
      (window as any).__tabY.push(tab.getBoundingClientRect().y);
      if (performance.now() < until) requestAnimationFrame(sample);
    };
    sample();
  });
  await selectSource("rarity");
  await expect(
    page.getByRole("tab", { name: "rarity", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "rarityを閉じる", exact: true })
    .click();
  await expect(page.locator(".file-tab-slot[inert]")).toHaveCount(0);
  const positions = await page.evaluate(
    () => (window as any).__tabY as number[],
  );
  expect(positions.length).toBeGreaterThan(3);
  expect(Math.max(...positions) - Math.min(...positions)).toBeLessThan(1);
});

test("object definitions share bounded headers and whole-item deletion with stable Undo", async () => {
  await page.getByRole("button", { name: "プロジェクト設定", exact: true }).click();
  const first = page.locator(".definition-item").first();
  await expect(page.locator(".ant-drawer-content-wrapper")).not.toHaveClass(/ant-drawer-panel-motion-.*-(?:appear|enter)(?:-|\s|$)/);
  const boundary = (await first.boundingBox())!;
  const remove = first.getByRole("button", { name: "productionを削除", exact: true });
  const button = (await remove.boundingBox())!;
  expect(boundary.x + boundary.width - button.x - button.width).toBeLessThan(20);
  expect(await first.evaluate(el => getComputedStyle(el).borderTopStyle)).toBe("solid");
  await first.getByLabel("プロファイル名").fill("release");
  await first.getByLabel("プロファイル名").press("Enter");
  await expect(first.getByLabel("releaseに含めるタグ")).toBeVisible();
  await first.getByRole("button", { name: "releaseを削除", exact: true }).click();
  await expect(page.getByLabel("releaseに含めるタグ")).toHaveCount(0);
  await expect(page.getByLabel("developmentに含めるタグ")).toBeVisible();
  await page.getByRole("button", { name: "閉じる", exact: true }).click();
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await page.getByRole("button", { name: "プロジェクト設定", exact: true }).click();
  await expect(page.getByLabel("releaseに含めるタグ")).toBeVisible();
  await expect(page.locator(".ant-drawer-content-wrapper")).toBeInViewport({ ratio: .99 });
  await page.screenshot({ path: test.info().outputPath("profile-object-boundaries.png") });
  await page.getByRole("button", { name: "閉じる", exact: true }).click();
  await selectSource("quest");
  await page.getByRole("tab", { name: "キーと参照", exact: true }).click();
  const ref = page.locator(".definition-item").filter({ has: page.getByLabel("参照1の名前", { exact: true }) });
  await expect(ref.locator(".definition-item-header").getByRole("button", { name: "参照1を削除" })).toBeVisible();
  await ref.getByRole("button", { name: "参照1を削除" }).click();
  await expect(page.getByLabel("参照1の名前", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("参照1の名前", { exact: true })).toHaveValue("rewardItem");
});

test("searchable single choices toggle closed on a second empty click and still filter", async () => {
  await selectSource("common");
  const choice = page.getByRole("combobox", { name: "rarity 行1", exact: true });
  await choice.click();
  await expect(choice).toHaveAttribute("aria-expanded", "true");
  await choice.click();
  await expect(choice).toHaveAttribute("aria-expanded", "false");
  await choice.click();
  await choice.fill("Epic");
  await expect(page.locator(".ant-select-item-option-content").filter({ hasText: /^Epic$/ })).toBeVisible();
  await choice.click();
  await expect(choice).toHaveAttribute("aria-expanded", "true");
  await choice.press("Escape");
  await expect(choice).toHaveAttribute("aria-expanded", "false");
  const type = page.getByRole("combobox", { name: "型", exact: true }).first();
  await type.click();
  await type.click();
  await expect(type).toHaveAttribute("aria-expanded", "false");
  const profile = page.getByRole("combobox", { name: "出力プロファイル", exact: true });
  await profile.click();
  await profile.click();
  await expect(profile).toHaveAttribute("aria-expanded", "false");
  await selectSource("quest");
  const reference = page.getByRole("combobox", { name: "rewardItemId 行1", exact: true });
  await reference.click();
  await reference.click();
  await expect(reference).toHaveAttribute("aria-expanded", "false");
});

test("tab and Explorer file menus share draft path operations without changing the active file", async () => {
  await page.evaluate(() => Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: async (value: string) => { (window as any).__copied = value; } } }));
  await selectSource("rarity");
  await page.locator('.tree-name[data-path="types/rarity.yaml"]').dblclick();
  await selectSource("quest");
  const tab = page.locator('.file-tab').filter({ hasText: /^rarity/ });
  await tab.click({ button: "right" });
  const names = await page.getByRole("menuitem").allTextContents();
  expect(names).toEqual(["名前を変更", "パスをコピー", "削除"]);
  await page.getByRole("menuitem", { name: "パスをコピー", exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__copied)).toBe(path.join(await fs.realpath(root), "sources/types/rarity.yaml"));
  await expect(page.locator('.file-tab[aria-selected="true"]')).toContainText("quest");
  await page.getByRole("button", { name: "エクスプローラーの表示切替", exact: true }).click();
  await tab.click({ button: "right" });
  await page.getByRole("menuitem", { name: "名前を変更", exact: true }).click();
  const rename = page.getByRole("textbox", { name: "項目の名前", exact: true });
  await expect(rename).toBeFocused();
  await rename.fill("rarity-renamed");
  await rename.press("Enter");
  const node = page.locator('.tree-name[data-path="types/rarity-renamed.yaml"]');
  await node.click({ button: "right" });
  await expect(page.getByRole("menuitem")).toHaveText(names);
  await page.getByRole("menuitem", { name: "パスをコピー", exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__copied)).toBe(path.join(await fs.realpath(root), "sources/types/rarity-renamed.yaml"));
  await tab.click({ button: "right" });
  await expect(page.getByRole("menuitem")).toHaveText(names);
  await page.getByRole("menuitem", { name: "削除", exact: true }).click();
  await expect(node).toHaveCount(0);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(node).toBeVisible();
});

test("hierarchy drag accepts ancestors and root, shows file insertion lines and rejects descendants", async () => {
  await selectSource("rarity");
  const node = (value: string) => page.locator(`.tree-name[data-path="${value}"]`);
  const target = node("types/outer");
  const move = async (from: string, to: ReturnType<typeof node>, kind: "folder" | "file" | "root") => {
    const data = await page.evaluateHandle(() => new DataTransfer());
    await node(from).dispatchEvent("dragstart", { dataTransfer: data });
    const box = (await to.boundingBox())!;
    const args = { dataTransfer: data, clientX: box.x + 10, clientY: box.y + box.height / 2 };
    await to.dispatchEvent("dragover", args);
    if (kind === "folder") await expect(page.locator(".folder-drop-target")).toHaveCount(1);
    if (kind === "file") await expect(page.locator(".explorer-drop-line")).toBeVisible();
    if (kind === "root") await expect(page.locator(".explorer")).toHaveClass(/root-drop-target/);
    await to.dispatchEvent("drop", args);
    await expect(page.locator(".folder-drop-target")).toHaveCount(0);
    await expect(page.locator(".explorer-drop-line")).toBeHidden();
  };
  await target.click();
  await node("types/outer/inner").click();
  await move("types/outer/inner/deep-folder-with-a-very-long-name", target, "folder");
  await expect(node("types/outer/deep-folder-with-a-very-long-name")).toBeVisible();
  await move("types/outer/deep-folder-with-a-very-long-name", page.locator(".explorer-heading"), "root");
  await expect(node("deep-folder-with-a-very-long-name")).toBeVisible();
  await move("deep-folder-with-a-very-long-name", node("types/rarity.yaml"), "file");
  await expect(node("types/deep-folder-with-a-very-long-name")).toBeVisible();
  const data = await page.evaluateHandle(() => new DataTransfer());
  await node("types").dispatchEvent("dragstart", { dataTransfer: data });
  const box = (await target.boundingBox())!;
  const args = { dataTransfer: data, clientX: box.x + 10, clientY: box.y + box.height / 2 };
  await target.dispatchEvent("dragover", args);
  await expect(page.locator(".folder-drop-target")).toHaveCount(0);
  await target.dispatchEvent("drop", args);
  await expect(node("types/outer")).toBeVisible();
  const guide = await target.locator('xpath=ancestor::*[contains(@class,"ant-tree-treenode")][1]').evaluate(el => getComputedStyle(el, "::before").backgroundImage);
  expect(guide).toContain("linear-gradient");
  expect(await target.locator('xpath=ancestor::*[contains(@class,"ant-tree-treenode")][1]').evaluate(el => parseFloat(getComputedStyle(el, "::before").height))).toBe(28);
  await page.screenshot({ path: test.info().outputPath("explorer-hierarchy-guides.png") });
});

test("hierarchy drop reasons explain conflicts and descendants without changing drafts, and rename recovers", async () => {
  await page.emulateMedia({ colorScheme: "dark" });
  const node = (value: string) => page.locator(`.tree-name[data-path="${value}"]`);
  const feedback = page.locator(".explorer-drop-feedback");
  const original = await fs.readFile(path.join(root, "sources/types/rarity.yaml"), "utf8");
  const existing = await fs.readFile(path.join(root, "sources/types/target/rarity.yaml"), "utf8");
  await node("types").click();
  await node("types/target").click();
  await node("types/incoming").click();
  await page.getByRole("separator", { name: "エクスプローラーの幅", exact: true }).press("Home");
  await expect.poll(() => page.locator(".tree-scroll").evaluate(el =>
    el.getAnimations({ subtree: true }).filter(a => a.playState === "running").length,
  )).toBe(0);
  const data = await page.evaluateHandle(() => new DataTransfer());
  const start = async (from: string) => node(from).dispatchEvent("dragstart", { dataTransfer: data });
  const over = async (target: ReturnType<typeof node>) => {
    const box = (await target.boundingBox())!;
    const args = { dataTransfer: data, clientX: box.x + 10, clientY: box.y + box.height / 2 };
    await target.dispatchEvent("dragover", args);
    return args;
  };
  await start("types/rarity.yaml");
  const before = (await node("types/target").boundingBox())!.y;
  const args = await over(node("types/target"));
  await expect(feedback).toContainText("移動できません");
  await expect(feedback.getByRole("status")).toHaveCount(1);
  await expect(feedback.getByRole("alert")).toHaveCount(0);
  await expect(feedback).toContainText("移動先「types/target」");
  await expect(feedback).toContainText("「rarity.yaml」");
  await expect(feedback).toContainText("名前を変更するか、別のフォルダ");
  await expect(page.locator(".folder-drop-target")).toHaveCount(0);
  expect((await node("types/target").boundingBox())!.y).toBe(before);
  expect(await feedback.evaluate(el => getComputedStyle(el).pointerEvents)).toBe("none");
  const noticeBox = (await feedback.boundingBox())!;
  const explorerBox = (await page.locator(".explorer").boundingBox())!;
  expect(noticeBox.x).toBeGreaterThanOrEqual(explorerBox.x);
  expect(noticeBox.x + noticeBox.width).toBeLessThanOrEqual(explorerBox.x + explorerBox.width);
  await page.screenshot({ path: test.info().outputPath("drop-conflict-dark.png") });
  await node("types/target").dispatchEvent("drop", args);
  await expect(feedback).toHaveCount(0);
  await expect(page.locator(".ant-message-notice")).toContainText("「rarity.yaml」");
  await expect(page.getByRole("status", { name: "保存済み", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "元に戻す", exact: true })).toBeDisabled();
  expect(await fs.readFile(path.join(root, "sources/types/rarity.yaml"), "utf8")).toBe(original);
  expect(await fs.readFile(path.join(root, "sources/types/target/rarity.yaml"), "utf8")).toBe(existing);

  await start("types/rarity.yaml");
  await over(page.locator(".explorer-heading"));
  await expect(feedback).toContainText("移動先「ルートフォルダ」");
  await over(node("types/target/other.yaml"));
  await expect(feedback).toContainText("移動先「types/target」");
  await over(node("types/incoming"));
  await expect(feedback).toHaveCount(0);
  await expect(page.locator(".folder-drop-target")).toHaveCount(1);
  await over(node("types/target"));
  await page.locator(".explorer").dispatchEvent("dragleave", { relatedTarget: null });
  await expect(feedback).toHaveCount(0);
  await over(node("types/target"));
  await node("types/rarity.yaml").dispatchEvent("dragend", { dataTransfer: data });
  await expect(feedback).toHaveCount(0);

  await start("types/incoming/clash-folder");
  await over(node("types/target"));
  await expect(feedback).toContainText("「clash-folder」");
  await page.emulateMedia({ colorScheme: "light" });
  await page.screenshot({ path: test.info().outputPath("drop-conflict-light.png") });
  await over(node("types/incoming/clash-folder"));
  await expect(feedback).toContainText("自身や、その配下");
  await node("types/incoming/clash-folder").dispatchEvent("dragend", { dataTransfer: data });
  await start("types/incoming");
  await over(node("types/incoming/clash-folder"));
  await expect(feedback).toContainText("自身や、その配下");
  await node("types/incoming").dispatchEvent("dragend", { dataTransfer: data });
  await expect(page.getByRole("button", { name: "元に戻す", exact: true })).toBeDisabled();

  await node("types/rarity.yaml").click();
  await page.keyboard.press("F2");
  const rename = page.getByRole("textbox", { name: "項目の名前", exact: true });
  await rename.fill("rarity-moved");
  await rename.press("Enter");
  await node("types/rarity-moved.yaml").dragTo(node("types/target"));
  await expect(node("types/target/rarity-moved.yaml")).toBeVisible();
  await expect(feedback).toHaveCount(0);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(node("types/rarity-moved.yaml")).toBeVisible();
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(node("types/rarity.yaml")).toBeVisible();
  await expect(page.getByRole("status", { name: "保存済み", exact: true })).toBeVisible();
});

test("hierarchy drop reasons appear during a native forbidden drag even without a drop event", async () => {
  const node = (value: string) => page.locator(`.tree-name[data-path="${value}"]`);
  await node("types").click();
  await expect.poll(() => page.locator(".tree-scroll").evaluate(el =>
    el.getAnimations({ subtree: true }).filter(a => a.playState === "running").length,
  )).toBe(0);
  // rc-tree can prepare its expansion after an empty getAnimations() result.
  // Let locator actionability track the actual source / target positions;
  // coordinates captured before that prepare phase can grab a different file.
  await node("types/rarity.yaml").hover({ position: { x: 5, y: 10 } });
  await page.mouse.down();
  try {
    // Cross the native drag threshold while still inside the source, rather
    // than jumping straight out of it (WebKit can otherwise omit dragstart).
    await node("types/rarity.yaml").hover({ position: { x: 13, y: 10 } });
    await node("types/target").hover({ position: { x: 8, y: 10 } });
    // WebKit requires a second move for dragover after the initial dragenter.
    await node("types/target").hover({ position: { x: 9, y: 10 } });
    await expect(page.locator(".explorer-drop-feedback")).toContainText("移動先「types/target」");
  } finally {
    await page.mouse.up();
  }
  await expect(node("types/rarity.yaml")).toBeVisible();
  await expect(page.locator(".explorer-drop-feedback")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "元に戻す", exact: true })).toBeDisabled();
});

test("hierarchy guides end at direct children and follow moves, Undo, filtering and virtual reuse", async () => {
  await page.emulateMedia({ colorScheme: "dark" });
  const node = (value: string) => page.locator(`.tree-name[data-path="${value}"]`);
  const row = (value: string) => node(value).locator('xpath=ancestor::*[@role="treeitem"][1]');
  const expand = async (value: string) => {
    await expect(row(value)).toBeVisible();
    if (await row(value).getAttribute("aria-expanded") === "false") await node(value).click();
  };
  const settled = async () => expect.poll(() => page.locator(".tree-scroll").evaluate(el =>
    el.getAnimations({ subtree: true }).filter(a => a.playState === "running").length,
  )).toBe(0);
  const deep = "types/outer/inner/deep-folder-with-a-very-long-name";
  const segments = (value: string) => row(value).evaluate(el => {
    const style = getComputedStyle(el, "::before");
    if (style.backgroundImage === "none") return [];
    const positions = style.backgroundPosition.split(",");
    return style.backgroundSize.split(",").map((size, i) => {
      const [x, y] = positions[i].trim().split(/\s+/).map(parseFloat);
      const [width, height] = size.trim().split(/\s+/).map(parseFloat);
      return { x, y, width, height };
    });
  });
  const expectLeaf = async (value: string, depth: number, continuing: number[]) => {
    await expect(node(value)).toBeVisible();
    // Expansion copies can be visible before rc-tree replaces them. Measure
    // the final virtual row, not an icon being detached between geometry reads.
    await expect(page.locator(".ant-tree-treenode-motion")).toHaveCount(0);
    await expect.poll(() => segments(value)).toEqual([
      ...continuing.map(x => ({ x, y: 0, width: 1, height: 28 })),
      { x: (depth - 1) * 16 + 8, y: 0, width: 1, height: 14 },
      { x: (depth - 1) * 16 + 8, y: 14, width: 21, height: 1 },
    ]);
    const guides = await segments(value);
    const elbow = guides.at(-1)!;
    const box = (await row(value).boundingBox())!;
    const icon = (await row(value).locator(".ant-tree-iconEle").boundingBox())!;
    expect(box.x + elbow.x + elbow.width).toBeLessThan(icon.x);
  };
  await node("types").click();
  await node("types/outer").click();
  await node("types/outer/inner").click();
  await node(deep).click();
  await expectLeaf(`${deep}/leaf.yaml`, 4, [8, 40]);
  // The sole child folder ends its parent's branch at its own centre. Its
  // descendants must not inherit a line from that finished branch.
  await expect.poll(() => segments("types/outer/inner")).toEqual([
    { x: 8, y: 0, width: 1, height: 28 },
    { x: 24, y: 0, width: 1, height: 14 },
    { x: 24, y: 14, width: 10, height: 1 },
  ]);
  const elbow = (await segments("types/outer/inner")).at(-1)!;
  const innerBox = (await row("types/outer/inner").boundingBox())!;
  const triangle = (await row("types/outer/inner").locator(".ant-tree-switcher .anticon").boundingBox())!;
  expect(innerBox.x + elbow.x + elbow.width).toBeLessThan(triangle.x);
  await settled();
  await page.screenshot({ path: test.info().outputPath("guides-deep.png") });

  await node(deep).dragTo(page.locator(".explorer-heading"));
  const moved = "deep-folder-with-a-very-long-name";
  await expect(node(moved)).toBeVisible();
  expect(await segments(moved)).toEqual([]);
  await expectLeaf(`${moved}/leaf.yaml`, 1, []);
  await settled();
  await page.screenshot({ path: test.info().outputPath("guides-moved.png") });
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expand(deep);
  await expectLeaf(`${deep}/leaf.yaml`, 4, [8, 40]);
  await page.getByRole("button", { name: "やり直す", exact: true }).click();
  await expand(moved);
  await expectLeaf(`${moved}/leaf.yaml`, 1, []);

  await page.getByLabel("表・型を検索").fill("leaf");
  await expectLeaf(`${moved}/leaf.yaml`, 1, []);
  await page.getByLabel("表・型を検索").fill("");
  // Row recycling must not leave the root leaf's old geometry on deep nodes
  // or draw past the final child after scrolling back through a large folder.
  const holder = page.locator(".ant-tree-list-holder");
  await holder.evaluate(el => { el.scrollTop = el.scrollHeight; });
  await expectLeaf("types/z-virtual-64.yaml", 1, []);
  await holder.evaluate(el => { el.scrollTop = 0; });
  await expectLeaf(`${moved}/leaf.yaml`, 1, []);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expand(deep);
  await expectLeaf(`${deep}/leaf.yaml`, 4, [8, 40]);
  await node("types/outer/inner").click();
  await expect(node(`${deep}/leaf.yaml`)).toHaveCount(0);
  await node("types/outer/inner").click();
  await expectLeaf(`${deep}/leaf.yaml`, 4, [8, 40]);
  await page.emulateMedia({ colorScheme: "light" });
  await settled();
  await page.screenshot({ path: test.info().outputPath("guides-undo-light.png") });
});

test("large frozen row utilities select records and stay accessible at the horizontal end", async () => {
  await selectSource("announcement");
  await expect(page.getByRole("combobox", { name: "追加先ファイル", exact: true })).toHaveCount(0);
  const action = page.getByRole("button", { name: "行1の操作", exact: true });
  const tags = page.getByRole("combobox", { name: "行1のタグ", exact: true });
  const before = (await action.boundingBox())!.x;
  await page.locator(".grid-editor").hover();
  await page.mouse.wheel(16000, 0);
  await expect.poll(async () => (await action.boundingBox())!.x).toBeCloseTo(before, 0);
  await expect(tags).toBeVisible();
  await page.locator(".row-number").nth(0).click();
  const selections = page.locator('.ant-table-tbody-virtual-holder input[type="checkbox"]');
  await expect(selections.nth(0)).toBeChecked();
  await page.locator(".row-number").nth(2).click({ modifiers: ["Shift"] });
  await expect(selections.nth(1)).toBeChecked();
  await expect(selections.nth(2)).toBeChecked();
  await action.click();
  await expect(page.getByRole("menuitem", { name: /削除/ })).toBeVisible();
  await page.keyboard.press("Escape");
  // Escape clears grid selection as well as dismissing the row menu.
  await page.locator(".row-number").nth(0).click();
  await page.locator(".row-number").nth(2).click({ modifiers: ["Shift"] });
  await page.locator(".row-number").nth(1).click({ modifiers: ["ControlOrMeta"] });
  await expect(selections.nth(1)).not.toBeChecked();
  await page.getByRole("button", { name: "行2の操作", exact: true }).click();
  await page.getByRole("menuitem", { name: /削除/ }).click();
  await page.locator(".row-number").nth(4).click({ modifiers: ["Shift"] });
  await expect(page.locator('.ant-table-tbody-virtual-holder input[type="checkbox"]:checked')).toHaveCount(1);
  const resizer = page.getByRole("separator", { name: "エクスプローラーの幅", exact: true });
  const rect = (await resizer.boundingBox())!;
  expect(await page.evaluate(({x, y}) => document.elementFromPoint(x, y)?.closest('[role="separator"]') !== null, { x: rect.x + rect.width / 2, y: rect.y + 150 })).toBe(true);
});

test("large deletion at virtual scroll ends clips exits below headers and beyond frozen columns", async () => {
  await selectSource("announcement");
  await page.evaluate(() => {
    const animate = Element.prototype.animate;
    (window as any).__heldExits = [];
    (window as any).__deleteMoves = [];
    Element.prototype.animate = function(frames, options) {
      const a = animate.call(this, frames, options);
      if (typeof options === "object" && options.duration === 300 && this.matches("[data-row-key],[data-motion-key]")) (window as any).__deleteMoves.push(frames);
      if (this.classList.contains("edit-exit")) { a.pause(); a.currentTime = 0; (window as any).__heldExits.push(a); }
      return a;
    };
    const holder = document.querySelector<HTMLElement>(".ant-table-tbody-virtual-holder")!;
    holder.scrollTop = holder.scrollHeight;
  });
  await expect(page.getByRole("button", { name: "行400の操作", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "行400の操作", exact: true }).click();
  await page.getByRole("menuitem", { name: /削除/ }).click();
  await expect(page.getByRole("button", { name: "行400の操作", exact: true })).toHaveCount(0);
  const ghost = page.locator(".edit-exit").first();
  await expect(ghost).toBeAttached();
  const viewport = (await page.locator(".ant-table-tbody-virtual-holder").boundingBox())!;
  const exit = (await ghost.boundingBox())!;
  expect(exit.y).toBeGreaterThanOrEqual(viewport.y - 1);
  expect(exit.y + exit.height).toBeLessThanOrEqual(viewport.y + viewport.height + 1);
  await page.evaluate(() => (window as any).__heldExits.forEach((a: Animation) => a.currentTime = 150));
  expect(await ghost.evaluate(el => Number(getComputedStyle(el).opacity))).toBeGreaterThan(.2);
  await expect.poll(() => page.evaluate(() => (window as any).__deleteMoves.length)).toBeGreaterThan(0);
  await page.screenshot({ path: test.info().outputPath("virtual-row-exit.png") });
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.locator(".edit-exit")).toHaveCount(0);
  await page.locator(".grid-editor").hover();
  await page.mouse.wheel(20000, 0);
  await expect(page.getByRole("button", { name: "field59の操作", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "field59の操作", exact: true }).click();
  await page.getByRole("menuitem", { name: /列を削除/ }).click();
  await expect(ghost).toBeAttached();
  const frozen = (await page.locator(".ant-table-header .ant-table-cell-fix-start-shadow").boundingBox())!;
  for (const layer of await page.locator(".edit-exit").all()) {
    const box = (await layer.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(frozen.x + frozen.width - 1);
  }
  await page.evaluate(() => (window as any).__heldExits.forEach((a: Animation) => { if(a.playState !== "idle") a.currentTime = 150; }));
  await page.screenshot({ path: test.info().outputPath("virtual-column-exit.png") });
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.locator(".edit-exit")).toHaveCount(0);
});

test("hierarchy native pointer drag moves a folder to an ancestor and then the root", async () => {
  await selectSource("rarity");
  const node = (value: string) => page.locator(`.tree-name[data-path="${value}"]`);
  await node("types/outer").click();
  await node("types/outer/inner").click();
  await node("types/outer/inner/deep-folder-with-a-very-long-name").dragTo(node("types/outer"));
  const moved = node("types/outer/deep-folder-with-a-very-long-name");
  await expect(moved).toBeVisible();
  await moved.dragTo(page.locator(".explorer-heading"));
  await expect(node("deep-folder-with-a-very-long-name")).toBeVisible();
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(moved).toBeVisible();
});

test("project file cancel and invalid TOML preserve the current dirty document", async () => {
  const message = page.getByLabel("message 行1", { exact: true });
  await message.fill("切替前のdraft");
  await message.press("Tab");
  dialogPath = null;
  await page.getByRole("button", { name: /開く$/ }).click();
  await page
    .getByRole("button", { name: "破棄して続ける", exact: true })
    .click();
  await expect(message).toHaveValue("切替前のdraft");
  const invalidRoot = path.join(root, "invalid");
  await fs.mkdir(invalidRoot);
  dialogPath = path.join(invalidRoot, "masterdata.toml");
  await fs.writeFile(dialogPath, "name = [");
  await page.getByRole("button", { name: /開く$/ }).click();
  await page
    .getByRole("button", { name: "破棄して続ける", exact: true })
    .click();
  await expect(page.getByText(/プロジェクトを開けません/)).toBeVisible();
  await expect(message).toHaveValue("切替前のdraft");
  await saveAll();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  expect(
    await fs.readFile(
      path.join(root, "sources/announcements/announcement.yaml"),
      "utf8",
    ),
  ).toContain("切替前のdraft");
});

test("project settings save to TOML and reopening restores profile, output and display name", async () => {
  await page
    .getByRole("button", { name: "プロジェクト設定", exact: true })
    .click();
  const drawer = page.getByRole("dialog", {
    name: "プロジェクト設定",
    exact: true,
  });
  await drawer.getByLabel("プロジェクト名", { exact: true }).fill("TOMLで管理");
  await drawer.getByLabel("プロジェクト名", { exact: true }).press("Tab");
  await drawer
    .getByLabel("C#の出力先", { exact: true })
    .fill("../Unity/Generated");
  await drawer.getByLabel("C#の出力先", { exact: true }).press("Tab");
  await drawer
    .getByLabel("プロファイル名", { exact: true })
    .first()
    .fill("production.v2");
  await drawer
    .getByLabel("プロファイル名", { exact: true })
    .first()
    .press("Tab");
  await drawer.getByRole("button", { name: "閉じる", exact: true }).click();
  await saveAll();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  const config = readConfig(
    await fs.readFile(path.join(root, "masterdata.toml"), "utf8"),
  );
  expect(config.name).toBe("TOMLで管理");
  expect(config.publish.csharp).toBe("../Unity/Generated");
  expect(config.profiles["production.v2"].exclude).toEqual(["debug", "event"]);
  expect(
    await fs.stat(path.join(root, "masterdata.yaml")).catch(() => null),
  ).toBeNull();
  await page.getByRole("button", { name: /開く$/ }).click();
  await expect(page.getByText("TOMLで管理", { exact: true })).toBeVisible();
  await page
    .getByRole("button", { name: "プロジェクト設定", exact: true })
    .click();
  await expect(
    page.getByLabel("プロファイル名", { exact: true }).first(),
  ).toHaveValue("production.v2");
});

test("rightmost tab exit remains visibly present without relying on a following tab", async () => {
  await page.locator('.file-tab[aria-selected="true"]').dblclick();
  await selectSource("rarity");
  await page.locator('.file-tab[aria-selected="true"]').dblclick();
  for (const name of ["rarity", "announcement"]) {
    const tab = page.getByRole("tab", { name, exact: true });
    await expect(
      tab.locator('xpath=ancestor::*[contains(@class,"file-tab-slot")][1]'),
    ).toHaveCSS("opacity", "1");
    await tab.evaluate((element) => {
      const slot = element.closest<HTMLElement>(".file-tab-slot")!;
      (window as any).__exitInitialWidth = slot.getBoundingClientRect().width;
      (window as any).__exitBackground =
        getComputedStyle(element).backgroundColor;
      (window as any).__exitFrames = new Promise((resolve) => {
        const frames: any[] = [];
        let exitingAt = 0;
        let held = false;
        const start = performance.now();
        const sample = () => {
          if (!slot.isConnected || performance.now() - start > 1500)
            return resolve(frames);
          if (slot.inert) {
            exitingAt ||= performance.now();
            frames.push({
              width: slot.getBoundingClientRect().width,
              opacity: Number(getComputedStyle(slot).opacity),
              hidden: slot.getAttribute("aria-hidden"),
              background: getComputedStyle(element).backgroundColor,
              time: performance.now() - exitingAt,
            });
            if (!held) {
              held = true;
              // Simulate an expensive editor commit after close. Both animated
              // properties must still share the browser's timeline.
              const until = performance.now() + 150;
              while (performance.now() < until) {}
            }
          }
          requestAnimationFrame(sample);
        };
        sample();
      });
    });
    await tab
      .getByRole("button", { name: `${name}を閉じる`, exact: true })
      .click();
    const frames = (await page.evaluate(
      () => (window as any).__exitFrames,
    )) as {
      width: number;
      opacity: number;
      hidden: string;
      time: number;
      background: string;
    }[];
    await fs.writeFile(
      test.info().outputPath(`tab-${name}-exit.json`),
      JSON.stringify(frames),
    );
    const initialWidth = await page.evaluate(
      () => (window as any).__exitInitialWidth as number,
    );
    // Main-thread sampling can skip frames while the compositor keeps fading.
    // Observe a coupled intermediate state and time, not a fixed frame count.
    expect(
      frames.some(
        (f) =>
          f.width > 5 &&
          f.width < initialWidth - 1 &&
          f.opacity > 0.01 &&
          f.opacity < 0.99,
      ),
    ).toBe(true);
    expect(frames.at(-1)!.time).toBeGreaterThan(100);
    expect(frames.every((f) => f.hidden === "true")).toBe(true);
    const background = await page.evaluate(
      () => (window as any).__exitBackground,
    );
    expect(
      frames
        .filter((f) => f.opacity > 0.5)
        .every((f) => f.background === background),
    ).toBe(true);
    await expect(page.locator(".file-tab-slot[inert]")).toHaveCount(0);
  }
});

async function selectTableFile(label: string | RegExp) {
  await page
    .getByRole("combobox", { name: "表示するデータ", exact: true })
    .click();
  await page
    .locator(
      ".ant-select-dropdown:not(.ant-select-dropdown-hidden) .ant-select-item-option-content",
    )
    .filter({ hasText: label })
    .click();
}
async function pasteText(target: ReturnType<Page["locator"]>, text: string) {
  await target.evaluate((element, text) => {
    const clipboardData = new DataTransfer();
    clipboardData.setData("text/plain", text);
    element.dispatchEvent(
      new ClipboardEvent("paste", {
        clipboardData,
        bubbles: true,
        cancelable: true,
      }),
    );
  }, text);
}

test("table file scope controls all record creation without an independent destination", async () => {
  await selectSource("schema");
  const add = page.getByRole("button", { name: /レコードを追加/ });
  await expect(add).toBeDisabled();
  await expect(add).toHaveAccessibleDescription(/1つのファイルを選択/);
  await expect(page.getByLabel("追加先ファイル", { exact: true })).toHaveCount(
    0,
  );
  await page.getByLabel("表を検索", { exact: true }).fill("銅の剣");
  await expect(add).toBeDisabled();
  await page.getByLabel("表を検索", { exact: true }).fill("");
  await page.getByRole("button", { name: "行1の操作", exact: true }).click();
  await expect(page.getByRole("menuitem", { name: /複製/ })).toBeDisabled();
  await page.keyboard.press("Escape");
  const name = page.getByLabel("name 行1", { exact: true });
  await pasteText(name, "超過1\n超過2\n超過3\n超過4");
  await expect(
    page.getByText(/複数ファイルの表示中はレコードを増やせません/),
  ).toBeVisible();
  await expect(name).toHaveValue("銅の剣");
  await pasteText(name, "既存行の編集\n二番目の編集");
  await expect(name).toHaveValue("既存行の編集");
  await selectTableFile(/^common$/);
  await expect(add).toBeEnabled();
  await add.click();
  await expect(page.getByLabel("id 行4", { exact: true })).toBeFocused();
  await page.getByRole("button", { name: "行4の操作", exact: true }).click();
  await page.getByRole("menuitem", { name: /複製/ }).click();
  await expect(page.getByLabel("id 行5", { exact: true })).toBeFocused();
  await pasteText(
    page.getByLabel("name 行5", { exact: true }),
    "貼付け5\n貼付け6",
  );
  await expect(page.getByLabel("name 行6", { exact: true })).toHaveValue(
    "貼付け6",
  );
  await saveAll();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  const common = parse(
    await fs.readFile(path.join(root, "sources/items/common.yaml"), "utf8"),
  );
  const schema = parse(
    await fs.readFile(path.join(root, "sources/items/schema.yaml"), "utf8"),
  );
  expect(common.records).toHaveLength(6);
  expect(schema.records).toHaveLength(0);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(page.getByLabel("name 行6", { exact: true })).toHaveCount(0);
});

test("empty table definition and new data file have unambiguous record creation and save targets", async () => {
  await selectSource("schema");
  await selectTableFile(/schema（テーブル定義ファイル内）/);
  const add = page.getByRole("button", { name: /レコードを追加/ });
  await expect(add).toBeEnabled();
  await add.click();
  await expect(page.getByLabel("id 行1", { exact: true })).toHaveValue("0");
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await page
    .getByRole("button", { name: "テーブルデータファイルを作成", exact: true })
    .click();
  await expect(
    page.getByRole("tab", { name: "new-data1", exact: true }),
  ).toBeVisible();
  await expect(add).toBeEnabled();
  await add.click();
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(
    page.getByRole("status", { name: "保存済み", exact: true }),
  ).toBeVisible();
  expect(
    parse(
      await fs.readFile(
        path.join(root, "sources/items/new-data1.yaml"),
        "utf8",
      ),
    ).records,
  ).toHaveLength(1);
  await selectSource("schema");
  await selectTableFile(/^new-data1$/);
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(add).toBeDisabled();
});

test("default context menus are cancelled across editor surfaces while custom menus and editing remain usable", async () => {
  for (const target of [
    page.locator(".titlebar"),
    page.locator(".grid-editor"),
    page.getByLabel("message 行1", { exact: true }),
  ]) {
    expect(
      await target.evaluate(
        (element) =>
          !element.dispatchEvent(
            new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
          ),
      ),
    ).toBe(true);
  }
  await page
    .getByRole("button", { name: "プロジェクト設定", exact: true })
    .click();
  expect(
    await page
      .locator(".ant-drawer-body")
      .evaluate(
        (element) =>
          !element.dispatchEvent(
            new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
          ),
      ),
  ).toBe(true);
  await page.getByRole("button", { name: "閉じる", exact: true }).click();
  await selectSource("rarity");
  await page
    .getByRole("tab", { name: "rarity", exact: true })
    .click({ button: "right" });
  await expect(
    page.getByRole("menuitem", { name: "パスをコピー", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("menuitem", { name: "パスをコピー", exact: true }),
  ).toBeHidden();
  await page
    .locator('.tree-name[title="types/rarity.yaml"]')
    .click({ button: "right" });
  await expect(
    page.getByRole("menuitem", { name: "名前を変更", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("menuitem", { name: "名前を変更", exact: true }),
  ).toBeHidden();
  const member = page.getByLabel("候補1の名前", { exact: true });
  await member.fill("EditStillWorks");
  await member.press("Enter");
  await page.getByRole("button", { name: "元に戻す", exact: true }).click();
  await expect(member).toHaveValue("Common");
});

test("welcome suppresses the default WebView context menu", async () => {
  expect(
    await page
      .locator(".welcome")
      .evaluate(
        (element) =>
          !element.dispatchEvent(
            new MouseEvent("contextmenu", { bubbles: true, cancelable: true }),
          ),
      ),
  ).toBe(true);
  await expect(
    page.getByRole("button", { name: /プロジェクトを開く/ }),
  ).toBeVisible();
});

test("keyboard tab navigation skips outgoing tabs and reopening during exit stays singular", async () => {
  for (const name of ["announcement", "rarity", "combat-stats"]) {
    await selectSource(name);
    await page.locator('.file-tab[aria-selected="true"]').dblclick();
  }
  const first = page.getByRole("tab", { name: "announcement", exact: true });
  await first.click();
  await page
    .getByRole("button", { name: "rarityを閉じる", exact: true })
    .click();
  await first.press("ArrowRight");
  await expect(
    page.getByRole("tab", { name: "combat-stats", exact: true }),
  ).toBeFocused();
  await page
    .getByRole("button", { name: "combat-statsを閉じる", exact: true })
    .click();
  await selectSource("combat-stats");
  await expect(
    page.getByRole("tab", { name: "combat-stats", exact: true }),
  ).toHaveCount(1);
  await expect(page.locator(".file-tab-slot[inert]")).toHaveCount(0);
  await expect(
    page.getByRole("tab", { name: "combat-stats", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
});
