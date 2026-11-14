const crypto = require('node:crypto');
const path = require('node:path');
let shared;
function createWorkspaceView(vscode) {
  let current;
  function dispose() {
    const previous = current;
    current = undefined;
    previous?.client.dispose();
    previous?.panel.dispose();
  }
  async function present(session, action = {
    kind: 'reveal'
  }, options = {}) {
    if (options.signal?.aborted) return {
      state: 'cancelled'
    };
    const {
      createDeliveryClient
    } = await (shared ??= import('./workspace-delivery.mjs'));
    if (options.signal?.aborted) return {
      state: 'cancelled'
    };
    if (current?.url === session.url) current.panel.reveal();else {
      const address = new URL(session.url);
      if (address.protocol !== 'http:' || address.hostname !== '127.0.0.1' || address.username || address.password) throw new Error('The Vizzi editor tab requires a local workspace address');
      dispose();
      const port = Number(address.port || 80);
      const panel = vscode.window.createWebviewPanel('vizzi.workspace', `Vizzi — ${path.basename(session.root)}`, vscode.ViewColumn.Active, {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: [],
        portMapping: [{
          webviewPort: port,
          extensionHostPort: port
        }]
      });
      const nonce = crypto.randomBytes(16).toString('hex');
      const client = createDeliveryClient({
        send: frame => panel.webview.postMessage({
          type: 'vizzi.delivery',
          nonce,
          frame
        })
      });
      const view = {
        url: session.url,
        panel,
        client
      };
      current = view;
      const messages = panel.webview.onDidReceiveMessage(message => {
        if (current === view && message?.type === 'vizzi.delivery' && message.nonce === nonce) client.receive(message.frame);
      });
      panel.onDidDispose(() => {
        client.dispose();
        messages.dispose();
        if (current === view) current = undefined;
      });
      try {
        panel.webview.html = `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; frame-src ${address.origin}; style-src 'nonce-${nonce}'; script-src 'nonce-${nonce}'; base-uri 'none';"><title>Vizzi</title>
<style nonce="${nonce}">html,body,iframe{width:100%;height:100%;margin:0;padding:0;border:0;display:block;overflow:hidden}</style></head><body>
<iframe id="workspace" title="Vizzi visual workspace" src="${address.origin}/present" sandbox="allow-scripts allow-same-origin allow-forms allow-downloads"></iframe>
<script nonce="${nonce}">
const api=acquireVsCodeApi();const nonce=${JSON.stringify(nonce)};const origin=${JSON.stringify(address.origin)};
const iframe=document.getElementById('workspace');let connected,candidate;
const probe=()=>iframe.contentWindow.postMessage({protocol:'vizzi-delivery-connect',version:1,type:'probe'},origin);
iframe.addEventListener('load',probe);
window.addEventListener('message',event=>{
 const data=event.data;
 if(event.source===iframe.contentWindow&&event.origin===origin&&data?.protocol==='vizzi-delivery-connect'&&data.version===1&&data.type==='hello'){
  if(!/^[0-9a-f]{32}$/.test(data.document)||connected?.document===data.document||candidate?.document===data.document)return;
  candidate?.port.close();const channels=new MessageChannel(),challenge=crypto.randomUUID(),record={document:data.document,challenge,port:channels.port1};candidate=record;
  channels.port1.onmessage=message=>{
   const frame=message.data;
   if(candidate===record&&frame?.protocol==='vizzi-delivery-connect'&&frame.type==='attached'&&frame.version===1&&frame.document===record.document&&frame.challenge===challenge){connected?.port.close();connected=record;candidate=undefined;return;}
   if(connected===record&&frame?.document===record.document)api.postMessage({type:'vizzi.delivery',nonce,frame});
  };
  channels.port1.start();iframe.contentWindow.postMessage({protocol:'vizzi-delivery-connect',version:1,type:'attach',document:record.document,challenge},origin,[channels.port2]);return;
 }
 if(data?.type==='vizzi.delivery'&&data.nonce===nonce)connected?.port.postMessage(data.frame);
});
probe();
</script></body></html>`;
      } catch (error) {
        dispose();
        throw error;
      }
    }
    return current.client.deliver(action, options);
  }
  return {
    present,
    dispose
  };
}
module.exports = {
  createWorkspaceView
};
