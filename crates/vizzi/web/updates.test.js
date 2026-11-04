import test from "node:test";
import assert from "node:assert/strict";
import { followChanges } from "./updates.js";

test("review uses an independent endpoint and event without consuming live workspace events",()=>{
  const handlers=new Map(),received=[];let address;
  const adapter=followChanges({url:"/api/comparisons/events",eventName:"comparison",session:()=>({receive:value=>{received.push(value);return true;},close(){}}),createSource:url=>{address=url;return {addEventListener:(name,handler)=>handlers.set(name,handler),close(){}};}});
  assert.equal(address,"/api/comparisons/events");assert.ok(!handlers.has("workspace"));handlers.get("comparison")({data:'{"schema":1}'});assert.deepEqual(received,[{schema:1}]);adapter.stop();
});

function fixture() {
  const sources = [], sessions = [], timers = [];
  const adapter = followChanges({
    session: () => { const messages = [], errors = []; const value = { messages, errors, receive: change => { messages.push(change); return true; }, close: error => errors.push(error) }; sessions.push(value); return value; },
    createSource: url => {
      assert.equal(url, "/api/events");
      const handlers = new Map();
      const source = { closed: false, addEventListener: (name, handler) => handlers.set(name, handler), close() { this.closed = true; }, emit: (name, event) => handlers.get(name)?.(event) };
      sources.push(source); return source;
    },
    schedule: (callback, delay) => { const timer = { callback, delay }; timers.push(timer); return timer; },
    cancel: timer => { timer.cancelled = true; },
  });
  return { adapter, sources, sessions, timers, reconnect() { timers.at(-1).callback(); } };
}
test("live delivery creates no timer while healthy and replaces failed connections", () => {
  const f = fixture(); assert.equal(f.sources.length, 1); assert.equal(f.timers.length, 0);
  f.sources[0].emit("workspace", { data: '{"schema":1}' });
  assert.deepEqual(f.sessions[0].messages, [{ schema: 1 }]);
  f.sources[0].emit("error", {}); assert.equal(f.sources[0].closed, true); assert.equal(f.timers.length, 1);
  assert.equal(f.sessions[0].errors[0].message, "Live updates disconnected");
  f.adapter.restart(); assert.equal(f.timers.length, 1);
  f.sources[0].emit("error", {}); assert.equal(f.timers.length, 1);
  f.reconnect(); assert.equal(f.sources.length, 2);
  f.sources[0].emit("workspace", { data: 'invalid late data' });
  f.sources[0].emit("workspace", { data: '{}' });
  assert.equal(f.sessions[0].messages.length, 1);
  f.sources[1].emit("workspace", { data: '{"schema":1,"revision":"2"}' });
  assert.equal(f.sessions[1].messages.length, 1);
  f.adapter.stop(); assert.equal(f.sources[1].closed, true);
});

test("the default browser factory opens the notification endpoint", () => {
  const saved = globalThis.EventSource; const sources = [];
  globalThis.EventSource = class {
    constructor(url) { this.url = url; sources.push(this); }
    addEventListener() {}
    close() { this.closed = true; }
  };
  try {
    const adapter = followChanges({ session: () => ({ close() {}, receive() { return true; } }) });
    assert.equal(sources.length, 1); assert.equal(sources[0].url, "/api/events");
    adapter.stop(); assert.equal(sources[0].closed, true);
  } finally { globalThis.EventSource = saved; }
});
test("invalid data and view resynchronisation use bounded retry, and stop cancels it", () => {
  const f = fixture();
  f.sources[0].emit("workspace", { data: "not JSON" });
  assert.equal(f.sessions[0].errors.length, 1); assert.ok(f.sessions[0].errors[0] instanceof Error);
  for (let i = 0; i < 12; i++) { f.reconnect(); f.sources.at(-1).emit("error", {}); }
  assert.ok(f.timers.every(timer => timer.delay >= 100 && timer.delay <= 4000));
  assert.equal(f.timers.at(-1).delay, 4000);
  f.reconnect(); f.sources.at(-1).emit("workspace", { data: "{}" });
  f.adapter.restart(); assert.equal(f.timers.at(-1).delay, 100);
  f.adapter.stop(); assert.equal(f.timers.at(-1).cancelled, true);
  const count = f.sources.length; f.reconnect(); assert.equal(f.sources.length, count);
});
test("consumer rejection closes the stream and schedules resynchronisation", () => {
  const f = fixture();
  f.sessions[0].receive = () => { throw new Error("unsupported schema"); };
  f.sources[0].emit("workspace", { data: "{}" });
  assert.equal(f.sources[0].closed, true); assert.equal(f.sessions[0].errors[0].message, "unsupported schema");
  assert.equal(f.timers.length, 1);
});

test("source-construction failures retry, and stopping a healthy source is terminal", () => {
  const failures = [], timers = [];
  const adapter = followChanges({ session: () => ({ close: error => failures.push(error) }), createSource: () => { throw new Error("unavailable"); }, schedule: callback => { timers.push(callback); return 1; }, cancel: () => {} });
  assert.equal(failures[0].message, "unavailable"); assert.equal(timers.length, 1);
  adapter.stop(); timers[0](); assert.equal(failures.length, 1);
  const f = fixture(); f.adapter.stop(); f.adapter.stop(); f.adapter.restart();
  f.sources[0].emit("workspace", { data: "invalid" });
  assert.equal(f.sources[0].closed, true); assert.equal(f.timers.length, 0); assert.equal(f.sessions[0].messages.length, 0);
});
