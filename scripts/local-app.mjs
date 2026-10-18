import { spawn, spawnSync } from "node:child_process";
import { once } from "node:events";
import {
  cp,
  lstat,
  mkdir,
  mkdtemp,
  rename,
  rm,
  readFile,
  writeFile,
} from "node:fs/promises";
import { homedir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const devName = "MasterData [Dev]";
export function locations(
  platform,
  home,
  localAppData,
  target = join(root, "src-tauri/target/release"),
) {
  if (platform === "darwin")
    return {
      source: join(target, "bundle/macos", `${devName}.app`),
      destination: join(home, "Applications", `${devName}.app`),
      executable: "Contents/MacOS/masterdata-desktop",
    };
  if (platform === "win32" && localAppData)
    return {
      source: target,
      destination: join(localAppData, "Programs", "MasterData-Dev"),
      executable: "masterdata-desktop.exe",
    };
  throw new Error(
    "このinstall scriptはmacOS / Windows用です。WindowsではLOCALAPPDATAが必要です。",
  );
}
function execute(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    timeout: 1200000,
    ...options,
  });
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error(`${command} failed (${result.signal ?? result.status})`);
  return result;
}
function npm(args) {
  // Only fixed tokens are passed to cmd.exe; all paths remain cwd or structured arguments.
  if (process.platform === "win32")
    execute(process.env.ComSpec || "cmd.exe", [
      "/d",
      "/s",
      "/c",
      `npm ${args.join(" ")}`,
    ]);
  else execute("npm", args);
}
export function matchingProcesses(text, destination, platform) {
  const target = platform === "win32" ? destination.toLowerCase() : destination;
  if (platform === "win32") {
    const records = JSON.parse(text || "[]");
    return (Array.isArray(records) ? records : [records]).filter(
      (record) =>
        record?.ExecutablePath &&
        (record.ExecutablePath.toLowerCase() === target ||
          record.ExecutablePath.toLowerCase().startsWith(target + "\\")),
    );
  }
  return text.split("\n").flatMap((line) => {
    const match = line.trim().match(/^(\d+)\s+(.+)$/);
    return match && (match[2] === target || match[2].startsWith(target + "/"))
      ? [{ ProcessId: Number(match[1]), ExecutablePath: match[2] }]
      : [];
  });
}
export function assertNotRunning(destination) {
  const windows = process.platform === "win32";
  const command = windows ? "powershell.exe" : "ps";
  const args = windows
    ? [
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "Get-CimInstance Win32_Process | Select-Object ProcessId,ExecutablePath | ConvertTo-Json -Compress",
      ]
    : ["-axo", "pid=,comm="];
  const result = spawnSync(command, args, {
    encoding: "utf8",
    timeout: 15000,
    windowsHide: true,
  });
  if (result.error || result.status !== 0)
    throw new Error("起動中のアプリを確認できません。installを中止しました。");
  if (matchingProcesses(result.stdout, destination, process.platform).length)
    throw new Error(
      `${devName} が起動中です。未保存の変更を保存してアプリを終了し、もう一度実行してください。\n${destination}`,
    );
}
async function exists(file) {
  try {
    await lstat(file);
    return true;
  } catch (e) {
    if (e.code === "ENOENT") return false;
    throw e;
  }
}
export async function install(
  source,
  destination,
  inspect,
  copy = (a, b) => cp(a, b, { recursive: true, dereference: false }),
  beforeReplace = () => {},
  unregister = async () => {},
) {
  await mkdir(dirname(destination), { recursive: true });
  const staging = await mkdtemp(join(dirname(destination), ".masterdata-dev-"));
  const next = join(staging, basename(destination));
  const previous = join(staging, "previous");
  let backup = false;
  let retain = false;
  try {
    await copy(source, next);
    await inspect(next);
    await beforeReplace();
    if (await exists(destination)) {
      await rename(destination, previous);
      backup = true;
    }
    try {
      await rename(next, destination);
    } catch (error) {
      if (backup) {
        try {
          await rename(previous, destination);
        } catch (restore) {
          retain = true;
          throw new Error(
            `復旧に失敗しました。旧版を保持: ${previous}: ${restore.message}`,
            { cause: error },
          );
        }
      }
      throw error;
    }
  } finally {
    try {
      // Starting a staged macOS bundle registers its temporary path, even when
      // that path is hidden. Removing files alone leaves stale launch entries.
      await unregister(next);
      if (!retain) await unregister(previous);
    } finally {
      if (!retain) await rm(staging, { recursive: true, force: true });
    }
  }
}
const lsregister =
  "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
