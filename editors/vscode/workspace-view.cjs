const crypto = require('node:crypto');

function createWorkspaceView(vscode) {
  let current;
  function dispose() {
    const previous = current;
    current = undefined;
    previous?.ready(false);
    previous?.panel.dispose();
  }
  async function show(url,name,{review=false,sourceBrowser=false}={}) {
    if (current?.url === url) {current.panel.reveal();if(review)await send('vizzi.review');return;}
    const address = new URL(url);
    if (address.protocol !== 'http:' || address.hostname !== '127.0.0.1') {
      throw new Error('The Vizzi editor tab requires a local workspace address');
    }
    dispose();
    const port = Number(address.port || 80);
    const panel = vscode.window.createWebviewPanel('vizzi.workspace',`Vizzi — ${name}`,vscode.ViewColumn.Active,{
      enableScripts:true,retainContextWhenHidden:true,localResourceRoots:[],
      portMapping:[{webviewPort:port,extensionHostPort:port}],
    });
    const nonce = crypto.randomBytes(16).toString('hex');
    let ready;
    const loaded=new Promise(resolve=>{ready=resolve;});
    const view={url,panel,nonce,ready,loaded};
    current=view;
    const messages=panel.webview.onDidReceiveMessage(message=>{if (message?.type==='vizzi.ready') ready(true);});
    panel.onDidDispose(()=>{ready(false);messages.dispose();if (current===view) current=undefined;});
    try {
      panel.webview.html = `<!DOCTYPE html>
<html lang="en"><head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; frame-src ${address.origin}; style-src 'nonce-${nonce}'; script-src 'nonce-${nonce}'; base-uri 'none';">
<title>Vizzi</title>
<style nonce="${nonce}">html,body,iframe{width:100%;height:100%;margin:0;padding:0;border:0;display:block;overflow:hidden}</style>
</head><body><iframe id="workspace" title="Vizzi visual workspace" src="${address.origin}/${review?'review':sourceBrowser?'explore':''}" sandbox="allow-scripts allow-same-origin allow-forms allow-downloads"></iframe>
<script nonce="${nonce}">
const api=acquireVsCodeApi();
window.addEventListener('message',event=>{
  const data=event.data;if(data?.nonce!=='${nonce}')return;
  const location=document.getElementById('workspace').contentWindow.location;
  if(data.type==='vizzi.review'){location.href='${address.origin}/review';return;}
  if(data.type==='vizzi.browse'){
    if(![data.snapshot,data.file,data.version].every(value=>typeof value==='string'&&/^[0-9a-f]{64}$/.test(value))||!Number.isSafeInteger(data.line)||data.line<1)return;
    location.href='${address.origin}/explore#editor-file='+data.file+'&snapshot='+data.snapshot+'&version='+data.version+'&line='+data.line;return;
  }
  if(data.type!=='vizzi.explore'||!/^[0-9a-f]{64}$/.test(data.snapshot)||!/^[0-9a-f]{64}$/.test(data.call))return;
  location.href='${address.origin}/#editor-call='+data.call+'&snapshot='+data.snapshot;
});
api.postMessage({type:'vizzi.ready'});
</script></body></html>`;
    } catch (error) {dispose();throw error;}
  }
  async function send(type,handoff={}) {
    const view=current;
    if (!view) throw new Error('Open the Vizzi workspace before navigating');
    let timer;
    try {
      const ready=await Promise.race([view.loaded,new Promise(resolve=>{timer=setTimeout(()=>resolve(false),10000);})]);
      if (!ready || current!==view) throw new Error('The Vizzi tab closed or did not become ready; retry the command');
      if (!await view.panel.webview.postMessage({...handoff,type,nonce:view.nonce})) throw new Error('The Vizzi tab is unavailable; retry the command');
    } finally {clearTimeout(timer);}
  }
  return {show,dispose,explore:handoff=>send('vizzi.explore',handoff),browse:handoff=>send('vizzi.browse',handoff)};
}

module.exports = {createWorkspaceView};
