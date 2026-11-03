export function describeSource(source) {
  if (!source) return { coverage: "Read-only · no source selected", diagnostics: [] };
  const { classification, outlines, encoding } = source;
  const diagnostics = outlines.flatMap(outline => outline.diagnostics);
  const provenance = classification.provenance;
  const evidence = provenance.kind === "extension" ? `by .${provenance.extension} extension` : `assigned for ${provenance.scope}`;
  let coverage;
  if (classification.kind === "source_only") {
    coverage = `Source only · ${evidence}`;
    const explanations = {
      language_unassigned: "C/C++ language is unassigned; source is available without an outline",
      dialect_unsupported: "CUDA analysis is unavailable; source remains readable",
      fragment_unassigned: "Textual fragment has no language assignment",
      analyzer_unavailable: `${classification.language} analyzer is unavailable; source remains readable`,
    };
    diagnostics.push(explanations[classification.reason]);
  } else {
    coverage = `Read-only · ${outlines.map(outline => `${outline.context.language}: ${outline.coverage === "syntax" ? "syntax parsed" : outline.coverage === "partial" ? "partial syntax" : outline.coverage === "deferred" ? "analysis deferred" : "analysis unavailable"}`).join(" · ")} · raw syntax · configuration unknown · ${evidence}`;
  }
  if (encoding === "unsupported" && !diagnostics.some(text => text.includes("UTF-8"))) diagnostics.push("Source is not UTF-8; text preview is unavailable");
  return { coverage, diagnostics };
}
