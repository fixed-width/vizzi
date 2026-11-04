export function readiness(status) {
  const value=status.snapshot!==null?"ready":status.pending?"initializing":status.error!==null?"failed":"not_started";
  if(status.readiness!==undefined && status.readiness!==value)throw new Error("Invalid workspace readiness");
  return value;
}

export function createStartup({transport,onState=()=>{},onError=()=>{}}) {
  let latest,stream,attempted=false,request;
  function emit() {
    if(!latest)return;
    const value=readiness(latest),initial=value!=="ready";
    onState({initial,busy:Boolean(request),label:value==="initializing"?"Indexing workspace…":value==="failed"?"Initial indexing failed":value==="not_started"?"Workspace index not started":null,
      action:value==="not_started"?"Start indexing":value==="failed"?"Retry indexing":null});
  }
  function activate() {
    if(request)return request.promise;
    attempted=true;const current=request={abort:new AbortController()};emit();
    current.promise=(async()=>{
      try {await transport("/api/workspace/start",{method:"POST",headers:{"Content-Type":"application/json"},body:"{}",signal:current.abort.signal});return current===request;}
      catch(error){if(current===request)onError(error);return false;}
      finally {if(current===request){request=undefined;emit();}}
    })();
    return current.promise;
  }
  return {
    activate,
    receive(status,identity) {
      readiness(status);
      if(identity!==stream){stream=identity;attempted=false;request?.abort.abort();request=undefined;}
      latest=status;emit();
      if(readiness(status)==="not_started" && !attempted)void activate();
    },
  };
}
