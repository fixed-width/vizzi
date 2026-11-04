import { validateDocumentWindow } from "./windows.js";

export function validateReviewWindow(binding, body) {
  const {token,comparison,side}=binding.review,source=binding.source;
  if(body?.schema!==1 || body.token!==token || body.comparison!==comparison || body.side!==side)throw new Error("Mismatched review window");
  validateDocumentWindow(source,body.window);
  return body.window;
}

export function createReviewWindowReader({transport,onWindow=()=>{},onError=()=>{}}) {
  let binding,current,fetch;
  async function prepare(view,query,options={}) {
    if(!view.source)return null;
    const {token,side}=view.review;
    const parameters=new URLSearchParams(Object.entries({...query,file:view.source.id,side,version:view.source.version}).filter(([key,value])=>value!=null && !["snapshot","previous_snapshot"].includes(key)));
    const body=await transport(`/api/comparisons/${encodeURIComponent(token)}/window?${parameters}`,options);
    return validateReviewWindow(view,body);
  }
  return {
    prepare,
    activate(view) {fetch?.abort();fetch=undefined;binding=view;current={};},
    async request(query,intent) {
      const view=binding,token=current={};fetch?.abort();const abort=fetch=new AbortController();
      try {const window=await prepare(view,query,{signal:abort.signal});if(token!==current)return false;onWindow(window,query,intent);return true;}
      catch(error){if(token===current)onError(error);return false;}
      finally {if(token===current)fetch=undefined;}
    },
  };
}
