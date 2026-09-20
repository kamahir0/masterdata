import { beforeEach, expect, test, vi } from "vitest";

const rust = vi.hoisted(() => vi.fn());
vi.mock("../src/web-wasm", () => ({ callRust: rust }));

function file(name: string, source: string) {
  return { kind: "file", name, getFile: async () => ({ text: async () => source }) };
}

function directory(name: string, children: Record<string, unknown>) {
  return {
    kind: "directory", name,
    getFileHandle: async (child: string) => {
      const result = children[child] as { kind?: string } | undefined;
      if (result?.kind !== "file") throw new Error(`missing file: ${child}`);
      return result;
    },
    getDirectoryHandle: async (child: string) => {
      const result = children[child] as { kind?: string } | undefined;
      if (result?.kind !== "directory") throw new Error(`missing folder: ${child}`);
      return result;
    },
    async *values() { yield* Object.values(children); },
  };
}

beforeEach(() => {
  rust.mockReset();
  rust.mockImplementation(async (request: { op: string; source?: string; files?: Array<{ path: string }> }) => {
    if (request.op === "parse_config") return { config: {
      project: { id: "game", name: "Game", version: "1" },
      sources: { roots: ["data"] }, build: { profiles: {} },
    } };
    if (request.op === "analyze") return { files: request.files!.map(({ path }) => ({ path, kind: "data", table: "item", typeName: null, diagnostic: null })) };
    if (request.op === "identity") return { identity: request.source };
    throw new Error(`unexpected ${request.op}`);
  });
});

test("Browser Host reads only selected source roots and rejects path traversal", async () => {
  const root = directory("project", {
    "masterdata.toml": file("masterdata.toml", "config"),
    data: directory("data", { "item.yaml": file("item.yaml", "kind: data") }),
    secret: directory("secret", { "outside.yaml": file("outside.yaml", "private") }),
  });
  Object.assign(window, { showDirectoryPicker: vi.fn().mockResolvedValue(root) });
  const { invokeBrowser } = await import("../src/browser-host");
  const workspace = await invokeBrowser<{ files: Array<{ path: string }> }>("authoring_workspace", { projectPath: "browser-picker" });
  expect(workspace.files.map((entry) => entry.path)).toEqual(["data/item.yaml"]);
  expect(rust.mock.calls.find(([request]) => request.op === "analyze")?.[0].files).toEqual([{ path: "data/item.yaml", source: "kind: data" }]);
  await expect(invokeBrowser("source_content", { relativePath: "../secret/outside.yaml" })).rejects.toMatchObject({ diagnostic: { code: "E-WEB-SOURCE-PATH" } });
  await expect(invokeBrowser("source_content", { relativePath: "secret/outside.yaml" })).rejects.toMatchObject({ diagnostic: { code: "E-WEB-SOURCE-NOT-FOUND" } });
});

test("Browser Host refuses a source root that escapes the selected directory", async () => {
  rust.mockImplementation(async (request: { op: string }) => request.op === "parse_config" ? { config: {
    project: { id: "game", name: "Game", version: "1" }, sources: { roots: ["../outside"] }, build: { profiles: {} },
  } } : { files: [] });
  Object.assign(window, { showDirectoryPicker: vi.fn().mockResolvedValue(directory("project", {
    "masterdata.toml": file("masterdata.toml", "config"),
  })) });
  const { invokeBrowser } = await import("../src/browser-host");
  await expect(invokeBrowser("authoring_workspace", { projectPath: "browser-picker" })).rejects.toMatchObject({ diagnostic: { code: "E-WEB-SOURCE-PATH" } });
});
