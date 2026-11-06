# Fixed-weight prefill resource diagnostic

For contributors investigating the 4B candidate's observed 4.76 GiB peak after `47f41c6`.
Keep the exact Q4_K_M weights, runtime, source, prompts, answer-first schema, output limits,
8,192-token context, cache types, two CPU threads, nice 10 and no-GPU policy unchanged.
Do not alter weights or choose a profile using answer scores during the resource screen.

Freeze three profiles before inference: logical batch 512 for all, physical microbatch
128 (control), 32 and 8. Smaller physical batches may reduce scratch-buffer memory but
must be measured; they may also cost prefill speed and change floating-point outputs.
The local pinned runtime describes batch as the logical maximum and ubatch as the
physical maximum. Prior defaults stay 512/128 unless an explicit profile is selected.

Stage 1 runs the same complete-source broad commit question once per profile under its
own process-tree observer. Include initialization, full prompt prefill and normal answer
generation; use a uniform 600-second request timeout for this larger resource screen.
Record startup, actual arguments, memory, latency, tokens, errors and unchanged weight
hash. Every process closes before the next. Resource-screen outputs are diagnostics,
not a quality benchmark. If a request fails, retain the failure rather than silently
changing context or model settings.

Select the fastest full response among profiles with successful execution and at most
4 GiB observed peak PSS. Selection uses resource results only. If none qualify, report
the budget miss and stop this tuning cohort. Do not invent a fit from the weight size.
Stage 2, when qualified, runs all fifteen unchanged questions once on the selected
profile, with complete source and the same response settings. Compare actual prompts,
source hashes, field order and graded content with the prior 4B control. Record changed
answers; identical weights do not imply identical outputs under changed batching.

The source-specific case filter and profile selection are explicit CLI arguments and
must match their preparation. Freeze permitted profiles, selected case IDs and code/data/
binary hashes before each stage. The native selection/provenance engine and dependencies
do not change. Use TDD for Node profile/argument/selection logic and normal baseline checks;
do not rerun redundant native mutation cohorts. Finish correctness jobs before inference.

Preserve original runs and the dispatch oracle erratum. Use corrected scoring for the
receiver and never call the real dispatch method invented. Grade remaining model facts,
unsupported claims, citation support and task completeness independently of resource fit.
No training, weight conversion, planner, chat UI, editor or Vizzi startup is included.
