import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export interface CliOptions {
  cwd?: string;
  stdin?: string;
}

export interface CliResult {
  exitCode: number;
  stdout: string;
  stderr: string;
  data?: unknown;
}

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
let builtBinaryPath: string | null = null;

export function ensureRustCliBinary(): string {
  if (builtBinaryPath && fs.existsSync(builtBinaryPath)) {
    return builtBinaryPath;
  }
  const ext = process.platform === "win32" ? ".exe" : "";
  const binPath = path.join(repoRoot, "src-tauri", "target", "debug", `masterdata${ext}`);
  const build = spawnSync(
    "cargo",
    ["build", "--manifest-path", path.join(repoRoot, "src-tauri", "Cargo.toml"), "--bin", "masterdata"],
    {
      cwd: repoRoot,
      encoding: "utf8",
      timeout: 300000,
    },
  );
  if (build.error) throw build.error;
  if (build.status !== 0) {
    throw new Error(`Failed to build Rust masterdata CLI:\n${build.stderr || build.stdout}`);
  }
  builtBinaryPath = binPath;
  return binPath;
}

export async function runCli(
  argv: string[],
  options: CliOptions = {},
): Promise<CliResult> {
  const bin = ensureRustCliBinary();
  const cwd = options.cwd ?? process.cwd();

  return await new Promise<CliResult>((resolve, reject) => {
    const child = spawn(bin, argv, {
      cwd,
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
    });

    const stdoutChunks: Buffer[] = [];
    const stderrChunks: Buffer[] = [];

    child.stdout.on("data", (chunk: Buffer) => stdoutChunks.push(Buffer.from(chunk)));
    child.stderr.on("data", (chunk: Buffer) => stderrChunks.push(Buffer.from(chunk)));
    child.on("error", reject);

    if (options.stdin !== undefined) {
      child.stdin.write(options.stdin);
    }
    child.stdin.end();

    child.on("close", (code) => {
      const stdout = Buffer.concat(stdoutChunks).toString("utf8");
      const stderr = Buffer.concat(stderrChunks).toString("utf8");
      let data: unknown = undefined;
      if (argv.includes("--json") && stdout.trim()) {
        try {
          data = JSON.parse(stdout);
        } catch {
          // Keep undefined if output is not valid JSON
        }
      }
      resolve({
        exitCode: code ?? 1,
        stdout,
        stderr,
        data,
      });
    });
  });
}
