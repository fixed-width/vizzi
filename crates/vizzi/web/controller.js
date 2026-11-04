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

export function createController({ transport, onView = () => {}, onStatus = () => {}, onError = () => {}, onResync = () => {}, acceptView = () => true, viewSchema = 2 }) {
  let query = {}, viewRequest, completedViewRequest, liveSession, activeFetch;
  let acceptedQuery = {}, requestedOptions;
  let displayedSnapshot = null, latestStatus = null;
  function invalidate() {
    const dirty = viewRequest !== completedViewRequest;
    viewRequest = {};
    if (!dirty) completedViewRequest = viewRequest;
    activeFetch?.abort(); activeFetch = undefined;
  }
  async function load(patch = {}, options) {
    if (options !== undefined || Object.keys(patch).length) requestedOptions = options;
    const requestOptions = requestedOptions ?? {};
    query = { ...query, ...patch };
    if (latestStatus && latestStatus.snapshot === null) return false;
    const request = viewRequest = {}, previous = activeFetch;
    const fetch = activeFetch = new AbortController(); previous?.abort();
    const target = latestStatus?.snapshot;
    const parameters = new URLSearchParams(Object.entries({ ...query, snapshot: target }).filter(([, value]) => value != null));
    let accepted = false, rejected = false;
    try {
      const info = { ...requestOptions, query: { ...query } };
      const next = await transport(`/api/view?${parameters}`, { signal: fetch.signal, ...info });
      if (request !== viewRequest) return false;
      if (!next || next.schema !== viewSchema || typeof next.snapshot !== "string" || !next.selection || (target && next.snapshot !== target)) throw new Error("Invalid or mismatched pinned view response");
      if (target && latestStatus.snapshot !== target) return load({}, requestOptions);
      if (!acceptView(next, info)) {
        rejected = true;
        query = { ...acceptedQuery }; requestedOptions = undefined; completedViewRequest = request;
        return false;
      }
      query = { ...query, file: next.selection.file, entity: next.selection.entity };
      acceptedQuery = { ...query }; requestedOptions = undefined;
      displayedSnapshot = next.snapshot; onView(next, { ...info, query: { ...query } }); completedViewRequest = request;
      accepted = true; return true;
    } catch (error) {
      if (request === viewRequest) {
        if (requestOptions.rollbackOnFailure) { query = { ...acceptedQuery }; requestedOptions = undefined; completedViewRequest = request; }
        onError(error); onResync(error);
      }
      return false;
    } finally {
      if (request === viewRequest) {
        activeFetch = undefined;
        if ((accepted || rejected) && latestStatus?.snapshot && latestStatus.snapshot !== displayedSnapshot) void load();
      }
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
  return { load, updates };
}
