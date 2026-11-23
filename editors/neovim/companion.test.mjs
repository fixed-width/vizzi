import assert from 'node:assert/strict';
import {execFile, spawn} from 'node:child_process';
import {mkdir, mkdtemp, readFile, realpath, rm, writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join, resolve} from 'node:path';
import {test} from 'node:test';
import {setTimeout as delay} from 'node:timers/promises';
import {fileURLToPath} from 'node:url';
import {promisify} from 'node:util';

const run = promisify(execFile);
const nvim = process.env.VIZZI_NVIM_EXECUTABLE || 'nvim';
const binary = resolve(process.env.VIZZI_TEST_BINARY || 'target/release/vizzi');
const plugin = fileURLToPath(new URL('.', import.meta.url));

async function until(observe, message) {
  const deadline = Date.now() + 5000;
  do {
    const result = await observe();
    if (result) return result;
    await delay(20);
  } while (Date.now() < deadline);
  assert.fail(message);
}

async function launch(cleanups, directory, root, name) {
  const runtime = join(directory, name);
  await mkdir(runtime, {mode: 0o700});
  const socket = join(runtime, 'rpc');
  const urls = join(runtime, 'urls');
  await writeFile(urls, '');
  const host = spawn(nvim, ['--headless', '-u', join(directory, 'init.lua'), '-i', 'NONE', '--listen', socket], {
    cwd: root,
    env: {
      ...process.env,
      XDG_RUNTIME_DIR: runtime,
      XDG_CACHE_HOME: join(directory, 'cache'),
      NVIM_LOG_FILE: join(runtime, 'nvim.log'),
      VIZZI_NVIM_PLUGIN: plugin,
      VIZZI_NVIM_URLS: urls,
      VIZZI_TEST_BINARY: binary,
    },
  });
  let diagnostics = '';
  host.on('error', error => {diagnostics += error.message;});
  host.stderr.on('data', chunk => {diagnostics += chunk;});
  const exited = new Promise(resolveExit => host.once('close', resolveExit));
  const quit = async () => {
    if (host.exitCode !== null || host.signalCode !== null) return;
    await run(nvim, ['--server', socket, '--remote-send', '<Cmd>qa!<CR>']).catch(() => host.kill());
    await exited;
  };
  cleanups.push(quit);
  const evaluate = async lua => {
    const expr = 'luaeval(' + JSON.stringify(lua) + ')';
    const {stdout} = await run(nvim, ['--server', socket, '--remote-expr', expr]);
    return stdout.trim();
  };
  await until(() => evaluate('1').catch(() => ''), 'Neovim did not start');
  assert.equal(await evaluate('vim.fn.exists(":VizziOpen")'), '2', diagnostics);
  return {
    evaluate,
    command: command => evaluate('(function() vim.cmd(' + JSON.stringify(command) + '); return true end)()'),
    url: async (count = 1) => until(async () => {
      const opened = (await readFile(urls, 'utf8')).trim().split('\n').filter(Boolean);
      return opened[count - 1];
    }, 'Vizzi did not open a browser URL. ' + diagnostics),
    quit,
  };
}

async function openFromBrowser(url, cleanups) {
  const {browser} = await import('../../experiments/browser/support.mjs');
  const chromium = await browser({desktopBus: false});
  cleanups.push(() => chromium.close());
  const page = await chromium.newPage();
  await page.goto(url);
  await page.waitForFunction(() => document.querySelector('#visual-source-status').textContent.includes('ready'));
  await page.getByRole('button', {name: 'Add bubble', exact: true}).click();
  await page.getByLabel('Find a function', {exact: true}).fill('entry');
  await page.getByRole('button', {name: 'Find functions', exact: true}).click();
  await page.locator('[data-add-symbol][data-name="entry"]').click();
  const entry = page.locator('[data-view][data-name="entry"]');
  await entry.locator('.code-line').first().waitFor();
  const button = entry.getByRole('button', {name: 'Open in Neovim', exact: true});
  return async () => {
    const waiting = page.waitForResponse(response => new URL(response.url()).pathname === '/api/editor/open');
    await button.click();
    const response = await waiting;
    return {status: response.status(), text: () => response.text()};
  };
}

