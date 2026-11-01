import test from "node:test";
import assert from "node:assert/strict";
import { createController } from "./controller.js";

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function view(file, snapshot = "s1") { return { snapshot, selection: { file, entity: null } }; }

test("late views and errors cannot replace a newer selection", async () => {
  const requests = [], views = [], errors = [];
  const controller = createController({ transport: uri => { const pending = deferred(); requests.push({ uri, ...pending }); return pending.promise; }, onView: value => views.push(value), onError: error => errors.push(error) });
  const first = controller.load({ file: "first" });
  const second = controller.load({ file: "second", filter: "a&b" });
  assert.equal(new URL(requests[1].uri, "http://local").searchParams.get("filter"), "a&b");
  requests[1].resolve(view("second")); assert.equal(await second, true);
  requests[0].resolve(view("first")); assert.equal(await first, false);
  assert.deepEqual(views.map(value => value.selection.file), ["second"]);
  const stale = controller.load({ file: "old" }); const latest = controller.load({ file: "new" });
  requests[3].resolve(view("new")); await latest;
  requests[2].reject(new Error("stale failure")); assert.equal(await stale, false);
  assert.deepEqual(errors, []);
});

test("publication reloads preserve reconciled selection and coalesce while a view is pending", async () => {
  const requests = []; let snapshot = "s1"; let pending;
  const controller = createController({ transport: async uri => {
    requests.push(uri);
    if (uri === "/api/status") return { snapshot };
    if (pending) return pending.promise;
    return view("surviving-file", snapshot);
  }});
  await controller.load({ file: "removed-file" });
  await controller.poll(); assert.equal(requests.filter(uri => uri.startsWith("/api/view")).length, 1);
  snapshot = "s2"; await controller.poll();
  assert.equal(new URL(requests.at(-1), "http://local").searchParams.get("file"), "surviving-file");
  pending = deferred(); snapshot = "s3";
  const loading = controller.poll(); await Promise.resolve(); await Promise.resolve();
  const before = requests.filter(uri => uri.startsWith("/api/view")).length;
  await controller.poll();
  assert.equal(requests.filter(uri => uri.startsWith("/api/view")).length, before);
  pending.resolve(view("surviving-file", "s3")); await loading;
});

test("failed requests preserve the displayed view and can retry", async () => {
  const views = [], errors = []; let fail = false;
  const controller = createController({ transport: async () => { if (fail) throw new Error("offline"); return view("a"); }, onView: value => views.push(value), onError: error => errors.push(error) });
  await controller.load(); fail = true; assert.equal(await controller.load({ file: "b" }), false);
  assert.equal(views.length, 1); assert.equal(errors.length, 1);
  fail = false; await controller.load(); assert.equal(views.length, 2);
});

test("first status poll opens a view and optional query values retain their types", async () => {
  const requests = [], views = [], statuses = [];
  const controller = createController({ transport: async uri => {
    requests.push(uri);
    return uri === "/api/status" ? { snapshot: "s1" } : view("a");
  }, onView: value => views.push(value), onStatus: value => statuses.push(value) });
  await controller.poll();
  assert.equal(views.length, 1); assert.equal(statuses[0].snapshot, "s1");
  assert.equal(new URL(requests[1], "http://local").searchParams.get("filter") ?? "", "");
  await controller.load({ file: "a?b&c", entity: null });
  const query = new URL(requests.at(-1), "http://local").searchParams;
  assert.equal(query.get("file"), "a?b&c"); assert.equal(query.has("entity"), false);
});

test("a completed stale request cannot clear a newer pending request", async () => {
  const requests = [];
  const controller = createController({ transport: uri => {
    if (uri === "/api/status") return Promise.resolve({ snapshot: "s2" });
    const request = deferred(); requests.push(request);
    if (requests.length > 2) request.resolve(view("unexpected", "s2"));
    return request.promise;
  }});
  const first = controller.load({ file: "a" }); const second = controller.load({ file: "b" });
  requests[0].resolve(view("a")); await first;
  await controller.poll();
  assert.equal(requests.length, 2);
  requests[1].resolve(view("b", "s2")); await second;
});

test("late status successes and failures cannot replace newer status", async () => {
  const requests = [], statuses = [], errors = [];
  const controller = createController({ transport: () => { const request = deferred(); requests.push(request); return request.promise; }, onStatus: value => statuses.push(value), onError: error => errors.push(error.message) });
  const old = controller.poll(); const current = controller.poll();
  requests[1].resolve({ snapshot: null, name: "current" }); await current;
  requests[0].resolve({ snapshot: null, name: "obsolete" }); await old;
  assert.deepEqual(statuses.map(status => status.name), ["current"]);
  const oldError = controller.poll(); const newError = controller.poll();
  requests[3].reject(new Error("latest failure")); await newError;
  requests[2].reject(new Error("obsolete failure")); await oldError;
  assert.deepEqual(errors, ["latest failure"]);
});

test("poll retries a failed selection even when the published snapshot is unchanged", async () => {
  let fail = false;
  const views = [];
  const controller = createController({
    transport: async uri => {
      if (uri === "/api/status") return { snapshot: "s1" };
      if (fail) throw new Error("offline");
      return view(new URL(uri, "http://local").searchParams.get("file"));
    },
    onView: value => views.push(value.selection.file),
  });
  await controller.load({ file: "a" });
  fail = true; await controller.load({ file: "b" });
  fail = false; await controller.poll();
  assert.deepEqual(views, ["a", "b"]);
});
