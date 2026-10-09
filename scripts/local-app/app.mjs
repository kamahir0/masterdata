import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { realpathSync } from 'node:fs';
import { cp, lstat, mkdir, mkdtemp, rename, rm } from 'node:fs/promises';
import { homedir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const script = fileURLToPath(import.meta.url);
const root = resolve(dirname(script), '../..');

function run(command, args, cwd = root, env = process.env) {
  const result = spawnSync(command, args, { cwd, env, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} failed (${result.signal ?? result.status})`);
}

function npm(args) {
  const cwd = join(root, 'desktop/web');
  // npm.cmd needs cmd.exe on Windows. Only fixed command tokens reach this shell;
  // repository paths go through cwd so spaces and shell metacharacters stay data.
  if (process.platform === 'win32') {
    run(process.env.ComSpec || 'cmd.exe', ['/d', '/s', '/c', `npm ${args.join(' ')}`], cwd);
  } else {
    run('npm', args, cwd);
  }
}

export function locations(platform, home, localAppData, target) {
  if (platform === 'darwin') {
    return {
      source: join(target, 'release/bundle/macos/MasterData.app'),
      destination: join(home, 'Applications/masterdata-local.app'),
      executable: 'Contents/MacOS/masterdata-desktop',
    };
  }
  if (platform === 'win32' && localAppData) {
    return {
      source: join(target, 'release/masterdata-desktop.exe'),
      destination: join(localAppData, 'Programs/masterdata-local/masterdata-desktop.exe'),
      executable: '',
    };
  }
  throw new Error('macOS / Windows only; Windows requires LOCALAPPDATA.');
}

async function exists(path) {
  try { await lstat(path); return true; }
  catch (error) { if (error.code === 'ENOENT') return false; throw error; }
}

export async function smoke(executable, duration = 2000) {
  const child = spawn(executable, [], { cwd: homedir(), stdio: 'ignore', windowsHide: true });
  const exit = once(child, 'exit');
  let timer;
  try {
    const result = await Promise.race([
      exit.then(([code, signal]) => { throw new Error(`App exited during smoke (${signal ?? code})`); }),
      new Promise(resolve => { timer = setTimeout(resolve, duration); }),
    ]);
    return result;
  } finally {
    clearTimeout(timer);
    // Stop only the scratch process started here; never terminate an existing GUI.
    if (child.pid && child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
    await exit.catch(() => {});
  }
}

export async function install(source, destination, inspect) {
  await mkdir(dirname(destination), { recursive: true });
  const staging = await mkdtemp(join(dirname(destination), '.masterdata-local-'));
  const next = join(staging, basename(destination));
  const previous = join(staging, 'previous');
  let backedUp = false;
  let keepBackup = false;
  try {
    await cp(source, next, { recursive: true, dereference: false });
    await inspect(next);
    if (await exists(destination)) {
      await rename(destination, previous);
      backedUp = true;
    }
    try { await rename(next, destination); }
    catch (error) {
      if (backedUp) {
        try { await rename(previous, destination); }
        catch (restoreError) {
          keepBackup = true;
          throw new Error(`Restore failed; previous app retained at ${previous}: ${restoreError.message}`, { cause: error });
        }
      }
      throw error;
    }
  } finally {
    // Delete only our unique scratch directory, and retain a backup if rollback fails.
    if (!keepBackup) await rm(staging, { recursive: true, force: true });
  }
}

async function launch(executable) {
  // Direct launch preserves the shell PATH needed by the native .NET boundary.
  const child = spawn(executable, [], {
    cwd: homedir(), detached: true, stdio: 'ignore', windowsHide: true,
  });
  await once(child, 'spawn');
  child.unref();
}

async function main(args) {
  if (args.length === 1 && args[0] === '--help') {
    console.log('Usage: app.command / app.bat [--no-launch]\nDependencies -> verify -> package -> staged smoke -> per-user install -> launch.');
    return;
  }
  if (args.some(arg => arg !== '--no-launch')) throw new Error('Unknown argument; use --help.');
  const target = join(root, 'target');
  const paths = locations(process.platform, homedir(), process.env.LOCALAPPDATA, target);
  console.log('\n[dependencies] Node.js, Rust, npm, .NET SDK');
  if (Number(process.versions.node.split('.')[0]) < 22) throw new Error('Node.js 22 or newer is required.');
  run('cargo', ['--version']);
  const sdk = spawnSync('dotnet', ['--list-sdks'], { encoding: 'utf8' });
  if (sdk.error) throw sdk.error;
  if (sdk.status !== 0 || !sdk.stdout.split('\n').some(line => Number(line.match(/^(\d+)\./)?.[1]) >= 8)) {
    throw new Error('.NET SDK 8 or newer is required.');
  }
  console.log(sdk.stdout.trim());
  npm(['ci']);
  console.log('\n[verify] Frontend and shared Desktop application tests');
  npm(['run', 'build']);
  run('cargo', ['test', '-p', 'masterdata-desktop', '--locked']);
  console.log('\n[package] Production Desktop (evidence feature disabled)');
  const cli = join(root, 'desktop/web/node_modules/@tauri-apps/cli/tauri.js');
  const bundle = process.platform === 'darwin'
    ? ['--bundles', 'app', '--config', '{"bundle":{"active":true}}']
    : ['--no-bundle'];
  run(process.execPath, [cli, 'build', '--ci', '--features', 'host', ...bundle, '--', '--locked'],
    join(root, 'desktop/native'), { ...process.env, CARGO_TARGET_DIR: target });
  console.log(`\n[install] ${paths.destination}`);
  try {
    await install(paths.source, paths.destination, async staged => {
      console.log('[smoke] Staged production app must survive 2 seconds');
      await smoke(paths.executable ? join(staged, paths.executable) : staged);
    });
  } catch (error) {
    throw new Error(`Install failed at ${paths.destination}. Close the local GUI and retry. ${error.message}`, { cause: error });
  }
  if (!args.includes('--no-launch')) {
    console.log('\n[launch] MasterData');
    await launch(paths.executable ? join(paths.destination, paths.executable) : paths.destination);
  }
  console.log(`\nLocal app ready: ${paths.destination}`);
}

if (process.argv[1] && realpathSync(process.argv[1]) === script) {
  main(process.argv.slice(2)).catch(error => {
    console.error(`\nLocal app workflow failed: ${error.message}`);
    process.exitCode = 1;
  });
}
