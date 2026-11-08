# Choose views by the questions they help answer

Vizzi exists to help developers understand unfamiliar source and review changes. A
visualisation earns its place when it helps answer a question, preserves useful context
or exposes evidence that is awkward to recover otherwise. More diagrams and faster
indexing do not, by themselves, establish that benefit.

This is a focused research update for the [next product milestones](../plans/product-milestones.md),
not an exhaustive survey or a final renderer decision. Literature findings and Vizzi's
design inferences are distinguished below.

The 2026-11-08 maintainer clarification now selects a
[visual working-set/call-trail interaction](visual-working-sets.md), with Code Bubbles as
the closest reference. Earlier suggestions below to prioritise a small replacing trail
before a canvas describe the prior checkpoint; the canvas interaction is now the active
product hypothesis. The source/relationship confidence and evaluation limits still apply.

## Evidence and its limits

| Primary source | Finding or contribution | Implication for Vizzi; limit |
| --- | --- | --- |
| Sillito, Murphy and De Volder, **Questions Programmers Ask During Software Evolution Tasks**, FSE 2006. [Author-hosted paper](https://www.cs.ubc.ca/~murphy/papers/other/asking-answering-fse06.pdf), [DOI](https://doi.org/10.1145/1181775.1181779). | Two qualitative studies catalogue 44 kinds of questions, from locating starting points to connecting and integrating information. | Choose a concrete investigation and record the questions/navigation it requires. This motivates task-based evaluation; it does not identify a universally best visual encoding. |
| Shneiderman, **The Eyes Have It: A Task by Data Type Taxonomy for Information Visualizations**, 1996. [Paper](https://vis.csail.mit.edu/classes/6.859/readings/pdfs/Shneiderman-TheEyesHaveIt.pdf). | Separates data types from tasks including overview, filtering, relating items, history and extraction. | Keep overview, focused relationships and source detail coordinated, with a way back to prior context. This is a design framework, not a controlled evaluation of Vizzi or software graphs. |
| Bragdon et al., **Code Bubbles: A Working Set-based Interface for Code Understanding and Maintenance**, CHI 2010. [Publisher](https://doi.org/10.1145/1753326.1753706), [author-hosted paper](https://andrewbragdon.com/papers/p2503-bragdon.pdf). | Explores concurrently visible, method-sized working sets and navigation between related fragments. | Make relevant code easy to revisit; evaluate a small investigation trail before adopting a whole canvas-of-fragments interface. Its Java IDE setting and study tasks do not establish results for mixed-language repositories or this read-only browser. |
| Wettel, Lanza and Robbes, **Software Systems as Cities: A Controlled Experiment**, ICSE 2011. [Author-hosted paper](https://wettel.github.io/download/Wettel11a-icse.pdf), [replication materials](https://wettel.github.io/codecity-experiment.html), [DOI](https://doi.org/10.1145/1985793.1985868). | Evaluates the CodeCity metaphor on reverse-engineering tasks with Eclipse/metric-based controls. | City and hierarchy/metric views are credible candidates when those questions matter. That evidence does not establish their suitability for line-level review, Rust/Python/C++ semantics or a GPU backend. |
| Bacchelli and Bird, **Expectations, Outcomes, and Challenges of Modern Code Review**, ICSE 2013. [Microsoft Research](https://www.microsoft.com/en-us/research/publication/expectations-outcomes-and-challenges-of-modern-code-review/). | Observations, interviews, surveys and review-comment analysis identify code/change understanding as central, with knowledge-sharing benefits beyond defect finding. | A review milestone must connect changes to surrounding source and contracts. Diff correctness alone is insufficient; this study does not compare Vizzi-style view layouts. |
| **Graph-based visualization of merge requests for code review**, Journal of Systems and Software 195, 111506, 2023. [Paper](https://www.sciencedirect.com/science/article/pii/S0164121222001820), [DOI](https://doi.org/10.1016/j.jss.2022.111506). | ReviewVis combines changed entities and relationships; its evaluations collect developer perceptions of usefulness/navigation. The paper reports limits for very small and large changes. | Explore a focused change/context view, retaining a conventional diff. Perceived usefulness is not proof of faster reviews or increased defect detection. Its semantic relationships are richer than Vizzi's current lexical facts. |
| Wurzel Gonçalves et al., **Code Review Comprehension: Reviewing Strategies Seen Through Code Comprehension Theories**, 2025 author preprint. [Paper](https://arxiv.org/html/2503.21455v1). | Observations/interviews with ten experienced reviewers over 25 reviews describe context-building and opportunistic inspection strategies. | Let a reviewer move from the change to relevant unchanged code and back without losing the comparison. This qualitative model does not establish a causal benefit from a particular visualisation. |

The resulting view choices are project hypotheses. None of these sources establishes
that a force-directed graph, 3D city or GPU renderer is necessary for the first useful
workflow.

## A small set of complementary perspectives

| User question | Candidate perspective | Evidence needed before presenting it |
| --- | --- | --- |
| Where is this subsystem, and what source is available? | Existing directory/file overview plus filtering and declaration lookup. | Actual captured paths, language assignments and coverage, including omissions. |
| Which source modules does this area declare or import? | A bounded neighbourhood of module/import declarations, with a keyboard-accessible relationship list. | Typed syntax facts, literal paths and source spans; conservative target matching with explicit unresolved cases. |
| What source supports this explanation? | Existing source/outline detail and a small, session-local investigation trail. | Exact document versions and anchors; expired observations must be explicit. |
| What changed, and what surrounding code explains it? | Change overview, before/after source diff and declarations, linked to unchanged context on the selected side. | A frozen pair of source observations, complete change inventory and side-specific anchors. |
| Where are size, complexity or historical hotspots concentrated? | Treemaps/cities, heatmaps or temporal views. | Suitable metrics/history, an actual question requiring them and a separate task evaluation. These are later candidates. |

Start the learning milestone with a walkthrough of the existing prototype. A graph is
not an acceptance criterion by itself. If navigation/history solves the observed problem,
do not add edges merely to increase the number of views. If a structural view is justified,
start with the selected module's neighbourhood and expand on demand, reporting hidden
counts and coverage. Whole-repository node counts are not a usefulness measure.

## Relationship meaning is part of the interface

The implemented [language adapters](../prototype/LANGUAGES.md) currently expose lexical
declarations, not resolved dependencies. An import declaration is evidence that text
declares an import. It is not proof of a call, runtime dependency, selected build
configuration or transitive change impact.

The first structural candidate therefore distinguishes module containment, declared
imports and unresolved/external references. Every visible relationship leads to the
source that supports it. Rust is the first extractor; Python/C/C++ remain usable through
existing views and clearly report unavailable relationship analysis until their own
capabilities exist. A Rust import and a C include must not silently acquire identical
semantic meaning.

For review, changed source and declarations provide orientation. Related unchanged code
provides context. The interface must distinguish both from an inferred impact set, which
these raw facts cannot establish. Automated reviewer recommendations, call-graph completeness
and change-risk scores require different evidence.

## Evaluate usefulness separately from performance

For each milestone, retain the question, pinned input, expected evidence, observed
navigation, incorrect conclusions and remaining uncertainty. Compare the task walkthrough
with the existing map/source/outline baseline. A maintainer or agent replay establishes
functional feasibility, not an unfamiliar developer's comprehension benefit. A stronger
usability claim requires suitable human evaluation and recorded participant familiarity.

Keep the [existing resource evidence](../performance/BUDGETS.md) as an engineering
constraint. Measure the actual new relationship/comparison workload when it exists;
fast synthetic drawing or syntax parsing cannot validate its usefulness. Rust/wgpu and
other renderer choices remain open until representative views expose their requirements.
