# Provisional interactive resource budgets

These are proposed engineering targets informed by
[full-repository measurements](../../experiments/qualification/RESULTS.md). They are
not predeclared acceptance criteria, enforced limits, or proof of the 16 GB laptop
baseline. Update the targets with hardware and workflow evidence rather than silently
claiming the current implementation passes them.
The [responsiveness follow-up](../../experiments/qualification/RESPONSIVENESS_RESULTS.md)
updates ordinary edits and outline rendering; it does not repeat retained-reparse or
cancellation qualification.
The [cancellation follow-up](../../experiments/qualification/CANCELLATION_RESULTS.md)
repeats four-worker retained reparse and measures cooperative interruption within files.

| Area | Proposed target / policy | Current evidence |
| --- | --- | --- |
| Interactive parsing | Four shared workers by default | Preserves throughput without separate per-language pools |
| Background throughput | Eight only with available resources and measured query impact | Retained reparse improves about 1.6×; Linux core-query p95 grows from at most 44.4 to 52.6 ms |
| Native workspace memory | Aim below 8 GiB peak while retaining the previous observation | Maximum sampled Linux reparse RSS 6.492 GiB on a 62.5 GiB host; no enforced cap |
| Bounded core view | Aim for p95 below 50 ms for the current 2,000-tile query | Four-worker observations fit on this host; eight-worker Linux has misses |
| Ordinary edit visibility | Aim for all five probe edits below 200 ms before collecting meaningful tails | Follow-up: PyTorch 98–121 ms; Linux 135–192 ms. Five samples each fit, with no tail claim; periodic recovery can still dominate |
| Cancellation response | Aim below 250 ms after a request during parsing | Follow-up direct engine-return ranges: Linux 41–46 ms; PyTorch 18–23 ms. Three samples each; cooperative callbacks do not impose a hard deadline |
| Source and outline DOM | Bound each pane by visible rows plus overscan; initial ceiling 200 rendered entries | Source lines 56–58; 42,443-declaration C++ outline now renders 21 buttons. Complete transport/index construction remains linear in source/declaration size |
| Overview motion | Maintain rAF p99 below 33.3 ms in the software reference workload | 16.8 ms in all recorded steady/refresh runs; physical presentation unqualified |

Cold indexing is a separate throughput/startup measure: about 60 s Linux and 11 s
PyTorch at four workers. The host currently exposes its browser URL only after initial
publication. Progress/partial exploration during cold indexing is a separate design
decision, not something the existing loading behavior already provides.

The native memory target leaves room for an editor, browser, OS, and filesystem cache,
but the combined envelope has not been validated. Point-in-time renderer heap is not
peak browser RSS; this host has neither a 16 GiB cgroup cap nor baseline-laptop hardware.
Per-file size/admission and process-wide scheduling remain open. The separate
cooperative cancellation implementation does not impose a hard deadline on a long parse.

## Work before compiler-context expansion

Bounded outline DOM and verified plan compatibility are implemented. Linux small-source
engine attempts fell from 128–153 ms to 42–45 ms. Merging/hashing still scales with
snapshot size, and full source/outline responses remain large. Keep the follow-up's
measurements separate from the initial cold/reparse matrix.

Cooperative built-in parser/extraction cancellation is implemented and measured. It
preserves old publications and resets interrupted parsers; necessary complete edits
still require analysis and transfer.

1. Design bounded hot-file incremental trees using the
   [session/admission plan](../plans/parser-responsiveness.md).
   Keep immutable published facts separate from mutable parser/tree ownership. Tree cache
   memory must be admitted within an explicit budget, not multiplied by every language.
2. Reduce full-capture memory overlap, for example by streaming bounded input batches
   and sharing immutable captured source across analysis-context changes. Keep byte
   verification and pinned-source consistency; metadata alone is still not proof.
3. Bound selected-source and outline transport/projection work with coherent snapshot
   and selection semantics. DOM virtualization alone does not bound transferred data
   or eliminate changed-version index construction.

Do not replace semantic correctness checks with these performance targets. Language
assignments, partial grammar coverage, source-only cases, cancellation, cache validity,
and immutable publication must remain explicit and independently tested.
