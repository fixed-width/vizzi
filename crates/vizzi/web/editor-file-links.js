const opaque=value=>typeof value==='string' && /^[0-9a-f]{64}$/.test(value);

export function installEditorFileLinks({window=globalThis.window,ready,open,failed}) {
  const pending=[];let processing=false;
  async function resume() {
    if(processing || !ready())return;
    processing=true;
    try{while(pending.length && ready())try{await open(pending.shift());}catch(error){failed(error);}}
    finally{processing=false;}
  }
  function receive(hash) {
    const params=new URLSearchParams(hash.replace(/^#/,''));
    if(!params.has('editor-file'))return;
    const request={snapshot:params.get('snapshot'),file:params.get('editor-file'),version:params.get('version'),line:Number(params.get('line'))};
    if(![request.snapshot,request.file,request.version].every(opaque) || !Number.isSafeInteger(request.line) || request.line<1){failed(new Error('Invalid editor file link'));return;}
    if(pending.length>=16){failed(new Error('Too many editor file requests; retry when the source browser is ready'));return;}
    pending.push(request);
    if(window.location.hash===hash)window.history.replaceState(window.history.state,'',window.location.pathname+window.location.search);
    void resume();
  }
  window.addEventListener('hashchange',event=>receive(new URL(event.newURL).hash));
  receive(window.location.hash);
  return {resume,pending:()=>pending.length>0};
}
