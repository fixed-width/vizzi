# Repository QA experiment architecture

Scope at baseline `5ee084d9e7d6876a61af627403ca99fe208d043f`; read
[the behaviour contract](PRODUCT.md) first. This is an isolated experimental crate and
runner, not a new dependency of the viewer or a settled inference/storage architecture.

`vizzi-qa` uses Rust 2024 and inherited safe-Rust/documentation lints. Its library owns
bounded text chunks, deterministic lexical retrieval, content-addressed corpus identity,
incremental replacement/removal and source-bound response validation. Native capture
uses existing no-follow directory input. Source is read but never executed. Source and
documentation scope is explicit; code-like LPC files remain excluded.

Use captured declaration boundaries where available and finite overlapping source
windows otherwise. Bound excerpts and total query context. Tokenize ordinary words and
code identifiers; start with a lexical retrieval baseline so its limitations can be
measured before adding embeddings. Reuse unchanged chunking after verified content
comparison. Rebuild derived lookup structures when needed and report that cost honestly.

A socket-free native process retains the corpus and exchanges bounded JSON lines with
the experiment runner. Each retrieved context carries a snapshot and source identities.
The runner supplies only question text and these excerpts to a separately launched
llama.cpp server on loopback. Model/runtime identifiers, hashes, flags and prompts are
recorded. GPU offload is disabled; requests and models run sequentially with two threads.
Node handles experiment orchestration and HTTP, as in the existing browser harness;
retrieval and provenance rules stay in Rust. No new third-party Rust dependency is needed.

Gold cases live outside frozen source archives and are never indexed. Keep reusable
templates separate from concrete cases. Validate evidence needles against the pinned
source before running. Generate a labelled oracle-source diagnostic to distinguish
retrieval failure from model failure. A model response references prompt-local evidence
labels and literal quotes; Rust checks those bindings against the exact context.
Human/agent rubric scoring remains separate from deterministic provenance checks.

Use a deterministic changing-source fixture for rename/delete/same-metadata updates and
held-context refusal. Real-repository cases test cross-file source and documentation
questions. Record cache state, omissions, input/output bounds and incomplete runs.
Use the existing safe process-tree memory observer where available. Performance does
not overlap correctness tests or mutation cohorts. TDD and serial cargo-mutants runs
exercise retrieval/update/validation behavior before interpreting model results.