export function macAppRegistration(bundle, unregister = false, run = spawnSync) {
  const result = run(lsregister, [unregister ? "-u" : "-f", bundle], {
    encoding: "utf8",
    timeout: 15000,
  });
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim();
  // Launch Services returns kLSApplicationNotFoundErr when a newly packaged
  // bundle has not been indexed yet. No registration is already the desired
  // result for -u; the same error must still fail an installation's registration.
  const alreadyAbsent = result.status === 1 && unregister &&
    output.startsWith(`failed to scan ${bundle}: -10814\n`) &&
    output.endsWith("from spotlight");
  if (result.error || (result.status !== 0 && !alreadyAbsent)) {
    throw new Error(
      `アプリの${unregister ? "登録解除" : "登録"}に失敗: ${bundle}\n${output}`,
      { cause: result.error },
    );
  }
}
export async function withMacBundle(
  bundle,
  action,
  unregister = (path) => macAppRegistration(path, true),
) {
  async function retire() {
    // Only the known output of this checkout is consumed. Do not discover or
    // delete other installations, or remove a bundle someone is running.
    if (process.platform === "darwin") assertNotRunning(bundle);
    try {
      await unregister(bundle);
    } finally {
      await rm(bundle, { recursive: true, force: true });
    }
  }
  await retire();
  try {
    return await action();
  } finally {
    await retire();
  }
}
export async function smoke(executable) {
  const child = spawn(executable, ["--smoke-test"], {
    cwd: homedir(),
    stdio: ["ignore", "ignore", "pipe"],
    windowsHide: true,
  });
  let stderr = "";
  child.stderr.on("data", (data) => {
    stderr = (stderr + data).slice(-8000);
  });
  let timer;
  try {
    const [code] = await Promise.race([
      once(child, "exit"),
      new Promise((_, reject) => {
        timer = setTimeout(
          () =>
            reject(
              new Error(`起動確認が時間内に終わりませんでした。\n${stderr}`),
            ),
          25000,
        );
      }),
    ]);
    if (code !== 0)
      throw new Error(`起動確認に失敗しました (${code})\n${stderr}`);
  } finally {
    clearTimeout(timer);
    if (child.exitCode === null && child.signalCode === null) {
      child.kill();
      await once(child, "exit").catch(() => {});
    }
  }
}
export async function copyWindows(source, destination, previous) {
  // Preserve unrelated files in the Windows install directory; only replace app-owned resources.
  if (previous && (await exists(previous)))
    await cp(previous, destination, { recursive: true });
  else await mkdir(destination);
  for (const name of [
    "masterdata-desktop.exe",
    "masterdata-host.exe",
    "sample",
    "licenses",
  ]) {
    await rm(join(destination, name), { recursive: true, force: true });
    await cp(join(source, name), join(destination, name), { recursive: true });
  }
}
async function shortcut(destination) {
  const shortcutPath = join(
    process.env.APPDATA,
    "Microsoft/Windows/Start Menu/Programs",
    `${devName}.lnk`,
  );
  await mkdir(dirname(shortcutPath), { recursive: true });
  const env = {
    ...process.env,
    MASTERDATA_SHORTCUT: shortcutPath,
    MASTERDATA_EXECUTABLE: join(destination, "masterdata-desktop.exe"),
    MASTERDATA_INSTALL: destination,
  };
  execute(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      "$s=(New-Object -ComObject WScript.Shell).CreateShortcut($env:MASTERDATA_SHORTCUT); $s.TargetPath=$env:MASTERDATA_EXECUTABLE; $s.WorkingDirectory=$env:MASTERDATA_INSTALL; $s.Save()",
    ],
    { env },
  );
}
async function launch(executable) {
  const child = spawn(executable, [], {
    cwd: homedir(),
    detached: true,
    stdio: "ignore",
    windowsHide: true,
  });
  await once(child, "spawn");
  child.unref();
}
export async function main(args) {
  if (args.length === 1 && args[0] === "--help") {
    console.log(
      "app.command / app.bat [--no-launch]\n現在のcheckoutを検証・buildし、MasterData [Dev]を固定先にinstallして起動します。Gitのpullは行いません。",
    );
    return;
  }
  if (args.some((arg) => arg !== "--no-launch"))
    throw new Error("不明な引数です。--helpを参照してください。");
  const paths = locations(
    process.platform,
    homedir(),
    process.env.LOCALAPPDATA,
  );
  assertNotRunning(paths.destination);
  // Directory lock coordinates invocations from separate checkouts without relying on process names.
  await mkdir(dirname(paths.destination), { recursive: true });
  const lock = paths.destination + ".install-lock";
  try {
    await mkdir(lock);
  } catch (error) {
    throw new Error(
      `別のinstallが実行中か、中断したlockがあります: ${lock}。他のinstallが終了していることを確認してください。`,
      { cause: error },
    );
  }
  try {
    console.log("[依存] Node.js / Rust / .NET SDK");
    execute("cargo", ["--version"]);
    const sdk = execute("dotnet", ["--list-sdks"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "inherit"],
    });
    if (
      !sdk.stdout
        .split("\n")
        .some((line) => Number(line.match(/^(\d+)\./)?.[1]) >= 8)
    )
      throw new Error(".NET 8以降のSDKをインストールしてください。");
    console.log(sdk.stdout.trim());
    npm(["ci"]);
    console.log(
      "[検証] TypeScript / 保存 / .NET compile・reload / native bridge",
    );
    npm(["run", "check"]);
    npm(["test"]);
    npm(["run", "build"]);
    npm(["run", "test:native"]);
    const packageAndInstall = async () => {
      console.log("[package] MasterData [Dev]");
      execute(process.execPath, [
        join(root, "node_modules/@tauri-apps/cli/tauri.js"),
        "build",
        "--ci",
        "--config",
        "src-tauri/tauri.dev.conf.json",
        ...(process.platform === "darwin"
          ? ["--bundles", "app"]
          : ["--no-bundle"]),
        "--",
        "--locked",
      ]);
      console.log(`[install] ${paths.destination}`);
      await install(
        paths.source,
        paths.destination,
        (next) => smoke(join(next, paths.executable)),
        process.platform === "win32"
          ? (source, next) => copyWindows(source, next, paths.destination)
          : undefined,
        () => assertNotRunning(paths.destination),
        process.platform === "darwin"
          ? (bundle) => macAppRegistration(bundle, true)
          : undefined,
      );
    };
    if (process.platform === "darwin") {
      await withMacBundle(paths.source, packageAndInstall);
      macAppRegistration(paths.destination);
    } else await packageAndInstall();
    if (process.platform === "win32") await shortcut(paths.destination);
    if (!args.includes("--no-launch")) {
      console.log("[起動] MasterData [Dev]");
      await launch(join(paths.destination, paths.executable));
    }
    console.log(`開発版: ${paths.destination}`);
  } finally {
    await rm(lock, { recursive: true });
  }
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  main(process.argv.slice(2)).catch((error) => {
    console.error(`\n失敗: ${error.message}`);
    process.exitCode = 1;
  });
