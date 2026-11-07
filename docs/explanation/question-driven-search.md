# Questions as an entry into source views

For contributors designing how a small local model assists unfamiliar-codebase exploration.
The useful output is a focused collection of source locations that a developer can inspect
and carry into coordinated views. A natural-language question can express intent before
the developer knows symbol names, directory structure or implementation vocabulary.

The source engine supplies captured declarations, literal excerpts, roles/scopes and
version-bound locations. A model can help choose search terms or compare those results
against the question. Those are separable roles: the
[first experiment](../qa/SEARCH.md) measures ordering a fixed candidate pool. Query expansion
and relationship traversal require their own evidence before adding them. Reranking cannot
recover a constructor or caller that candidate generation never found.

This fits Vizzi's purpose as an editor companion. A question such as “where are new
transactions created?” should lead to plausible implementation and caller locations,
with enough source to choose what to inspect next. Ordering alone does not establish
which caller executes in a configuration or prove a complete call path. Language analysis
and its coverage must support any displayed relationship; unresolved edges stay explicit.

The model's ordering is a suggestion. The engine retains the original pool, canonical
identities and source bytes, validates the proposed labels and handles stale captures.
Keeping every candidate available protects against an incorrectly demoted result. A
deterministic baseline makes model failure recoverable and supports use without inference.
An eventual interactive host can show initial results while optional ranking runs; a
slow model should not prevent the developer from beginning their investigation.

Permutation generation for passage ranking is studied by
[Sun et al.](https://aclanthology.org/2023.emnlp-main.923/). Their findings motivate a bounded
ranking interface, not a claim that a general 0.8B model will rank source code well.
[Liu et al.](https://arxiv.org/abs/2307.03172) document sensitivity to relevant information's
position in supplied contexts. Together with our own layout failures, that motivates
checking reversed presentation as well as ranking relevance. Neither paper validates
the codebase-specific candidate pool or its manual grades.

The earlier [answer experiments](../../experiments/qa/README.md) remain useful evidence
about source selection and model limits. Their complete-answer rubric is a different
task from reducing the number of unhelpful source results a developer must inspect.
Search should be judged by useful starting points, evidence coverage, task progress,
latency and resource cost, with independent maintainer review still needed.

The [candidate-coverage comparison](../../experiments/qa/SEARCH_COVERAGE_RESULTS.md) adds
raw macro declarations and deterministic topic/operation pools. It retrieves the efun
macro and commit validator but loses the known transaction constructor while returning
other creation sites. This illustrates why aggregate locator gains are insufficient:
source ownership, useful baseline hits and the developer's actual navigation task must
survive expansion. No model runs in that comparison, so its trade-off belongs to search
selection rather than model reasoning.
