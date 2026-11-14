const opaque=value=>typeof value==='string' && /^[0-9a-f]{64}$/.test(value);
const aborted=()=>new DOMException('Editor handoff superseded or cancelled','AbortError');
function pause(milliseconds,signal) {
  return new Promise((resolve,reject)=>{
    if(signal.aborted){reject(aborted());return;}
    const done=()=>{signal.removeEventListener('abort',cancel);resolve();};
    const timer=setTimeout(done,milliseconds);
    const cancel=()=>{clearTimeout(timer);signal.removeEventListener('abort',cancel);reject(aborted());};
    signal.addEventListener('abort',cancel,{once:true});
  });
}
function createHandoffClient({fetch=globalThis.fetch,pause:delay=pause}={}) {
  const scopes=new Map();let latest,sequence=0;
  async function request(session,path,body,signal) {
    const deadline=AbortSignal.timeout(5000);
    const response=await fetch(new URL('api/editor/handoff'+path,session.url).href,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body),signal:signal?AbortSignal.any([deadline,signal]):deadline});
    if(!response.ok){
      if(response.status===404)throw new Error('This Vizzi executable does not support captured-source handoff. Update the extension and executable together.');
      let message=await response.text();try{message=JSON.parse(message).message || message;}catch{}throw new Error(message.slice(0,512) || 'Vizzi handoff failed');
    }
    const result=await response.json();
    if(result?.schema!==1)throw new Error('Invalid native handoff response; update the extension and executable together');
    return result;
  }
  async function attach(session) {
    const key=JSON.stringify([session.root,session.url]);
    if(!scopes.has(key)) {
      const pending=request(session,'/window',{root:session.root}).then(result=>{if(!opaque(result.scope))throw new Error('Invalid native handoff scope');return result.scope;});
      scopes.set(key,pending);pending.catch(()=>{if(scopes.get(key)===pending)scopes.delete(key);});
    }
    return {key,scope:await scopes.get(key)};
  }
  function begin() {
    latest?.controller.abort();
    if(sequence===Number.MAX_SAFE_INTEGER)throw new Error('Restart the Vizzi extension to reset editor requests');
    const controller=new AbortController(),intent={sequence:++sequence,controller,signal:controller.signal,current:()=>latest===intent && !controller.signal.aborted};
    latest=intent;return intent;
  }
  async function resolve(session,source,target,intent,{check=async()=>{},signal:parentSignal}={}) {
    const signal=parentSignal?AbortSignal.any([intent.signal,parentSignal]):intent.signal;
    if(session.root!==source.root)throw new Error('The selected source workspace changed; retry from the editor');
    const verify=async(final=false)=>{if(signal.aborted || !intent.current())throw aborted();await check({final});if(signal.aborted || !intent.current())throw aborted();};
    let attached,completed=false;
    const cancel=async()=>{if(attached)await request(session,'/cancel',{scope:attached.scope,sequence:intent.sequence}).catch(()=>{});};
    const onAbort=()=>{void cancel();};
    signal.addEventListener('abort',onAbort,{once:true});
    try {
      await verify();attached=await attach(session);await verify();
      let action={kind:'start',path:source.path,text:source.text,target},reattached=false;
      for(;;) {
        const result=await request(session,'',{scope:attached.scope,sequence:intent.sequence,action},signal);
        if(signal.aborted || !intent.current())throw aborted();
        if(result.sequence!==intent.sequence)throw new Error('Invalid native handoff sequence');
        if(result.state==='ready') {
          const d=result.destination;
          if(d?.kind!==target.kind || !opaque(d.snapshot) || (target.kind==='call'?!opaque(d.call):!opaque(d.file)||!opaque(d.version)||!Number.isSafeInteger(d.line)||d.line<1||d.line>target.line))throw new Error('Invalid native handoff destination');
          await verify(true);completed=true;
          return target.kind==='call'?{snapshot:d.snapshot,call:d.call}:{snapshot:d.snapshot,file:d.file,version:d.version,line:d.line};
        }
        if(result.state==='pending') {
          if(!opaque(result.handle) || !Number.isInteger(result.retry_after_ms) || result.retry_after_ms<1 || result.retry_after_ms>1000)throw new Error('Invalid native handoff continuation');
          action={kind:'continue',handle:result.handle};await delay(result.retry_after_ms,signal);await verify();continue;
        }
        if(result.state==='superseded' || result.state==='cancelled')throw aborted();
        if(result.state==='refused' && result.code==='scope_expired' && action.kind==='start' && !reattached){scopes.delete(attached.key);attached=await attach(session);await verify();reattached=true;continue;}
        if(result.state==='expired')throw new Error('The editor handoff expired before matching source was ready. Save the file and retry.');
        if(result.state==='refused')throw Object.assign(new Error(result.message || 'Vizzi refused this editor handoff'),{code:result.code});
        throw new Error('Invalid native handoff state');
      }
    } finally {signal.removeEventListener('abort',onAbort);if(!completed)await cancel();}
  }
  return {begin,resolve,dispose(){latest?.controller.abort();scopes.clear();}};
}
module.exports={createHandoffClient};
