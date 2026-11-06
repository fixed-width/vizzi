# Dispatch oracle correction

This erratum supersedes affected source-audit/grading judgments in the source-pack and
decomposition diagnostics. The original frozen libraries, raw model outputs, grades,
assessments and checksums are retained. No model inputs or inference settings change.

The evaluator incorrectly treated `Committer::process` as the direct Commit-message
handler and called `Committer::dispatch` an invented method. That was an agent source-audit
error, not a model error. At the pinned lpc-rs revision
`1a4f5b65dd2a269298748df2dfd68092a169a8d9`, the actual path is:

```text
Committer::run
  -> process(msg, tx)       # public timing/Stats wrapper, lines 263-277
     -> dispatch(msg, tx)   # declaration at line 279
        -> Commit arm      # lines 302-321
           -> commit(changeset)
              -> publish(...)
```

The model-facing source was already correct. Its captured lexical scope for the Commit
arm is `Committer::dispatch`; the evaluator failed to check that against the surrounding
definition. The [literal source audit](results/dispatch-erratum/source-audit.json) records
the full-file hash, wrapper, handler and call. A data regression test rejects the old
receiver oracle and accepts the corrected one against that captured scope.

## Revised judgments

Both receiver subanswers in the output-order diagnostic correctly name `dispatch` and
its call to `commit`. They now pass. Naming the actual method is no longer an unsupported
claim in broader answers or the base-pin answer. Separate incorrect request/reply
directions, validator/handler conflation, missing branches and wrong citations remain
failures where they actually occur.

| Earlier output-order diagnostic | As originally recorded | Corrected |
| --- | ---: | ---: |
| Status-first narrow complete tasks | 2/12 | 3/12 |
| Answer-first narrow complete tasks | 4/12 | 5/12 |
| Status-first narrow supported facts | 10/18 | 11/18 |
| Answer-first narrow supported facts | 13/18 | 14/18 |
| Broad complete workflows, either order | 0/3 | 0/3 |

Answer-first still helps two paired narrow tasks; three now succeed under both orders
and seven fail under both. The source-pack cohort's overall 0/3 complete-task scores and
fact counts do not change, but its short commit summary loses the false invention finding.
No model rerun is necessary: evaluator gold/forbidden claims never entered prompts.

The current 2B/4B model comparison uses the corrected receiver oracle and corrected live
baseline grades. All other questions, source and conditions remain frozen. This is a
post-output source-audit correction, clearly distinct from tuning a prompt or question to
improve a model score.

## Preserved evidence

- [Changed judgments, before/after](results/dispatch-erratum/changes.json)
- [Corrected receiver library](results/dispatch-erratum/decomposition-library.corrected.json)
- [Corrected decomposition grades](results/dispatch-erratum/decomposition-grades.corrected.json)
- [Corrected paired assessment](results/dispatch-erratum/decomposition-assessment.corrected.json)
- [Corrected source-pack grades](results/dispatch-erratum/source-packs-grades.corrected.json)
- [Corrected source-pack assessment](results/dispatch-erratum/source-packs-assessment.corrected.json)

This error reinforces the existing limitation: source-aware agent grading is not
independent acceptance. Main conclusions about omitted lifecycle branches, incorrect
read-only/retry reasoning and citation identity remain inspectable in raw output, but
the method-name invention claim is withdrawn. Fresh questions and independent human
source review are required before any product-quality claim.
