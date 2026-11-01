import test from "node:test";
import assert from "node:assert/strict";
import { summary } from "./statistics.mjs";

test("empty timing sets are absent, not zero-latency results", () => {
  assert.deepEqual(summary([]), { count: 0, p50: null, p95: null, p99: null, max: null, over33ms: 0, over50ms: 0 });
});
test("nearest-rank percentiles preserve input and count strict frame thresholds", () => {
  const values = [51, 50, 1000 / 30, 0, 20];
  assert.deepEqual(summary(values), { count: 5, p50: 1000 / 30, p95: 51, p99: 51, max: 51, over33ms: 2, over50ms: 1 });
  assert.equal(values[0], 51);
  assert.throws(() => summary([NaN]), TypeError);
  assert.throws(() => summary([-1]), TypeError);
});
