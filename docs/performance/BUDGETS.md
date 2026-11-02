# Provisional interactive resource budgets

These are proposed engineering targets informed by
[full-repository measurements](../../experiments/qualification/RESULTS.md). They are
not predeclared acceptance criteria, enforced limits, or proof of the 16 GB laptop
baseline. Update the targets with hardware and workflow evidence rather than silently
claiming the current implementation passes them.

| Area | Proposed target / policy | Current evidence |
| --- | --- | --- |
| Interactive parsing | Four shared workers by default | Preserves throughput without separate per-language pools |
| Background throughput | Eight only with available resources and measured query impact | Retained reparse improves about 1.6×; Linux core-query p95 grows from at most 44.4 to 52.6 ms |
| Native workspace memory | Aim below 8 GiB peak while retaining the previous observation | Maximum sampled Linux reparse RSS 6.492 GiB on a 62.5 GiB host; no enforced cap |
| Bounded core view | Aim for p95 below 50 ms for the current 2,000-tile query | Four-worker observations fit on this host; eight-worker Linux has misses |
| Ordinary edit visibility | Aim for all five probe edits below 200 ms before collecting meaningful tails | PyTorch 115–152 ms; Linux 249–2,155 ms fails this proposed gate |
| Cancellation response | Aim below 250 ms after a request during parsing | Observed maxima 845 ms / 2,265 ms; whole-file parsing prevents prompt cancellation |
| Source and outline DOM | Bound each pane by visible rows plus overscan; initial ceiling 200 rendered entries | Source lines 56–58; C++ outline creates 42,443 buttons and stalls |
| Overview motion | Maintain rAF p99 below 33.3 ms in the software reference workload | 16.8 ms in all recorded steady/refresh runs; physical presentation unqualified |

Cold indexing is a separate throughput/startup measure: about 60 s Linux and 11 s
PyTorch at four workers. The host currently exposes its browser URL only after initial
publication. Progress/partial exploration during cold indexing is a separate design
decision, not something the existing loading behavior already provides.

The native memory target leaves room for an editor, browser, OS, and filesystem cache,
but the combined envelope has not been validated. Point-in-time renderer heap is not
peak browser RSS; this host has neither a 16 GiB cgroup cap nor baseline-laptop hardware.
Per-file size/admission, process-wide scheduling, and cancellation inside a long parse
are not implemented merely because a worker limit exists.

## Work before compiler-context expansion

1. Bound declaration-outline rendering and its data work. Preserve identity, keyboard
   selection, and source linking while rendering only visible rows. Treat large-file
   opening and updates as separate cases from overview pan/zoom.
2. Make compatible-snapshot validation and retained-document selection cheap. A verified
   policy/context epoch can avoid reclassifying every file on a single-file hint; preserve
   clean-capture equivalence and the invalid-baseline safeguards when designing this seam.
3. Investigate safe parser progress cancellation and bounded hot-file incremental trees.
   Keep immutable published facts separate from mutable parser/tree ownership. Tree cache
   memory must be admitted within an explicit budget, not multiplied by every language.
4. Reduce full-capture memory overlap, for example by streaming bounded input batches
   and sharing immutable captured source across analysis-context changes. Keep byte
   verification and pinned-source consistency; metadata alone is still not proof.

Do not replace semantic correctness checks with these performance targets. Language
assignments, partial grammar coverage, source-only cases, cancellation, cache validity,
and immutable publication must remain explicit and independently tested.
