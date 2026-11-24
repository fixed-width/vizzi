const {spawn} = require('node:child_process');

async function startWorkspace({binary,root,connectionFile,configFile,output,onExit=()=>{},timeout=15000,signal,createOrigin}) {
  if (signal?.aborted) throw new Error('Vizzi startup cancelled.');
  const origin = await createOrigin?.();
  if (signal?.aborted) {await origin?.dispose();throw new Error('Vizzi startup cancelled.');}
  const args = configFile ? ['--config',configFile] : [];
  args.push('--root',root,'--vscode-connection',connectionFile,'--workers','1');
  if (origin) args.push('--editor-connection',origin.file);
  output.appendLine(`Starting Vizzi for ${root}`);
  let child;
  try {child = spawn(binary,args,{cwd:root,stdio:['ignore','ignore','pipe'],windowsHide:true,
    env:{...process.env,VIZZI_ORIGIN_JAVA_ARCHIVE_SESSION:origin?.javaArchiveSession}});}
  catch(error) {await origin?.dispose();throw error;}
  let stopped = false;
  let exited = false;
  let diagnostic = '';
  const closed = new Promise(resolve=>child.once('close',()=>{exited=true;resolve();})).then(()=>origin?.dispose());
  async function dispose() {
    if (stopped) return closed;
    stopped = true;
    if (!exited) {
      child.kill('SIGINT');
      const timer = setTimeout(()=>child.kill('SIGKILL'),2000);
      try {await closed;} finally {clearTimeout(timer);}
    } else await closed;
  }
  try {
    const url = await new Promise((resolve,reject)=>{
      const timer = setTimeout(()=>reject(new Error(`Vizzi startup timed out. ${diagnostic}`)),timeout);
      const abort = ()=>finish(new Error('Vizzi startup cancelled.'));
      function finish(error,url) {
        clearTimeout(timer);
        signal?.removeEventListener('abort',abort);
        if (error) reject(error);else resolve(url);
      }
      child.once('error',error=>finish(error));
      child.once('close',(code,signal)=>{
        onExit();
        finish(new Error(`Vizzi exited (${signal || code}). ${diagnostic}`));
        if (!stopped) output.appendLine(`Vizzi stopped (${signal || code}).`);
      });
      child.stderr.on('data',data=>{
        output.append(data.toString());
        diagnostic = (diagnostic+data.toString()).slice(-8192);
        const match = diagnostic.match(/(?:^|\n)Vizzi: (http:\/\/127\.0\.0\.1:\d+)\r?\n/);
        if (match) finish(undefined,match[1]+'/');
      });
      signal?.addEventListener('abort',abort,{once:true});
      if (signal?.aborted) abort();
    });
    return {url,dispose};
  } catch (error) {await dispose();throw error;}
}

module.exports = {startWorkspace};
