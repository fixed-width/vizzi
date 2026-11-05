# Question library reference

This directory is for evaluation authors. `templates.json` contains reusable questions;
`cases.json` contains concrete source-bound instances and codebase-specific questions.
The initial seed has 16 templates and 32 cases: 12 general questions instantiated on real
repositories, 12 codebase-specific questions, four operational questions whose answers
are absent, and four temporal-fixture questions including deletion. There are 27
answerable and five unanswerable cases, split into 18 development and 14 held-out cases.

A case carries these fields:

| Field | Meaning |
| --- | --- |
| `id` | Stable unique identity; use a new ID for a changed oracle/revision. |
| `repository` | Entry in the source manifest; real sources have immutable Git pins. |
| `question` | Only this text and supplied source go to the model. |
| `category` | Task family, independently of the template relationship. |
| `template` | Reusable template ID, or null for a codebase-specific formulation. |
| `split` | Development or held-out, fixed before candidate output is inspected. |
| `expected_status` | Answerable from the selected repository source, or insufficient evidence. |
| `required_facts` | Semantic assertions needed for the answer, each with one or more source alternatives. |
| `evidence` | Relative path and a unique literal needle supporting that assertion at the pin. |
| `forbidden_claims` | Important misconceptions or unsupported conclusions. |
| `oracle_diagnostic` | Whether to run a gold-selected-source diagnostic as well as ordinary retrieval. |
| `fixture_state` | Explicit changing-source revision label; not a fabricated Git commit. |

Add questions that a newcomer could ask before knowing paths or symbol names. Reusable
probes cover purpose, language/toolchain, entry points, configuration, source flow,
errors, extension interfaces, persistence, concurrency, invalidation and tests. A
question about an internal named component is also useful but should be labelled with
its assumed familiarity. Avoid embedding the desired answer in the question.

For each concrete case, inspect the pinned source and write the factual checklist
before running candidates. Require only facts the question asks for. Accept alternative
valid evidence and equivalent wording. Include scope/configuration caveats instead of
turning a syntax observation into a resolved call, runtime or impact claim. Do not turn
absence from one retrieved window into absence from the repository.

Choose short unique evidence needles. Preparation checks their existence and verifies
that oracle excerpts actually contain the anchors. The current oracle needle limit is
1,800 UTF-8 bytes; source excerpts and context have independent limits. Gold prose and
selectors stay outside frozen source archives and never enter a retrieval prompt.

Freeze the library hash for a cohort. After seeing results, keep ambiguous or misleading
seed cases visible in that cohort's report; record review notes and create revised cases
for the next version rather than improving scores by editing/removing failures.
Maintainer-authored questions and independent rubric review are needed before treating
this agent-authored seed as a representative comprehension benchmark.
