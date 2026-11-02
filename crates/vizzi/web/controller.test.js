import test from "node:test";
import assert from "node:assert/strict";
import { createController } from "./controller.js";
function deferred() { let resolve, reject; const promise = new Promise((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; }
const flush = () => new Promise(setImmediate);
const view = (file = "a", snapshot = "s1") => ({ schema: 2, snapshot, selection: { file, entity: null } });
const state = (snapshot = "s1", patch = {}) => ({ schema: 1, name: "test", generation: 1, snapshot, pending: false, measurements: null, error: null, observation: "native", ...patch });
const change = (revision = "1", snapshot = "s1", patch = {}) => ({ schema: 1, stream: "host", revision, status: state(snapshot), ...patch });

test("user selections cancel superseded fetches and reject late results/errors", async () => {
  const requests = [], views = [], errors = [];
  const c = createController({ transport: (uri, options) => { const p = deferred(); requests.push({ ...p, uri, options }); return p.promise; }, onView: v => views.push(v), onError: e => errors.push(e) });
  const a = c.load({ file: "old" }), b = c.load({ file: "new", filter: "a&b", entity: null });
  assert.equal(requests[0].options.signal.aborted, true);
  const query = new URL(requests[1].uri, "http://local").searchParams;
  assert.equal(query.get("filter"), "a&b"); assert.equal(query.has("entity"), false);
  requests[1].resolve(view("new")); assert.equal(await b, true);
  requests[0].resolve(view("old")); assert.equal(await a, false);
  const stale = c.load({ file: "stale" }), current = c.load({ file: "current" });
  requests[3].resolve(view("current")); await current;
  requests[2].reject(new Error("old failure")); assert.equal(await stale, false);
  assert.deepEqual(views.map(v => v.selection.file), ["new", "current"]); assert.deepEqual(errors, []);
});
test("notifications pin queries and order decimal revisions without numeric precision loss", async () => {
  const requests = [], statuses = [], views = [];
  const c = createController({ transport: async uri => { requests.push(uri); return view(); }, onStatus: s => statuses.push(s), onView: v => views.push(v) });
  const stream = c.updates();
  assert.equal(stream.receive(change("9007199254740993")), true); await flush();
  assert.equal(new URL(requests[0], "http://local").searchParams.get("snapshot"), "s1");
  assert.equal(stream.receive(change("9007199254740992")), false); assert.equal(stream.receive(change("9007199254740993")), false);
  assert.equal(stream.receive(change("18446744073709551615", "s1", { status: state("s1", { pending: true }) })), true); await flush();
  assert.equal(requests.length, 1); assert.equal(views.length, 1); assert.equal(statuses.length, 2); assert.equal(statuses[1].pending, true);
});
test("snapshots coalesce during a fetch without applying an obsolete selection fallback", async () => {
  const pending = [], views = []; let queue = false;
  const c = createController({ transport: uri => { if (!queue) return Promise.resolve(view()); const p = deferred(); pending.push({ uri, ...p }); return p.promise; }, onView: v => views.push(v) });
  const stream = c.updates(); stream.receive(change()); await flush(); queue = true;
  stream.receive(change("2", "s2")); stream.receive(change("3", "s3")); stream.receive(change("4", "s4"));
  assert.equal(pending.length, 1); pending[0].resolve(view("wrong-fallback", "s2")); await flush();
  assert.equal(views.length, 1); assert.equal(pending.length, 2);
  const query = new URL(pending[1].uri, "http://local").searchParams;
  assert.equal(query.get("snapshot"), "s4"); assert.equal(query.get("file"), "a");
  pending[1].resolve(view("a", "s4")); await flush(); assert.equal(views.at(-1).snapshot, "s4");
});
test("reconnect rejects old callbacks and source, but accepts a restarted stream at revision zero", async () => {
  const requests = [], views = [], errors = [];
  const c = createController({ transport: (uri, options) => { const p = deferred(); requests.push({ ...p, options, uri }); return p.promise; }, onView: v => views.push(v), onError: e => errors.push(e.message) });
  const old = c.updates(); old.receive(change("50")); old.close(new Error("disconnected"));
  assert.equal(requests[0].options.signal.aborted, true);
  const latest = c.updates(); latest.receive(change("0", "s2", { stream: "new-host" }));
  assert.equal(old.receive(change("999", "obsolete")), false);
  requests[0].resolve(view("obsolete")); requests[1].resolve(view("new", "s2")); await flush(); old.close(new Error("late"));
  assert.deepEqual(views.map(v => v.snapshot), ["s2"]); assert.deepEqual(errors, ["disconnected"]);
  assert.throws(() => latest.receive(change("1", "s2", { stream: "unexpected" })), /stream/i);
});
test("failed selection resynchronises and retries on an unchanged snapshot", async () => {
  let fail = false; const views = [], errors = [], resync = [];
  const c = createController({ transport: async uri => { if (fail) throw new Error("offline"); return view(new URL(uri, "http://local").searchParams.get("file") ?? "a"); }, onView: v => views.push(v.selection.file), onError: e => errors.push(e.message), onResync: () => resync.push(true) });
  const first = c.updates(); first.receive(change()); await flush();
  fail = true; assert.equal(await c.load({ file: "b" }), false); assert.deepEqual(resync, [true]); assert.deepEqual(errors, ["offline"]);
  first.close(); fail = false; const second = c.updates(); second.receive(change()); await flush();
  assert.deepEqual(views, ["a", "b"]); second.close(); second.close(new Error("already closed")); assert.equal(second.receive(change("2")), false);
});
test("invalid envelopes do not change accepted status or selection", async () => {
  const statuses = [], c = createController({ transport: async () => view(), onStatus: s => statuses.push(s) }), stream = c.updates();
  for (const invalid of [null, {}, change(1), change("-1"), change("01"), change("1.0"), change("18446744073709551616"), change("1", "s1", { schema: 2 }), change("1", "s1", { stream: "" }), change("1", "s1", { status: state("s1", { schema: 2 }) }), change("1", "s1", { status: state("s1", { pending: "yes" }) }), change("1", "s1", { status: state("s1", { measurements: {} }) })]) assert.throws(() => stream.receive(invalid), /notification|revision|status|schema|stream/i);
  assert.equal(statuses.length, 0); assert.equal(stream.receive(change("0")), true); await flush(); assert.equal(statuses.length, 1);
});
test("initialisation waits for source and a wrong pinned response triggers resynchronisation", async () => {
  const requests = [], errors = [], views = [], resync = [];
  const c = createController({ transport: async uri => { requests.push(uri); return view("wrong", "wrong-version"); }, onError: e => errors.push(e.message), onView: v => views.push(v), onResync: () => resync.push(true) });
  const stream = c.updates(); stream.receive(change("0", null)); assert.equal(await c.load({ file: "a" }), false); assert.equal(requests.length, 0);
  stream.receive(change()); await flush(); assert.equal(requests.length, 1); assert.equal(views.length, 0); assert.equal(errors.length, 1); assert.equal(resync.length, 1);
});
test("status arriving inside a view callback is followed by the newest pinned view", async () => {
  const requests = [], views = []; let stream;
  const c = createController({ transport: async uri => { requests.push(uri); return view("a", new URL(uri, "http://local").searchParams.get("snapshot")); }, onView: v => { views.push(v.snapshot); if (v.snapshot === "s1") stream.receive(change("2", "s2")); } });
  stream = c.updates(); stream.receive(change()); await flush(); assert.deepEqual(views, ["s1", "s2"]); assert.equal(requests.length, 2);
});

test("status validation accepts zeroes and exact bounds and rejects malformed fields", async () => {
  const c = createController({ transport: async () => view() });
  const measurements = { files: 0, bytes: 0, parsed: 0, reused: 0, cache_hits: 0, cache_warnings: 0, capture_ms: 0, total_ms: 0 };
  const valid = state(null, { generation: 0, name: "", observation: "", measurements });
  const s = c.updates(); assert.equal(s.receive(change("0", null, { stream: "x".repeat(256), status: valid })), true);
  for (const field of ["files", "bytes", "parsed", "reused", "cache_hits", "cache_warnings", "capture_ms", "total_ms"]) {
    for (const value of [-1, NaN, Infinity, "0", undefined]) {
      assert.throws(() => c.updates().receive(change("0", null, { status: { ...valid, measurements: { ...measurements, [field]: value } } })), /status/);
    }
  }
  for (const patch of [{ generation: -1 }, { generation: .5 }, { generation: Number.MAX_SAFE_INTEGER + 1 }, { name: null }, { observation: null }, { error: 1 }, { snapshot: "" }, { snapshot: "x".repeat(257) }, { measurements: undefined }]) {
    assert.throws(() => c.updates().receive(change("0", null, { status: { ...valid, ...patch } })), /status/);
  }
  for (const stream of [null, 0, {}, "x".repeat(257)]) assert.throws(() => c.updates().receive(change("0", null, { stream })), /stream/);
  assert.equal(c.updates().receive(change("0", null, { status: { ...valid, error: "failed" } })), true);
});

test("a clean reconnect with unchanged content does not reload the captured view", async () => {
  let calls = 0;
  const c = createController({ transport: async () => { calls++; return view(); } });
  const first = c.updates(); first.receive(change()); await flush();
  const second = c.updates(); second.receive(change("0", "s1", { stream: "replacement" })); await flush(); assert.equal(calls, 1);
  second.close(); c.updates().receive(change("0", "s1", { stream: "another" })); await flush(); assert.equal(calls, 1);
});
test("replacing a subscription directly cancels and supersedes its pending view", async () => {
  const requests = [], views = [], c = createController({ transport: (uri, options) => { const p = deferred(); requests.push({ ...p, options }); return p.promise; }, onView: v => views.push(v) });
  const first = c.updates(); first.receive(change());
  c.updates().receive(change("0", "s2", { stream: "replacement" }));
  assert.equal(requests.length, 2); assert.equal(requests[0].options.signal.aborted, true);
  requests[1].resolve(view("b", "s2")); requests[0].resolve(view()); await flush();
  assert.deepEqual(views.map(v => v.snapshot), ["s2"]);
});
test("a stale completion cannot clear a newer pending fetch", async () => {
  const requests = [], c = createController({ transport: uri => { const p = deferred(); requests.push({ ...p, uri }); return p.promise; } });
  const s = c.updates(); s.receive(change()); const selected = c.load({ file: "b" });
  requests[0].resolve(view()); await flush(); s.receive(change("2", "s2"));
  assert.equal(requests.length, 2);
  requests[1].resolve(view("b")); await flush(); assert.equal(requests.length, 3);
  requests[2].resolve(view("b", "s2")); assert.equal(await selected, true);
});
test("malformed view bodies produce a useful error without reconciling selection", async () => {
  for (const body of [null, {}, { ...view(), schema: 1 }, { ...view(), snapshot: 42 }, { ...view(), selection: null }]) {
    const errors = [], views = [], c = createController({ transport: async () => body, onError: e => errors.push(e.message), onView: v => views.push(v) });
    assert.equal(await c.load(), false); assert.equal(views.length, 0); assert.match(errors[0], /view/i);
  }
});

 test("summary-view schema is explicit and does not accept a bulk response", async () => {
  let schema=3;const errors=[];
  const controller=createController({viewSchema:3,transport:async()=>({schema,snapshot:"one",selection:{file:null,entity:null},source:null}),onError:error=>errors.push(error)});
  assert.equal(await controller.load(),true);schema=2;assert.equal(await controller.load(),false);assert.equal(errors.length,1);
});
