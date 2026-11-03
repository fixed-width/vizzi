import test from "node:test";
import assert from "node:assert/strict";
import { describeSource } from "./source-description.js";

const source = (reason, language = null) => ({ encoding: "utf8", classification: { kind: "source_only", reason, language, provenance: { kind: "extension", extension: "h" } }, outlines: [] });
test("source-only reasons, assignment evidence, and encoding stay distinct", () => {
  assert.match(describeSource(null).coverage, /no source/);
  assert.deepEqual(describeSource(null).diagnostics, []);
  for (const [reason, language, message] of [["language_unassigned", null, "language is unassigned"], ["dialect_unsupported", "cuda", "CUDA"], ["fragment_unassigned", null, "fragment"], ["analyzer_unavailable", "cpp", "cpp analyzer is unavailable"]]) {
    const description = describeSource(source(reason, language));
    assert.match(description.coverage, /Source only/);
    assert.ok(description.diagnostics.some(text => text.includes(message)));
    assert.ok(description.coverage.includes(".h extension"));
  }
  const input = source("language_unassigned"); input.encoding = "unsupported";
  const description = describeSource(input);
  assert.equal(description.diagnostics.length, 2);
  assert.ok(description.diagnostics[0].includes("language is unassigned"));
  assert.ok(description.diagnostics[1].includes("UTF-8"));
  input.classification.provenance = { kind: "explicit_scope", scope: "include" };
  assert.ok(describeSource(input).coverage.includes("assigned for include"));
});
test("raw analysis reports recovery and unknown configuration without inventing compiler coverage", () => {
  for (const [coverage, label] of [["syntax", "syntax parsed"], ["partial", "partial syntax"], ["unavailable", "analysis unavailable"]]) {
    const input = { encoding: "utf8", classification: { kind: "assigned", language: "rust", provenance: { kind: "extension", extension: "rs" } }, outlines: [{ context: { language: "rust" }, coverage, diagnostics: ["existing diagnostic"] }] };
    const description = describeSource(input);
    assert.ok(description.coverage.includes(`rust: ${label}`));
    assert.ok(description.coverage.includes("raw syntax"));
    assert.ok(description.coverage.includes("configuration unknown"));
    assert.deepEqual(description.diagnostics, ["existing diagnostic"]);
    assert.deepEqual(input.outlines[0].diagnostics, ["existing diagnostic"]);
    input.encoding = "unsupported"; input.outlines[0].diagnostics = ["Source is not UTF-8"];
    assert.deepEqual(describeSource(input).diagnostics, ["Source is not UTF-8"]);
    input.outlines[0].diagnostics = ["Existing warning", "Source is not UTF-8"];
    assert.deepEqual(describeSource(input).diagnostics, ["Existing warning", "Source is not UTF-8"]);
  }
  const combined = { encoding: "utf8", classification: { kind: "assigned", language: "rust", provenance: { kind: "extension", extension: "rs" } }, outlines: [
    { context: { language: "rust" }, coverage: "syntax", diagnostics: [] },
    { context: { language: "python" }, coverage: "partial", diagnostics: [] },
  ] };
  assert.ok(describeSource(combined).coverage.includes("rust: syntax parsed · python: partial syntax"));
});
test("deferred analysis preserves language assignment and its resource explanation", () => {
  const input = { encoding:"utf8", classification:{kind:"assigned",language:"cpp",provenance:{kind:"explicit_scope",scope:"."}},
    outlines:[{context:{language:"cpp"},coverage:"deferred",diagnostics:["Analysis deferred: active input capacity is occupied; source remains available"]}] };
  const description = describeSource(input);
  assert.ok(description.coverage.includes("analysis deferred"));
  assert.ok(description.coverage.includes("assigned for ."));
  assert.deepEqual(description.diagnostics, input.outlines[0].diagnostics);
  assert.ok(!description.coverage.includes("Source only"));
});