test('Neovim opens a native workspace and returns to its owning split', {timeout: 30000}, async t => {
  const directory = await realpath(await mkdtemp(join(tmpdir(), 'vizzi-nvim-')));
  const root = join(directory, 'project');
  await mkdir(root);
  const file = 'literal [name].rs';
  const target = join(root, file);
  await writeFile(target, '\ufeff// café\r\npub fn entry() {}\r\n');
  await writeFile(join(directory, 'init.lua'), `
    vim.opt.runtimepath:prepend(vim.env.VIZZI_NVIM_PLUGIN)
    vim.g.vizzi_executable = vim.env.VIZZI_TEST_BINARY
    vim.ui.open = function(url)
      vim.fn.writefile({url}, vim.env.VIZZI_NVIM_URLS, 'a')
      return {}
    end
  `);
  const cleanups = [];
  t.after(async () => {
    try {
      await Promise.all(cleanups.map(cleanup => cleanup()));
    } finally {
      await rm(directory, {recursive: true, force: true});
    }
  });
  const first = await launch(cleanups, directory, root, 'first');
  const second = await launch(cleanups, directory, root, 'second');
  const originWindow = await first.evaluate('vim.api.nvim_get_current_win()');
  const unrelated = await first.evaluate('vim.api.nvim_get_current_buf()');
  await first.command('VizziOpen');
  const url = await first.url();
  await first.command('vnew');
  const otherWindow = await first.evaluate('vim.api.nvim_get_current_win()');
  assert.notEqual(otherWindow, originWindow);
  await first.evaluate('vim.api.nvim_buf_set_lines(' + unrelated + ', 0, -1, false, {"unrelated edit"})');
  await first.command('VizziOpen');
  assert.equal(await first.url(2), url);
  const config = join(directory, 'vizzi.toml');
  await writeFile(config, 'root = "unused"\nport = 0\nworkers = 1\n');
  await second.evaluate('(function() vim.g.vizzi_config = ' + JSON.stringify(config) + '; return true end)()');
  await second.command('VizziOpen');
  const otherUrl = await second.url();
  assert.notEqual(otherUrl, url);
  assert.equal((await (await fetch(new URL('/api/investigation', url))).json()).editor, 'Neovim');

  const post = (path, body) => fetch(new URL(path, url), {
    method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(body),
  });
  assert.ok((await post('/api/workspace/start', {})).ok);
  const lookup = new URL('/api/investigation/lookup', url);
  lookup.searchParams.set('path', file);
  const captured = await until(async () => {
    const response = await fetch(lookup);
    if (response.ok) return response.json();
    await response.text();
  }, 'Native capture did not include the target file');
  const open = process.env.VIZZI_NVIM_BROWSER
    ? await openFromBrowser(url, cleanups)
    : () => post('/api/editor/open', {
      snapshot: captured.snapshot, file: captured.file, version: captured.version, line: 2,
    });
  let response = await open();
  assert.equal(response.status, 200, await response.text());
  assert.equal(await first.evaluate('vim.api.nvim_get_current_win()'), originWindow);
  assert.equal(await first.evaluate('vim.api.nvim_buf_get_name(0)'), target);
  assert.equal(await first.evaluate('vim.api.nvim_win_get_cursor(0)[1]'), '2');
  assert.equal(await first.evaluate('vim.api.nvim_buf_get_lines(' + unrelated + ', 0, 1, false)[1]'), 'unrelated edit');
  assert.equal(await first.evaluate('vim.api.nvim_buf_get_name(vim.api.nvim_win_get_buf(' + otherWindow + '))'), '');
  assert.equal(await second.evaluate('vim.api.nvim_buf_get_name(0)'), '');

  await first.command('call cursor(1, 1)');
  await first.command('call setline(1, "// modified")');
  response = await open();
  assert.equal(response.status, 502);
  assert.match(await response.text(), /unsaved changes/i);
  assert.equal(await first.evaluate('vim.api.nvim_win_get_cursor(0)[1]'), '1');
  assert.equal(await first.evaluate('vim.fn.getline(1)'), '// modified');
  await first.command('setlocal nomodified');
  response = await open();
  assert.equal(response.status, 502);
  assert.match(await response.text(), /differs/i);
  assert.equal(await first.evaluate('vim.api.nvim_win_get_cursor(0)[1]'), '1');

  const targetBuffer = await first.evaluate('vim.api.nvim_get_current_buf()');
  await first.command('buffer ' + unrelated);
  await first.command('bwipeout! ' + targetBuffer);
  const loaded = await first.evaluate('(function() local buffer = vim.fn.bufadd(' + JSON.stringify(target) +
    '); vim.fn.bufload(buffer); return buffer end)()');
  await first.command('autocmd BufWinEnter <buffer=' + loaded + '> ++once call setline(1, "// changed during presentation")');
  response = await open();
  assert.equal(response.status, 502);
  assert.match(await response.text(), /unsaved changes/i);
  assert.equal(await first.evaluate('vim.api.nvim_win_get_cursor(0)[1]'), '1');
  assert.equal(await first.evaluate('vim.fn.getline(1)'), '// changed during presentation');

  await first.command('VizziStop');
  await until(async () => fetch(url).then(() => false, () => true), 'Vizzi kept running after Stop');
  assert.ok((await fetch(otherUrl)).ok);
  await first.command('VizziOpen');
  const restartedUrl = await first.url(3);
  const replacement = join(directory, 'other project');
  await mkdir(replacement);
  const escaped = await first.evaluate('vim.fn.fnameescape(' + JSON.stringify(replacement) + ')');
  await first.command('VizziOpen ' + escaped);
  const replacementUrl = await first.url(4);
  await until(async () => fetch(restartedUrl).then(() => false, () => true), 'Vizzi kept running after changing projects');
  await first.command('close!');
  await until(async () => fetch(replacementUrl).then(() => false, () => true), 'Vizzi kept running after its split closed');
  await second.quit();
  await until(async () => fetch(otherUrl).then(() => false, () => true), 'Vizzi kept running after Neovim quit');
  await first.quit();
});
