import assert from 'node:assert/strict';
import {execFile, spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
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
  await page.getByRole('link', {name: 'Call workspace', exact: true}).click();
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

async function workspace(t, files) {
  const directory = await realpath(await mkdtemp(join(tmpdir(), 'vizzi-nvim-')));
  const root = join(directory, 'project');
  await mkdir(root);
  for (const [file, source] of Object.entries(files)) {
    await writeFile(join(root, file), source);
  }
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
  return {directory, root, cleanups, launch: name => launch(cleanups, directory, root, name)};
}

async function editorRequest(url, file) {
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
  return (line = 2) => post('/api/editor/open', {
    snapshot: captured.snapshot, file: captured.file, version: captured.version, line,
  });
}

test('Neovim opens a native workspace and returns to its owning split', {timeout: 30000}, async t => {
  const file = 'literal [name].rs';
  const fixture = await workspace(t, {[file]: '\ufeff// café\r\npub fn entry() {}\r\n'});
  const {directory, root, cleanups} = fixture;
  const target = join(root, file);
  const first = await fixture.launch('first');
  const second = await fixture.launch('second');
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

  const open = process.env.VIZZI_NVIM_BROWSER
    ? await openFromBrowser(url, cleanups)
    : await editorRequest(url, file);
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

test('return preserves an unrelated named buffer with nohidden', {timeout: 15000}, async t => {
  const saved = 'saved on disk\n';
  const fixture = await workspace(t, {'target.rs': '\npub fn entry() {}\n', 'notes.txt': saved});
  const host = await fixture.launch('origin');
  const notes = join(fixture.root, 'notes.txt');
  await host.command('edit notes.txt');
  const buffer = await host.evaluate('vim.api.nvim_get_current_buf()');
  await host.command('VizziOpen');
  const open = await editorRequest(await host.url(), 'target.rs');
  const optionState = 'table.concat({vim.o.hidden and 1 or 0, vim.o.autowrite and 1 or 0, vim.o.autowriteall and 1 or 0}, ":")';
  for (const [options, bufhidden] of [
    ['autowrite noautowriteall', ''],
    ['noautowrite autowriteall', ''],
    ['noautowrite noautowriteall', ''],
    ['autowrite autowriteall', 'wipe'],
  ]) {
    await host.command('buffer ' + buffer);
    await host.command('set nohidden ' + options);
    await host.command('setlocal bufhidden=' + bufhidden);
    await host.command('call setline(1, "unsaved note")');
    const before = await host.evaluate(optionState);
    const response = await open();
    assert.equal(response.status, 200, await response.text());
    assert.equal(await readFile(notes, 'utf8'), saved, options);
    assert.equal(await host.evaluate('vim.fn.getbufline(' + buffer + ', 1)[1]'), 'unsaved note');
    assert.equal(await host.evaluate('vim.fn.getbufvar(' + buffer + ', "&modified")'), '1');
    assert.equal(await host.evaluate('vim.fn.getbufvar(' + buffer + ', "&bufhidden")'), bufhidden);
    assert.equal(await host.evaluate(optionState), before);
  }
});

test('closing the origin inside a non-nested autocommand retires its workspace', {timeout: 15000}, async t => {
  const fixture = await workspace(t, {'lib.rs': 'pub fn entry() {}\n'});
  const host = await fixture.launch('origin');
  for (const reopen of [true, false]) {
    const window = await host.evaluate('vim.api.nvim_get_current_win()');
    await host.command('VizziOpen');
    const url = await host.url(reopen ? 1 : 3);
    await host.command('vnew');
    await host.command('autocmd User VizziCloseSplit ++once lua vim.api.nvim_win_close(' + window + ', true)' +
      (reopen ? '; vim.cmd.VizziOpen()' : ''));
    await host.command('doautocmd User VizziCloseSplit');
    if (reopen) {
      const replacement = await host.url(2);
      const open = await editorRequest(replacement, 'lib.rs');
      const response = await open(1);
      assert.equal(response.status, 200, await response.text());
      if (replacement === url) continue;
    }
    await until(async () => fetch(url).then(() => false, () => true), 'Vizzi kept running after its origin closed');
  }
});

test('return verifies NUL bytes in captured UTF-8 source', {timeout: 15000}, async t => {
  const file = 'lib.rs';
  const fixture = await workspace(t, {[file]: '// literal NUL\npub fn entry() { let _ = b"x\0y"; }\n'});
  const host = await fixture.launch('origin');
  await host.command('VizziOpen');
  const open = await editorRequest(await host.url(), file);
  const response = await open();
  assert.equal(response.status, 200, await response.text());
  assert.equal(await host.evaluate('vim.api.nvim_win_get_cursor(0)[1]'), '2');
});

test('SHA-256 agrees with Node for NULs and block padding boundaries', {timeout: 15000}, async t => {
  const inputs = [Buffer.alloc(0), Buffer.from('saved source'), ...[1, 55, 56, 63, 64, 65, 119, 120, 4097].map(
    size => Buffer.from(Array.from({length: size}, (_, index) => (index * 131) % 256)),
  )];
  const filenames = inputs.map((_, index) => 'input-' + index);
  const fixture = await workspace(t, Object.fromEntries(filenames.map((file, index) => [file, inputs[index]])));
  const script = join(fixture.directory, 'hash.lua');
  await writeFile(script, `
    vim.opt.runtimepath:prepend(vim.env.VIZZI_NVIM_PLUGIN)
    local sha256 = require('vizzi.sha256')
    for _, path in ipairs(arg) do
      local file = assert(io.open(path, 'rb'))
      local bytes = file:read('*a')
      file:close()
      io.stdout:write(sha256(bytes), '\\n')
    end
  `);
  const {stdout} = await run(nvim, ['--headless', '--clean', '-l', script, ...filenames], {
    cwd: fixture.root,
    env: {...process.env, VIZZI_NVIM_PLUGIN: plugin, NVIM_LOG_FILE: join(fixture.directory, 'nvim.log')},
  });
  assert.deepEqual(stdout.trim().split('\n'), inputs.map(bytes => createHash('sha256').update(bytes).digest('hex')));
});
