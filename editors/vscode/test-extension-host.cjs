const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const {spawn} = require('node:child_process');

async function main() {
  const repository = path.resolve(__dirname,'../..');
  const binary = path.resolve(process.env.VIZZI_TEST_BINARY || path.join(repository,'target/debug/vizzi'));
  await fs.access(binary);
  const directory = await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(),'vizzi-vscode-')));
  const project = path.join(directory,'project');
  const report = path.join(directory,'result.json');
  await fs.mkdir(project);
  await fs.writeFile(path.join(project,'a.py'),'# fixture\ndef leaf(): pass\ndef run(): leaf()\n');
  const workspace = process.argv.includes('--alias') ? path.join(directory,'alias') : project;
  if (workspace !== project) await fs.symlink(project,workspace,'dir');
  const log = await fs.open(path.join(directory,'launch.log'),'w');
  const code = process.env.VIZZI_VSCODE_EXECUTABLE || 'code';
  const extension = process.env.VIZZI_TEST_EXTENSION_PATH || __dirname;
  const args = ['--verbose','--wait','--no-sandbox','--disable-gpu','--disable-updates','--disable-telemetry',
    '--disable-workspace-trust','--skip-welcome','--skip-release-notes','--disable-extensions',
    `--user-data-dir=${path.join(directory,'user')}`,`--extensions-dir=${path.join(directory,'extensions')}`,
    `--extensionDevelopmentPath=${extension}`,`--extensionTestsPath=${path.join(__dirname,'extension-host.cjs')}`,workspace];
  const headless = process.platform === 'linux' && !process.env.DISPLAY;
  const child = spawn(headless ? 'xvfb-run' : code,headless ? ['-a',code,...args] : args,{
    stdio:['ignore',log.fd,log.fd],
    env:{...process.env,XDG_CACHE_HOME:path.join(directory,'cache'),XDG_CONFIG_HOME:path.join(directory,'config'),
      VIZZI_TEST_BINARY:binary,VIZZI_TEST_REPORT:report},
  });
  const timer = setTimeout(()=>child.kill(),120000);
  try {
    await new Promise((resolve,reject)=>{child.once('error',reject);child.once('exit',resolve);});
    const result = JSON.parse(await fs.readFile(report,'utf8'));
    if (!result.passed) throw new Error(result.error);
  } catch (error) {
    throw new Error(`${error.message}\nVS Code integration artifacts: ${directory}`);
  } finally {clearTimeout(timer);await log.close();}
  await fs.rm(directory,{recursive:true,force:true});
  console.log('VS Code extension host: native config, shared definitions/references, dirty buffers, disconnect and reconnect passed.');
}

main().catch(error=>{console.error(error.message);process.exitCode=1;});
