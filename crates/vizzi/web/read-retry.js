function pause(ms,signal){return new Promise((resolve,reject)=>{const abort=()=>{clearTimeout(timer);reject(signal.reason??new DOMException('Aborted','AbortError'));};const timer=setTimeout(()=>{signal?.removeEventListener('abort',abort);resolve();},ms);signal?.addEventListener('abort',abort,{once:true});if(signal?.aborted)abort();});}
export async function fetchRead(path,options={},fetcher=globalThis.fetch,wait=pause){
 const retryable=['/api/window','/api/dependency/window','/api/function-history/window','/api/function-history/confirm','/api/function-comparisons/window','/api/captured-functions','/api/structure','/api/visual/refresh','/api/references'].includes(path.split('?')[0]);
 for(let attempt=0;;attempt++){options.signal?.throwIfAborted();const response=await fetcher(path,options);if(!retryable||![429,503].includes(response.status)||attempt>=5)return response;await response.body?.cancel();await wait(50*2**attempt,options.signal);}
}
