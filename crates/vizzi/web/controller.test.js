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
  requests[1].resolve(view("second")); await second;
  requests[0].resolve(view("first")); await first;
  assert.deepEqual(views.map(value => value.selection.file), ["second"]);
  const stale = controller.load({ file: "old" }); const latest = controller.load({ file: "new" });
  requests[3].resolve(view("new")); await latest;
  requests[2].reject(new Error("stale failure")); await stale;
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
  await controller.load(); fail = true; await controller.load({ file: "b" });
  assert.equal(views.length, 1); assert.equal(errors.length, 1);
  fail = false; await controller.load(); assert.equal(views.length, 2);
});
