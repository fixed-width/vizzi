const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const {spawn,execFileSync} = require('node:child_process');
const net = require('node:net');

async function main() {
  const repository = path.resolve(__dirname,'../..');
  const binary = path.resolve(process.env.VIZZI_TEST_BINARY || path.join(repository,'target/debug/vizzi'));
  await fs.access(binary);
  const directory = await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-vscode-')));
  const project = path.join(directory,'project');
  const report = path.join(directory,'result.json');
  await fs.mkdir(project);
  const rust = process.argv.includes('--rust');
  if (rust) {
    await fs.writeFile(path.join(project,'Cargo.toml'),'[package]\nname="vizzi_editor_std_test"\nversion="0.1.0"\nedition="2024"\n[lib]\npath="a.rs"\n');
    await fs.writeFile(path.join(project,'a.rs'),'pub fn convert(value: Option<u8>) -> Result<u8, &\'static str> {\n    value.ok_or_else(|| "missing")\n}\n');
    const extension = process.env.VIZZI_RUST_ANALYZER_EXTENSION;
    if (!extension) throw new Error('Set VIZZI_RUST_ANALYZER_EXTENSION to a locally installed rust-analyzer extension directory');
    await fs.cp(extension,path.join(directory,'extensions',path.basename(extension)),{recursive:true});
    const sysroot = execFileSync('rustc',['--print','sysroot'],{encoding:'utf8'}).trim();
    const source = path.join(sysroot,'lib/rustlib/src/rust/library');
    await fs.access(path.join(source,'core/src/option.rs'));
    await fs.mkdir(path.join(directory,'user/User'),{recursive:true});
    await fs.writeFile(path.join(directory,'user/User/settings.json'),JSON.stringify({
      'rust-analyzer.checkOnSave':false,'rust-analyzer.cargo.buildScripts.enable':false,'rust-analyzer.procMacro.enable':false,
      'rust-analyzer.cargo.sysroot':sysroot,'rust-analyzer.cargo.sysrootSrc':source,
      'rust-analyzer.server.extraEnv':{CARGO_NET_OFFLINE:'true',CARGO_BUILD_JOBS:'1',RUSTUP_AUTO_INSTALL:'0'},
    }));
  } else await fs.writeFile(path.join(project,'a.py'),'# fixture\ndef leaf(): pass\ndef run(): leaf()\n');
  const workspace = process.argv.includes('--alias') ? path.join(directory,'alias') : project;
  if (workspace !== project) await fs.symlink(project,workspace,'dir');
  const log = await fs.open(path.join(directory,'launch.log'),'w');
  const code = process.env.VIZZI_VSCODE_EXECUTABLE || 'code';
  let extension = process.env.VIZZI_TEST_EXTENSION_PATH || __dirname;
  const testName = rust?'extension-host-rust.cjs':'extension-host.cjs';
  let testHost = path.join(__dirname,testName);
  if (process.env.VIZZI_TEST_EXTENSION_PATH) {
    await fs.cp(extension,path.join(directory,'extension'),{recursive:true});
    extension = path.join(directory,'extension');
    testHost = path.join(extension,testName);
    await fs.copyFile(path.join(__dirname,testName),testHost);
  }
  const args = ['--verbose','--wait','--no-sandbox','--disable-gpu','--disable-updates','--disable-telemetry',
    '--disable-workspace-trust','--skip-welcome','--skip-release-notes',...(!rust?['--disable-extensions']:[]),
    `--user-data-dir=${path.join(directory,'user')}`,`--extensions-dir=${path.join(directory,'extensions')}`,
    `--extensionDevelopmentPath=${extension}`,`--extensionTestsPath=${testHost}`,workspace];
  const inspect = process.argv.includes('--webview');
  let port;
  if (inspect) {
    const listener = net.createServer();
    await new Promise(resolve=>listener.listen(0,'127.0.0.1',resolve));
    port = listener.address().port;
    await new Promise(resolve=>listener.close(resolve));
    args.unshift(`--remote-debugging-port=${port}`,'--remote-debugging-address=127.0.0.1');
  }
  const headless = process.platform === 'linux' && !process.argv.includes('--visible');
  const child = spawn(headless ? 'xvfb-run' : code,headless ? ['-a',code,...args] : args,{
    stdio:['ignore',log.fd,log.fd],
    env:{...process.env,XDG_CACHE_HOME:path.join(directory,'cache'),XDG_CONFIG_HOME:path.join(directory,'config'),
      VIZZI_TEST_BINARY:binary,VIZZI_TEST_REPORT:report,VIZZI_TEST_WEBVIEW:inspect?'1':''},
  });
  const inspection = inspect ? require('./test-webview.cjs').verifyWebview({port,report})
    .catch(async error=>{
      const result = JSON.stringify({passed:false,error:error.stack});
      await fs.writeFile(report+'.webview',result);
      await fs.writeFile(report+'.webview-reopened',result);
    }) : undefined;
  const timer = setTimeout(()=>child.kill(),120000);
  try {
    await new Promise((resolve,reject)=>{child.once('error',reject);child.once('exit',resolve);});
    await inspection;
    const result = JSON.parse(await fs.readFile(report,'utf8'));
    if (!result.passed) throw new Error(result.error);
  } catch (error) {
    throw new Error(`${error.message}\nVS Code integration artifacts: ${directory}`);
  } finally {clearTimeout(timer);await log.close();}
  await fs.rm(directory,{recursive:true,force:true});
  console.log((rust?'Real VS Code rust-analyzer: standard-library source capture passed.':'VS Code extension host: sharing, definitions/references, reconnect, editor tab, explicit browser handoff, reuse and shutdown passed.')+(inspect?' Webview rendered, followed a call and restored its saved task.':''));
}

main().catch(error=>{console.error(error.message);process.exitCode=1;});
