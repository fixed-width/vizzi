import {readiness} from "./startup.js";

function notification(change) {
  const opaque = value => typeof value === "string" && value.length > 0 && value.length <= 256;
  if (!change || change.schema !== 1) throw new Error("Unsupported notification schema");
  if (!opaque(change.stream)) throw new Error("Invalid notification stream");
  if (typeof change.revision !== "string" || !/^(0|[1-9]\d{0,19})$/.test(change.revision) || BigInt(change.revision) > 18446744073709551615n) throw new Error("Invalid notification revision");
  const s = change.status;
  if (!s || s.schema !== 1 || typeof s.name !== "string" || typeof s.observation !== "string" || !Number.isSafeInteger(s.generation) || s.generation < 0 || typeof s.pending !== "boolean" || !(s.snapshot === null || opaque(s.snapshot)) || !(s.error === null || typeof s.error === "string")) throw new Error("Invalid notification status");
  readiness(s);
  if (s.measurements !== null) {
    const m = s.measurements;
    if (!m || !["files", "bytes", "parsed", "reused", "cache_hits", "cache_warnings"].every(key => Number.isSafeInteger(m[key]) && m[key] >= 0) || !["capture_ms", "total_ms"].every(key => Number.isFinite(m[key]) && m[key] >= 0)) throw new Error("Invalid notification status measurements");
  }
  return BigInt(change.revision);
}

export function createController({ transport, initialQuery = {}, onView = () => {}, onStatus = () => {}, onError = () => {}, onResync = () => {}, acceptView = () => true, viewSchema = 2 }) {
  let query = { ...initialQuery }, viewRequest, completedViewRequest, liveSession, activeFetch;
  let acceptedQuery = {}, requestedOptions;
  let displayedSnapshot = null, latestStatus = null;
  function invalidate() {
    const dirty = viewRequest !== completedViewRequest;
    viewRequest = {};
    if (!dirty) completedViewRequest = viewRequest;
    activeFetch?.abort(); activeFetch = undefined;
  }
  function begin(patch, options) {
    if (options !== undefined || Object.keys(patch).length) requestedOptions = options;
    const requestOptions = requestedOptions ?? {};
    query = { ...query, ...patch };
    if (latestStatus && latestStatus.snapshot === null) return null;
    const request = viewRequest = {}, previous = activeFetch;
    const fetch = activeFetch = new AbortController(); previous?.abort();
    const signal=requestOptions.signal?AbortSignal.any([fetch.signal,requestOptions.signal]):fetch.signal;
    const target=latestStatus?.snapshot;
    const info={...requestOptions,query:{...query}};
    return {request,fetch,signal,target,info,requestOptions,committed:false};
  }
  function rollback(record) {
    if(record.request!==viewRequest || record.committed)return;
    query={...acceptedQuery};requestedOptions=undefined;completedViewRequest=record.request;
    if(activeFetch===record.fetch)activeFetch=undefined;
  }
  function followPublication(record) {
    if(record.request===viewRequest && latestStatus?.snapshot && latestStatus.snapshot!==record.target && latestStatus.snapshot!==displayedSnapshot && !activeFetch)void load();
  }
  async function read(record) {
    if(!record)throw new Error('No source observation is available');
    const {request,signal,target,info}=record;
    const parameters=new URLSearchParams(Object.entries({...info.query,snapshot:target}).filter(([,value])=>value!=null));
    try {
      const next=await transport(`/api/view?${parameters}`,{...info,signal});
      if(request!==viewRequest || signal.aborted)throw new DOMException('View request cancelled','AbortError');
      if(!next || next.schema!==viewSchema || typeof next.snapshot!=='string' || !next.selection || target&&next.snapshot!==target)throw new Error('Invalid or mismatched pinned view response');
      const current=()=>request===viewRequest && !signal.aborted && !record.committed && (!target || latestStatus?.snapshot===target);
      return {view:next,info,current,
        commit() {
          if(!current())return false;
          if(!acceptView(next,info)){record.rejected=true;rollback(record);return false;}
          const browse=next.browse?{directory:next.browse.directory,offset:next.browse.offset}:{};
          query={...info.query,...browse,file:next.selection.file,entity:next.selection.entity};
          acceptedQuery={...query};requestedOptions=undefined;displayedSnapshot=next.snapshot;
          record.committed=true;completedViewRequest=request;
          onView(next,{...info,query:{...query}});
          if(activeFetch===record.fetch)activeFetch=undefined;
          return true;
        },
        cancel(){if(!record.committed){rollback(record);record.fetch.abort();followPublication(record);}}
      };
    } catch(error) {
      if(request===viewRequest){if(record.requestOptions.rollbackOnFailure)rollback(record);if(activeFetch===record.fetch)activeFetch=undefined;}
      if(record.preparing)followPublication(record);
      throw error;
    }
  }
  async function prepare(patch={},options) {const record=begin(patch,options);if(record)record.preparing=true;return read(record);}
  async function load(patch={},options) {
    const record=begin(patch,options);if(!record)return false;
    try {
      const prepared=await read(record);
      if(record.request===viewRequest && record.target && latestStatus?.snapshot!==record.target)return load({},record.requestOptions);
      return prepared.commit();
    } catch(error) {
      if(record.request===viewRequest){onError(error);onResync(error);}
      return false;
    } finally {
      if(record.request===viewRequest && activeFetch===record.fetch)activeFetch=undefined;
      if(record.request===viewRequest && (record.committed || record.rejected) && latestStatus?.snapshot && latestStatus.snapshot!==displayedSnapshot)void load();
    }
  }
  function updates() {
    invalidate(); latestStatus = null;
    const token = liveSession = {};
    let stream, revision = -1n;
    return {
      receive(change) {
        if (token !== liveSession) return false;
        const nextRevision = notification(change);
        if (stream !== undefined && stream !== change.stream) throw new Error("Notification stream changed without reconnecting");
        if (nextRevision <= revision) return false;
        stream = change.stream; revision = nextRevision;
        latestStatus = change.status; onStatus(latestStatus,{stream:change.stream});
        if (!activeFetch && latestStatus.snapshot && (viewRequest !== completedViewRequest || latestStatus.snapshot !== displayedSnapshot)) void load();
        return true;
      },
      close(error) {
        if (liveSession !== token) return;
        liveSession = undefined; latestStatus = null; invalidate();
        if (error) onError(error);
      },
    };
  }
  return { load, prepare, updates };
}
