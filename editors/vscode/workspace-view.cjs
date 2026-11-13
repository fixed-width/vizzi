const crypto = require('node:crypto');

function createWorkspaceView(vscode) {
  let current;
  function dispose() {
    const previous = current;
    current = undefined;
    previous?.panel.dispose();
  }
  function show(url,name) {
    if (current?.url === url) {current.panel.reveal();return;}
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
    current = {url,panel};
    panel.onDidDispose(()=>{if (current?.panel === panel) current=undefined;});
    const nonce = crypto.randomBytes(16).toString('hex');
    try {
      panel.webview.html = `<!DOCTYPE html>
<html lang="en"><head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; frame-src ${address.origin}; style-src 'nonce-${nonce}'; base-uri 'none';">
<title>Vizzi</title>
<style nonce="${nonce}">html,body,iframe{width:100%;height:100%;margin:0;padding:0;border:0;display:block;overflow:hidden}</style>
</head><body><iframe title="Vizzi visual workspace" src="${address.origin}/" sandbox="allow-scripts allow-same-origin allow-forms allow-downloads"></iframe></body></html>`;
    } catch (error) {dispose();throw error;}
  }
  return {show,dispose};
}

module.exports = {createWorkspaceView};
