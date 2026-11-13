const opaque=value=>typeof value==='string' && /^[0-9a-f]{64}$/.test(value);

export function installEditorCallLinks({window=globalThis.window,ready,open,failed,queued=()=>{}}) {
 const pending=[];
 let processing=false;
 async function resume() {
  if(processing||!ready())return;
  processing=true;
  try{while(pending.length&&ready()){
   const request=pending.shift();
   try{await open(request);}catch(error){failed(error);}
  }}finally{processing=false;}
 }
 function receive(hash) {
  const params=new URLSearchParams(hash.replace(/^#/,''));
  if(!params.has('editor-call'))return;
  const snapshot=params.get('snapshot'),call=params.get('editor-call');
  if(!opaque(snapshot)||!opaque(call)){failed(new Error('Invalid editor call link'));return;}
  if(pending.length>=16){failed(new Error('Too many editor call requests; retry after the workspace is ready'));return;}
  pending.push({snapshot,call});
  if(window.location.hash===hash)window.history.replaceState(null,'',window.location.pathname+window.location.search);
  queued();void resume();
 }
 window.addEventListener('hashchange',event=>receive(new URL(event.newURL).hash));
 receive(window.location.hash);
 return {resume};
}
