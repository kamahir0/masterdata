import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { install, locations, smoke } from './app.mjs';

async function sandbox(t) {
  const path = await mkdtemp(join(tmpdir(), 'masterdata local app '));
  t.after(() => rm(path, { recursive: true, force: true }));
  return path;
}

test('failed staged smoke retains the previous app and unrelated files', async t => {
  const root = await sandbox(t);
  const source = join(root, 'built.exe');
  const destination = join(root, 'user', 'app.exe');
  await mkdir(dirname(destination));
  await writeFile(source, 'new');
  await writeFile(destination, 'previous');
  await writeFile(join(root, 'user/preferences.json'), 'user bytes');
  await assert.rejects(install(source, destination, async staged => {
    assert.equal(await readFile(staged, 'utf8'), 'new');
    throw new Error('crashed');
  }), /crashed/);
  assert.equal(await readFile(destination, 'utf8'), 'previous');
  assert.equal(await readFile(join(root, 'user/preferences.json'), 'utf8'), 'user bytes');
  assert.deepEqual((await readdir(join(root, 'user'))).sort(), ['app.exe', 'preferences.json']);
});

test('missing build artifact cannot remove a previous installation', async t => {
  const root = await sandbox(t);
  const destination = join(root, 'app.exe');
  await writeFile(destination, 'previous');
  await assert.rejects(install(join(root, 'missing'), destination, async () => assert.fail()));
  assert.equal(await readFile(destination, 'utf8'), 'previous');
  assert.deepEqual(await readdir(root), ['app.exe']);
});

test('first install and repeated bundle replacement preserve neighboring data', async t => {
  const root = await sandbox(t);
  const source = join(root, 'built.app');
  const destination = join(root, 'Applications/local.app');
  await mkdir(join(source, 'Contents/MacOS'), { recursive: true });
  await writeFile(join(source, 'Contents/MacOS/app'), 'first', { mode: 0o755 });
  let inspected = 0;
  const inspect = async staged => {
    assert.ok(staged.endsWith('.app'));
    inspected++;
  };
  await install(source, destination, inspect);
  await writeFile(join(root, 'Applications/other.txt'), 'keep');
  await writeFile(join(source, 'Contents/MacOS/app'), 'second');
  await install(source, destination, inspect);
  assert.equal(inspected, 2);
  assert.equal(await readFile(join(destination, 'Contents/MacOS/app'), 'utf8'), 'second');
  assert.equal(await readFile(join(root, 'Applications/other.txt'), 'utf8'), 'keep');
  assert.deepEqual((await readdir(join(root, 'Applications'))).sort(), ['local.app', 'other.txt']);
});

test('platform locations stay within per-user installation directories', () => {
  assert.equal(locations('darwin', '/user', null, '/build').destination,
    join('/user', 'Applications/masterdata-local.app'));
  const windows = locations('win32', '/user', '/local', '/build');
  assert.equal(windows.destination, join('/local', 'Programs/masterdata-local/masterdata-desktop.exe'));
  assert.equal(windows.source, join('/build', 'release/masterdata-desktop.exe'));
  assert.throws(() => locations('win32', '/user', '', '/build'), /LOCALAPPDATA/);
  assert.throws(() => locations('linux', '/user', null, '/build'), /macOS/);
});

test('smoke rejects immediate exit and spawn failure', async t => {
  const root = await sandbox(t);
  await assert.rejects(smoke(process.execPath, 2000), /exited during smoke/);
  await assert.rejects(smoke(join(root, 'missing.exe'), 2000), /ENOENT/);
});

test('native launcher resolves its script from another working directory with spaces', async t => {
  if (!['darwin', 'win32'].includes(process.platform)) return;
  const root = await sandbox(t);
  const directory = join(root, 'launcher with spaces');
  await mkdir(directory);
  const original = dirname(fileURLToPath(import.meta.url));
  const extension = process.platform === 'win32' ? 'bat' : 'command';
  for (const file of ['app.mjs', `app.${extension}`]) {
    await writeFile(join(directory, file), await readFile(join(original, file)));
  }
  const result = process.platform === 'win32'
    ? spawnSync(process.env.ComSpec || 'cmd.exe', ['/d', '/s', '/c', `""${join(directory, 'app.bat')}" --help"`], {
      cwd: root, encoding: 'utf8', windowsVerbatimArguments: true,
    })
    : spawnSync('sh', [join(directory, 'app.command'), '--help'], { cwd: root, encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /Usage:/);
});
