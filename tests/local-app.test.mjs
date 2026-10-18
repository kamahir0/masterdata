import { test } from "node:test";
import assert from "node:assert/strict";
import {
  mkdtemp,
  mkdir,
  writeFile,
  readFile,
  rm,
  readdir,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  install,
  locations,
  matchingProcesses,
  copyWindows,
  withMacBundle,
  macAppRegistration,
} from "../scripts/local-app.mjs";
test("development identity and install target stay stable across builds on macOS and Windows", () => {
  assert.equal(
    locations("darwin", "/Users/a", "").destination,
    join("/Users/a", "Applications", "MasterData [Dev].app"),
  );
  assert.equal(
    locations("win32", "/home", "/local").destination,
    join("/local", "Programs", "MasterData-Dev"),
  );
  assert.equal(
    matchingProcesses(
      "123 /Users/a/Applications/MasterData [Dev].app/Contents/MacOS/masterdata-desktop\n124 /Users/a/Applications/MasterData.app/Contents/MacOS/masterdata-desktop",
      "/Users/a/Applications/MasterData [Dev].app",
      "darwin",
    ).length,
    1,
  );
  assert.equal(
    matchingProcesses(
      JSON.stringify([
        { ExecutablePath: "C:\\Apps\\Dev\\masterdata-desktop.exe" },
        { ExecutablePath: "C:\\Apps\\Dev2\\x.exe" },
      ]),
      "c:\\apps\\dev",
      "win32",
    ).length,
    1,
  );
});

test("macOS unregister accepts an unindexed app, but registration and other failures are reported", () => {
  const bundle = "/tmp/MasterData [Dev].app";
  const absent = () => ({ status: 1, stdout: `failed to scan ${bundle}: -10814\n from spotlight` });
  assert.doesNotThrow(() => macAppRegistration(bundle, true, absent));
  assert.throws(() => macAppRegistration(bundle, false, absent), /登録に失敗/);
  assert.throws(() => macAppRegistration(bundle, true, () => ({ status: 1, stderr: "permission denied" })), /permission denied/);
  assert.throws(() => macAppRegistration(bundle, true, () => ({ status: null, error: new Error("timeout") })), /登録解除に失敗/);
});

test("macOS consumes intermediate bundles after success, package failure and staging failure", async () => {
  const root = await mkdtemp(join(tmpdir(), "masterdata-bundle-"));
  const bundle = join(root, "MasterData [Dev].app");
  const installed = join(root, "installed.app");
  const unregister = async () => {};
  try {
    await mkdir(installed);
    await writeFile(join(installed, "binary"), "previous");
    for (const outcome of ["package-failure", "smoke-failure", "success"]) {
      // A previous interrupted build is reclaimed before packaging starts.
      await mkdir(bundle);
      await writeFile(join(bundle, "stale"), "interrupted");
      const registrations = new Set();
      const action = withMacBundle(
        bundle,
        async () => {
          await assert.rejects(readFile(join(bundle, "stale")), { code: "ENOENT" });
          await mkdir(bundle);
          await writeFile(join(bundle, "binary"), "next");
          if (outcome === "package-failure") throw new Error("package failed");
          await install(
            bundle,
            installed,
            async (staged) => {
              registrations.add(staged);
              if (outcome === "smoke-failure") throw new Error("smoke failed");
            },
            undefined,
            undefined,
            async (staged) => registrations.delete(staged),
          );
        },
        unregister,
      );
      if (outcome === "success") await action;
      else await assert.rejects(action, /failed/);
      assert.equal(registrations.size, 0);
      await assert.rejects(readFile(join(bundle, "binary")), { code: "ENOENT" });
      assert.deepEqual(await readdir(root), ["installed.app"]);
      assert.equal(
        await readFile(join(installed, "binary"), "utf8"),
        outcome === "success" ? "next" : "previous",
      );
    }
  } finally {
    await rm(root, { recursive: true });
  }
});
test("install uses one destination, and failed staging or active-app checks preserve previous app", async () => {
  const root = await mkdtemp(join(tmpdir(), "masterdata-install-"));
  try {
    const source = join(root, "source");
    const destination = join(root, "installed");
    await mkdir(source);
    await writeFile(join(source, "app"), "one");
    await install(source, destination, async () => {});
    await writeFile(join(source, "app"), "two");
    await assert.rejects(
      install(source, destination, async () => {
        throw new Error("smoke failed");
      }),
      /smoke/,
    );
    assert.equal(await readFile(join(destination, "app"), "utf8"), "one");
    await assert.rejects(
      install(
        source,
        destination,
        async () => {},
        undefined,
        () => {
          throw new Error("running");
        },
      ),
      /running/,
    );
    assert.equal(await readFile(join(destination, "app"), "utf8"), "one");
    await install(source, destination, async () => {});
    assert.equal(await readFile(join(destination, "app"), "utf8"), "two");
    assert.deepEqual((await readdir(root)).sort(), ["installed", "source"]);
  } finally {
    await rm(root, { recursive: true });
  }
});

test("Windows app updates include the private host, sample and notices while retaining unrelated files", async () => {
  const root = await mkdtemp(join(tmpdir(), "masterdata-windows-install-"));
  try {
    const source = join(root, "source"),
      previous = join(root, "previous"),
      next = join(root, "next");
    await mkdir(source);
    await mkdir(previous);
    for (const dir of ["sample", "licenses"]) {
      await mkdir(join(source, dir));
      await writeFile(join(source, dir, "content"), "new");
    }
    for (const file of ["masterdata-desktop.exe", "masterdata-host.exe"])
      await writeFile(join(source, file), "binary");
    await writeFile(join(previous, "user-project.yaml"), "preserve");
    await mkdir(join(previous, "sample"));
    await writeFile(join(previous, "sample", "old"), "retired");
    await copyWindows(source, next, previous);
    assert.equal(
      await readFile(join(next, "user-project.yaml"), "utf8"),
      "preserve",
    );
    assert.equal(
      await readFile(join(next, "licenses", "content"), "utf8"),
      "new",
    );
    assert.deepEqual(await readdir(join(next, "sample")), ["content"]);
  } finally {
    await rm(root, { recursive: true });
  }
});
