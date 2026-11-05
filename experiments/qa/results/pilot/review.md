# Source-aware response review

This evaluator artifact contains gold facts. Do not index it or supply it to candidates.

## qwen3.5-0.8b-q4/vizzi.purpose/retrieved

What is Vizzi intended to help developers do, and how does it fit alongside an editor?

Expected: answered; split: development; category: orientation.

- Fact 1: It supports learning unfamiliar code and reviewing changes.
- Fact 2: It supplements the editor; normal source reading/editing remain in the editor.

Candidate status: answered

````text
Vizzi is designed to help developers understand unfamiliar source code and assist with code reviews by providing coordinated views of the same material. It fits alongside an editor by serving as a bounded source preview that supports evidence inspection and version accuracy, while the editor remains the main surface for reading and changing source. The system uses a task-driven approach to help developers answer questions about unfamiliar codebases.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/vizzi.purpose/oracle_source

What is Vizzi intended to help developers do, and how does it fit alongside an editor?

Expected: answered; split: development; category: orientation.

- Fact 1: It supports learning unfamiliar code and reviewing changes.
- Fact 2: It supplements the editor; normal source reading/editing remain in the editor.

Candidate status: answered

````text
Vizzi is designed to help developers assemble and navigate unfamiliar codebases by providing coordinated views of source material, supporting code reviews, and assisting with learning unfamiliar repositories. It fits alongside an editor by acting as a supplementary tool that supplements the editor's capabilities, allowing developers to discover unfamiliar codebases without prewritten explanations, while the editor remains the primary surface for reading and changing source.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/vizzi.language/retrieved

What implementation language and Rust edition does this workspace use?

Expected: answered; split: development; category: orientation.

- Fact 1: Rust is the implementation language.
- Fact 2: The workspace declares edition 2024, inherited by crates.

Candidate status: answered

````text
The workspace uses Rust 2024 edition and tree-sitter-rust 0.24.2.
````

- Citation 1: AGENTS.md — "edition.workspace = true"
- Citation 3: docs/prototype/TECH.md — "tree-sitter-rust = \"0.24.2\""
- Citation 5: Cargo.toml — "edition = \"2024\""

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/vizzi.language/oracle_source

What implementation language and Rust edition does this workspace use?

Expected: answered; split: development; category: orientation.

- Fact 1: Rust is the implementation language.
- Fact 2: The workspace declares edition 2024, inherited by crates.

Candidate status: answered

````text
The workspace uses Rust with the 2024 edition.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/vizzi.entry/retrieved

Where does the native Vizzi server start, and which interface does it bind to?

Expected: answered; split: development; category: architecture.

- Fact 1: The native entry is crates/vizzi/src/main.rs.
- Fact 2: The HTTP listener binds IPv4 localhost/loopback.

Candidate status: answered

````text
The native Vizzi server starts in `crates/vizzi/src/main.rs` at `host_start` (IPv4 TCP socket creation with errno 1) and binds to `localhost` on port `port`. It is the server-side component that handles HTTP requests and diagnostic responses, distinct from the browser-side UI logic and review interface.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 1/2.

## qwen3.5-0.8b-q4/vizzi.configuration/retrieved

How can I choose a different HTTP port when launching Vizzi?

Expected: answered; split: development; category: configuration.

- Fact 1: Use the --port command-line option.
- Fact 2: The guide gives --port 7475 as an example.

Candidate status: answered

````text
You can choose a different HTTP port by using the `--port` flag in the `cargo run` command, for example: `cargo run --release -p vizzi -- --port 7475`. This sets the loopback port to 7475 while keeping the root path at 7474.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 1/2.

## qwen3.5-0.8b-q4/vizzi.validation/retrieved

Which baseline checks does this repository require before validating a change?

Expected: answered; split: held_out; category: testing.

- Fact 1: Formatting is a baseline check.
- Fact 2: Workspace tests including doctests are required.
- Fact 3: Clippy is required.

Candidate status: insufficient_evidence

````text
The provided excerpts do not explicitly state the baseline checks required before validating a change. While they mention that the system validates against a 'frozen reference repository' and that 'explicit scopes enable raw C/C++ analysis,' they do not list specific baseline checks or define the criteria for validation. The text focuses on the workflow, editor features, and performance metrics rather than the specific validation prerequisites.
````


Provenance: true; . Preferred anchors: 0/3.

## qwen3.5-0.8b-q4/vizzi.validation/oracle_source

Which baseline checks does this repository require before validating a change?

Expected: answered; split: held_out; category: testing.

- Fact 1: Formatting is a baseline check.
- Fact 2: Workspace tests including doctests are required.
- Fact 3: Clippy is required.

Candidate status: insufficient_evidence

````text
The provided excerpts do not explicitly list the specific baseline checks (e.g., formatting, workspace tests, Clippy) that this repository requires before validating a change. While they mention validating native and browser implementations, they do not detail the specific checklist items.
````


Provenance: true; . Preferred anchors: 3/3.

## qwen3.5-0.8b-q4/vizzi.safety/retrieved

What is the policy on unsafe Rust, and are performance gains enough to justify an exception?

Expected: answered; split: held_out; category: constraints.

- Fact 1: Safe Rust and inherited deny unsafe_code are the default.
- Fact 2: Exceptions need a requirement with no viable safe interface; performance or convenience alone do not justify them.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish a specific policy on unsafe Rust or a clear determination whether performance gains justify exceptions. While excerpt [1] mentions introducing unsafe code only for unavoidable FFI and citing safety as a priority, and excerpt [3] notes that safe wrappers satisfy the existing unsafe-code denial, neither excerpt explicitly defines a policy or provides evidence that performance gains are sufficient to override safety requirements. Therefore, the answer cannot be definitively given based on the supplied text alone.
````


Provenance: true; . Preferred anchors: 1/2.

## qwen3.5-0.8b-q4/vizzi.safety/oracle_source

What is the policy on unsafe Rust, and are performance gains enough to justify an exception?

Expected: answered; split: held_out; category: constraints.

- Fact 1: Safe Rust and inherited deny unsafe_code are the default.
- Fact 2: Exceptions need a requirement with no viable safe interface; performance or convenience alone do not justify them.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish the policy on unsafe Rust or the justification for performance gains. They only mention general documentation standards and the general principle that unsafe code should be rare and narrowly scoped, but they lack specific examples, code snippets, or detailed discussions regarding performance trade-offs that would support a definitive answer.
````


Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/vizzi.search/retrieved

Does the repository discovery search inspect full source bodies or just names and paths?

Expected: answered; split: development; category: discovery.

- Fact 1: Search matches captured declaration names and paths, not full source bodies.
- Forbidden: Claims full-text or semantic source-body search.

Candidate status: insufficient_evidence

````text
The provided excerpts do not explicitly state whether the discovery search inspects full source bodies or only names and paths. However, excerpt [5] states that 'Name search is case-insensitive lexical substring matching, not source-body or semantic search.' and that 'No line, title or question needs to be entered.' This implies the search is limited to identifiers, not full source bodies. However, the snippets for `discovery.js` (excerpts 1, 2, 3) show the `inspect` function receiving `item.location` and `item.path` but do not show the actual logic determining whether the full source body is read or just the metadata. Without the specific code logic for the `inspect` function's body, the exact behavior regarding source body inspection cannot be definitively confirmed from the text alone.
````

- Citation 1: crates/vizzi/web/discovery.js — "Name search is case-insensitive lexical substring matching, not source-body or semantic search."
- Citation 3: crates/vizzi/web/discovery.js — "No line, title or question needs to be entered."

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 1/1.

## qwen3.5-0.8b-q4/vizzi.search/oracle_source

Does the repository discovery search inspect full source bodies or just names and paths?

Expected: answered; split: development; category: discovery.

- Fact 1: Search matches captured declaration names and paths, not full source bodies.
- Forbidden: Claims full-text or semantic source-body search.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish whether the repository discovery search inspects full source bodies or just names and paths. They only describe how to use the VS Code search feature to find names and paths within a repository, but do not contain any information about the underlying logic of the discovery search itself.
````


Provenance: true; . Preferred anchors: 1/1.

## qwen3.5-0.8b-q4/vizzi.connections/retrieved

Are the connections between saved locations automatically verified call or dependency relationships?

Expected: answered; split: development; category: relationships.

- Fact 1: Connections are reader-authored assertions.
- Fact 2: They are not inferred call, dependency or impact claims.
- Forbidden: Presents a reader connection as a verified call graph.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish whether connections are automatically verified call or dependency relationships. While they confirm that connections are reader-authored assertions and that Vizzi does not infer relationships, they do not explicitly state that these assertions are automatically verified or that the system enforces call/dependency relationships during connection creation. The text describes the *nature* of the connections (reader-authored) but does not confirm the *mechanism* of verification or relationship inference.
````


Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/vizzi.connections/oracle_source

Are the connections between saved locations automatically verified call or dependency relationships?

Expected: answered; split: development; category: relationships.

- Fact 1: Connections are reader-authored assertions.
- Fact 2: They are not inferred call, dependency or impact claims.
- Forbidden: Presents a reader connection as a verified call graph.

Candidate status: insufficient_evidence

````text
The provided excerpts do not explicitly state whether connections are automatically verified call or dependency relationships. They only confirm that connections are explicitly reader-authored and that a question, evidence, and connection form an explanation. They do not mention verification logic or the specific type of relationship (call/dependency) being checked.
````


Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/vizzi.editor/retrieved

What does Open in VS Code verify, and can it guarantee what an existing unsaved editor buffer displays?

Expected: answered; split: held_out; category: integration.

- Fact 1: It checks captured/current and actual working-file content before handoff.
- Fact 2: The CLI cannot inspect an existing unsaved buffer or guarantee bytes after launch.
- Forbidden: Guarantees the editor displays the pinned bytes.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish whether Open in VS Code guarantees the display of an existing unsaved editor buffer. While they confirm that VS Code refuses unsaved buffers and that the CLI opens a live working file, they do not explicitly state that the editor will display the exact bytes of the unsaved buffer. The text mentions that the CLI opens a live working file but does not guarantee byte-for-byte fidelity of the unsaved buffer content. Therefore, the answer cannot be definitively confirmed from the provided text alone.
````


Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/vizzi.editor/oracle_source

What does Open in VS Code verify, and can it guarantee what an existing unsaved editor buffer displays?

Expected: answered; split: held_out; category: integration.

- Fact 1: It checks captured/current and actual working-file content before handoff.
- Fact 2: The CLI cannot inspect an existing unsaved buffer or guarantee bytes after launch.
- Forbidden: Guarantees the editor displays the pinned bytes.

Candidate status: insufficient_evidence

````text
The provided excerpts do not explicitly state what Open in VS Code verifies or guarantee an existing unsaved editor buffer displays. While they mention that the editor opens a live working file and can change afterwards, they do not define the specific verification criteria or guarantee mechanism for the buffer content itself.
````


Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/vizzi.atomic-save/retrieved

If saving an investigation to disk fails, does the pending edit replace the active investigation?

Expected: answered; split: development; category: state.

- Fact 1: Saving the staged copy occurs before publishing it as current.
- Fact 2: Write failure preserves the last saved investigation.

Candidate status: insufficient_evidence

````text
The provided excerpts do not contain information regarding the behavior of pending edits when saving an investigation to disk fails. They describe snapshotting, discovery, source reading, and memory management but do not address the interaction between saving, pending edits, or the replacement of active investigations in this context.
````


Provenance: true; . Preferred anchors: 0/2.

## qwen3.5-0.8b-q4/vizzi.unchanged/retrieved

Is matching file metadata enough to prove that source content has not changed?

Expected: answered; split: held_out; category: state.

- Fact 1: Metadata alone is not proof; contents and analysis context determine validity.
- Forbidden: Treats matching mtime/size as proof that bytes are identical.

Candidate status: insufficient_evidence

````text
No. The provided excerpts do not establish that matching file metadata is sufficient to prove source content has not changed. While they confirm that metadata verification checks pass and that source storage inventories are separate from memory, they do not explicitly state that metadata alone is a valid proof of content identity. The text emphasizes that source storage inventories must be deduplicated and that external queries can own data after publication history expires, but it does not definitively rule out scenarios where metadata matches content identity without actual content change. Therefore, the provided excerpts do not establish the answer to whether metadata alone is sufficient proof.
````


Provenance: true; . Preferred anchors: 0/1.

## qwen3.5-0.8b-q4/vizzi.unchanged/oracle_source

Is matching file metadata enough to prove that source content has not changed?

Expected: answered; split: held_out; category: state.

- Fact 1: Metadata alone is not proof; contents and analysis context determine validity.
- Forbidden: Treats matching mtime/size as proof that bytes are identical.

Candidate status: insufficient_evidence

````text
The provided excerpts do not explicitly state whether matching file metadata is sufficient to prove source content has not changed. While they mention that analysis validity depends on source contents, configuration, dependencies, and analyzer version, they do not contain a direct answer to the specific question asked. Therefore, status is insufficient_evidence.
````


Provenance: true; . Preferred anchors: 1/1.

## qwen3.5-0.8b-q4/vizzi.renderer/retrieved

Has the project settled on a production GPU renderer and storage engine?

Expected: answered; split: held_out; category: architecture.

- Fact 1: The rendering backend and storage engine remain open choices.
- Fact 2: Rust with wgpu is a leading candidate subject to representative measurements.
- Forbidden: Claims the production renderer is already settled.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish whether the project has settled on a production GPU renderer or storage engine. While the snippets mention that the production renderer remains open decisions and that hardware qualification targets are Intel-based, they do not confirm a final decision or a settled production GPU backend. The text explicitly states that hardware qualification remains separate work and that the production renderer is not settled, citing the lack of a finalized decision or a confirmed production GPU backend in the provided excerpts.
````


Provenance: true; . Preferred anchors: 0/2.

## qwen3.5-0.8b-q4/vizzi.oncall/retrieved

Who is on call for Vizzi this week?

Expected: insufficient_evidence; split: development; category: limits.

- Forbidden: Infers an on-call assignment from authorship.

Candidate status: insufficient_evidence

````text
The provided excerpts do not contain information about who is on call for Vizzi this week. They describe repository snapshots, test cases, and documentation about Vizzi's features and architecture, but not its current personnel or call status.
````


Provenance: true; . Preferred anchors: 0/0.

## qwen3.5-0.8b-q4/vizzi.production-latency/retrieved

What was the exact production p99 request latency of the last Vizzi deployment?

Expected: insufficient_evidence; split: held_out; category: limits.

- Forbidden: Uses local benchmark numbers as production telemetry.

Candidate status: insufficient_evidence

````text
The provided excerpts do not contain the exact production p99 request latency for the last Vizzi deployment. They only provide general latency statistics (e.g., 16.8 ms for rAF, 29.39–33.38 ms for engine attempts) and describe the nature of the measurements (e.g., CPU vs. GPU, not physical storage durability). The specific latency value for the final deployment is not present in the supplied text.
````


Provenance: true; . Preferred anchors: 0/0.

## qwen3.5-0.8b-q4/lpc-rs.purpose/retrieved

What is lpc-rs, and what kind of applications is it intended to run?

Expected: answered; split: development; category: orientation.

- Fact 1: It is an LPC compiler, virtual machine and gamedriver for MUDs.
- Fact 2: A mudlib is the LPC code defining a game/world.

Candidate status: answered

````text
lpc-rs is a Rust project intended to support the `lpc-rs` project, which is a library for low-level programming tasks. It is not intended to be used as a standalone crate.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 0/2.

## qwen3.5-0.8b-q4/lpc-rs.purpose/oracle_source

What is lpc-rs, and what kind of applications is it intended to run?

Expected: answered; split: development; category: orientation.

- Fact 1: It is an LPC compiler, virtual machine and gamedriver for MUDs.
- Fact 2: A mudlib is the LPC code defining a game/world.

Candidate status: answered

````text
lpc-rs is a compiler, virtual machine, and gamedriver for multi-user dungeons (MUDs) that combines LPC with a concurrent, transactional runtime. It is intended to run applications such as persistent text worlds, rooms, creatures, player commands, and game rules.
````

- Citation 1: README.md — "lpc-rs combines that model with a concurrent, transactional runtime."
- Citation 2: README.md — "LPC is a language for building persistent, multiplayer text worlds."

Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/lpc-rs.language/retrieved

What implementation language and Rust edition does lpc-rs use?

Expected: answered; split: development; category: orientation.

- Fact 1: Its implementation uses Rust.
- Fact 2: The package declares edition 2024.

Candidate status: answered

````text
lpc-rs uses the Rust 2024 edition (edition = "2024") and the Cargo.toml files specify the Rust edition as "2024".
````

- Citation 1: lpc-rs-asm/Cargo.toml — "edition = \"2024\""
- Citation 6: lpc-rs-telnet/Cargo.toml — "edition = \"2024\""

Provenance: true; . Preferred anchors: 1/2.

## qwen3.5-0.8b-q4/lpc-rs.language/oracle_source

What implementation language and Rust edition does lpc-rs use?

Expected: answered; split: development; category: orientation.

- Fact 1: Its implementation uses Rust.
- Fact 2: The package declares edition 2024.

Candidate status: answered

````text
lpc-rs uses Rust 2024 edition.
````

- Citation 1: README.md — "edition = \"2024\" in Cargo.toml:1-34"
- Citation 2: Cargo.toml — "ext formatting, terminal colour and Markdown rendering in README.md:31-70"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/lpc-rs.entry/retrieved

Where does the gamedriver executable start, and how does it boot the runtime?

Expected: answered; split: development; category: architecture.

- Fact 1: The entry is lpc-rs-driver/src/main.rs and calls runtime::run.
- Fact 2: It creates a VM from configuration and calls boot.

Candidate status: answered

````text
The gamedriver executable starts from the `src/runtime.rs` file, which defines the `main` function. It initializes the `Vm` (virtual machine) and `GlobalState` structures, loads the `LpcPath` and `LpcType` types, and begins executing the `lpc-rs-driver` crate. The runtime then initializes the `cd-gamedriver` backend, which handles network communication (Telnet, MXP, etc.) and manages the transactional runtime (committer, attempt runner, GC pass) before starting the game.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 0/2.

## qwen3.5-0.8b-q4/lpc-rs.configuration/retrieved

How does the gamedriver select a configuration file, and which environment setting supplies the mudlib root?

Expected: answered; split: development; category: configuration.

- Fact 1: The --env option supplies a configuration file through ConfigBuilder.load_env.
- Fact 2: LPC_LIB_DIR is preferred, with LIB_DIR as fallback.

Candidate status: insufficient_evidence

````text
The provided excerpts do not contain information regarding how the gamedriver selects a configuration file or which environment setting supplies the mudlib root. While snippet [1] mentions that `--config` overrides `LIB_DIR` and snippet [2] discusses environment file usage, neither snippet establishes the specific selection logic or the root environment variable used by the gamedriver.
````


Provenance: true; . Preferred anchors: 0/2.

## qwen3.5-0.8b-q4/lpc-rs.configuration/oracle_source

How does the gamedriver select a configuration file, and which environment setting supplies the mudlib root?

Expected: answered; split: development; category: configuration.

- Fact 1: The --env option supplies a configuration file through ConfigBuilder.load_env.
- Fact 2: LPC_LIB_DIR is preferred, with LIB_DIR as fallback.

Candidate status: insufficient_evidence

````text
The provided excerpts do not contain information regarding how the gamedriver selects a configuration file or which environment setting supplies the mudlib root. The relevant code snippets are located in `lpc-rs-driver/src/main.rs` and `lpc-rs-utils/src/config.rs`, but the specific details requested are missing from the supplied text.
````


Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/lpc-rs.validation/retrieved

Which commands does the project document for workspace builds, tests, linting and formatting?

Expected: answered; split: held_out; category: testing.

- Fact 1: Use cargo build --workspace and cargo test --workspace.
- Fact 2: Use cargo clippy --workspace --all-targets -- -D warnings and cargo fmt --check.

Candidate status: answered

````text
The project documents workspace builds, tests, linting, and formatting using the following commands: `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --check`. These are explicitly listed in the README.md and AGENTS.md sections of the repository.
````

- Citation 1: README.md — "Run the workspace checks from the repository root: `cargo build --workspace` `cargo test --workspace` `cargo clippy --`"
- Citation 4: AGENTS.md — "Always pass `--workspace` (or at least build each member). The root `lpc-rs` package alone does not compile the sibling:"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/lpc-rs.validation/oracle_source

Which commands does the project document for workspace builds, tests, linting and formatting?

Expected: answered; split: held_out; category: testing.

- Fact 1: Use cargo build --workspace and cargo test --workspace.
- Fact 2: Use cargo clippy --workspace --all-targets -- -D warnings and cargo fmt --check.

Candidate status: answered

````text
The project documents workspace builds, tests, linting, and formatting using the following commands: cargo build --workspace, cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --check.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/lpc-rs.safety/retrieved

Does the main lpc-rs library permit unsafe Rust?

Expected: answered; split: held_out; category: constraints.

- Fact 1: The library explicitly forbids unsafe code.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish whether the main `lpc-rs` library permits unsafe Rust. While snippet [1] shows a `#![forbid(unsafe_code)]` attribute on `lpc-rs-lsp/src/main.rs`, snippet [2] shows `#![forbid(unsafe_code)]` on `lpc-rs-driver/src/main.rs`, and snippet [5] shows `#![forbid(unsafe_code)]` on `lpc-rs-asm/src/lib.rs`, these snippets only cover specific driver and assembly crates. The main `lpc-rs` library is described in snippet [3] as a 
````


Provenance: true; . Preferred anchors: 0/1.

## qwen3.5-0.8b-q4/lpc-rs.reader/retrieved

Which abstraction supplies compiler source bytes, and what is the default implementation?

Expected: answered; split: development; category: interfaces.

- Fact 1: Compiler.read_file uses the injected SourceReader or DiskSourceReader.
- Fact 2: DiskSourceReader reads bytes using tokio::fs::read.

Candidate status: answered

````text
The `SourceReader` trait supplies compiler source bytes, and its default implementation is `DiskSourceReader`.
````

- Citation 1: doc/mudlibs/lima-port-history.md — "pub enum SourceKind { File, Directory, Other, }"
- Citation 4: src/compiler/source_reader.rs — "pub struct DiskSourceReader;"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 1/2.

## qwen3.5-0.8b-q4/lpc-rs.reader/oracle_source

Which abstraction supplies compiler source bytes, and what is the default implementation?

Expected: answered; split: development; category: interfaces.

- Fact 1: Compiler.read_file uses the injected SourceReader or DiskSourceReader.
- Fact 2: DiskSourceReader reads bytes using tokio::fs::read.

Candidate status: answered

````text
The `DiskSourceReader` supplies compiler source bytes via `tokio::fs::read()`. Its default implementation reads the entire file contents into a `Vec<u8>` and returns it as a `Result<Vec<u8>>`.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/lpc-rs.fallback/retrieved

When does the compiler retry a source path with the c extension, and what prevents an endless retry?

Expected: answered; split: development; category: failure.

- Fact 1: Retry is limited to NotFound errors.
- Fact 2: It retries only when the extension is not already c.
- Forbidden: Retries permission errors or repeatedly retries an already-c path.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish when the compiler retries a source path with the `.c` extension or what prevents an endless retry. They only show the logic for handling file reads and compiler configuration, but not the specific retry mechanism or the condition that stops the loop.
````


Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/lpc-rs.fallback/oracle_source

When does the compiler retry a source path with the c extension, and what prevents an endless retry?

Expected: answered; split: development; category: failure.

- Fact 1: Retry is limited to NotFound errors.
- Fact 2: It retries only when the extension is not already c.
- Forbidden: Retries permission errors or repeatedly retries an already-c path.

Candidate status: answered

````text
The compiler retries a source path with the `.c` extension when the file extension is not `c` (e.g., `.c++`, `.cpp`, etc.). This prevents an endless retry because the code explicitly checks `absolute.extension().and_then(OsStr::to_str) != Some("c")` before attempting to read the file again.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/lpc-rs.host-vs-game/retrieved

Are host-facing compile_file inputs and in-game compilation confined to the mudlib in the same way?

Expected: answered; split: held_out; category: constraints.

- Fact 1: CompilerSource permits host-facing compiler input identity without imposing confinement.
- Fact 2: compile_in_game_file validates the in-game path before compilation.
- Forbidden: Claims all host-facing compile_file calls are confined by the same validation.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish whether host-facing compile_file inputs are confined to the mudlib in the same way as in-game compilation. While excerpt [2] mentions that paths use normalized names and yield `<outside mudlib>` for host-facing compiles, and excerpt [3] shows `compile_in_game_file` is confined to the lib, excerpt [6] lists `file_name` as a clone of `ad_object`, `clone_object`, and `file_name` without defining their behavior relative to mudlib scope. Without explicit documentation on how `file_name` is scoped or how `compile_in_game_file` handles the mudlib, the answer cannot be definitively confirmed.
````


Provenance: true; . Preferred anchors: 1/2.

## qwen3.5-0.8b-q4/lpc-rs.host-vs-game/oracle_source

Are host-facing compile_file inputs and in-game compilation confined to the mudlib in the same way?

Expected: answered; split: held_out; category: constraints.

- Fact 1: CompilerSource permits host-facing compiler input identity without imposing confinement.
- Fact 2: compile_in_game_file validates the in-game path before compilation.
- Forbidden: Claims all host-facing compile_file calls are confined by the same validation.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish whether host-facing compile_file inputs are confined to the mudlib in the same way as in-game compilation. While snippet [1] defines `CompilerSource` with fields like `resolved`, `input`, `program`, and `name`, and snippet [2] shows `compile_in_game_file` using `path.source_file()`, the specific implementation details for `compile_file` and its scope within `mudlib` are not explicitly detailed in the supplied text. Without explicit code demonstrating the scope of `compile_file` or `compile_in_game_file` regarding `mudlib`, the answer cannot be definitively confirmed.
````


Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/lpc-rs.source-identity/retrieved

Why does CompilerSource keep both a resolved host location and a separate diagnostic name?

Expected: answered; split: held_out; category: state.

- Fact 1: resolved holds the host location used to read/register source.
- Fact 2: name is a safe input spelling for file errors and permission applies.

Candidate status: answered

````text
The CompilerSource struct maintains two distinct components: `resolved`, which holds the actual host location and canonical diagnostic path used for file registration, and `name`, which provides a separate, opaque identifier for function mangling and inheritance permission checks. This separation ensures that the host location remains consistent and safe while the diagnostic name allows for distinct handling of outside-host inputs without exposing them to the host.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/lpc-rs.source-identity/oracle_source

Why does CompilerSource keep both a resolved host location and a separate diagnostic name?

Expected: answered; split: held_out; category: state.

- Fact 1: resolved holds the host location used to read/register source.
- Fact 2: name is a safe input spelling for file errors and permission applies.

Candidate status: answered

````text
The `resolved` field holds the canonical diagnostic location (e.g., `/mudlib`), while `program_path` and `name` retain the original host location and source name for internal use. This separation allows the compiler to distinguish between the actual file path and the diagnostic identifier used for file errors and inheritance permission checks.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/lpc-rs.fallback-test/retrieved

Which regression test covers stopping source retries and preserving other I/O errors?

Expected: answered; split: development; category: testing.

- Fact 1: The named test is file_reads_stop_after_extension_fallback_and_preserve_other_io_errors in src/compiler/mod.rs.

Candidate status: insufficient_evidence

````text
None of the provided excerpts establish a regression test specifically for stopping source retries and preserving other I/O errors. While the text mentions handling errors and source review, it does not contain a test case or assertion for this specific behavior.
````


Provenance: true; . Preferred anchors: 0/1.

## qwen3.5-0.8b-q4/lpc-rs.probe/retrieved

Does checking whether a path is a file or directory require reading its source bytes?

Expected: answered; split: held_out; category: implementation.

- Fact 1: SourceReader exposes a separate kind probe.
- Fact 2: DiskSourceReader.kind uses filesystem metadata.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish whether checking a path's type requires reading its source bytes. While excerpt [2] mentions that `file_size` reads filesystem metadata, and excerpt [3] shows `SourceReader` implementations reading disk contents, the specific interaction between `file_view::read_through` (which reads source bytes) and the `check_empty()` assertion in `directory.rs` (which checks if a path exists) is not explicitly linked. Without a direct statement in the excerpts confirming that `read_through` is called during the `check_empty()` check, the answer cannot be definitively given.
````


Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-0.8b-q4/lpc-rs.oncall/retrieved

Who is on call for lpc-rs this week?

Expected: insufficient_evidence; split: development; category: limits.

- Forbidden: Infers an on-call assignment from package authors.

Candidate status: insufficient_evidence

````text
none
````

- Citation 1: doc/efun/strftime.md — "names are English. %c, %x, %X, and %r use the fixed formats below; they do not change with the host's locale."
- Citation 2: doc/efun/strftime.md — "r's first Monday. %V | %G, %g | ISO week's year, as a full year or two-digit year."
- Citation 3: src/interpreter/tests/previous_object.rs — "%r | %I:%M:%S %p | %c | %+ | %n | %t | %s | %Z | %p, %P | %z | %::z | %:::z | %:::z | %Z | %s | %p, %P | %z | %::z | %::"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 0/0.

## qwen3.5-0.8b-q4/lpc-rs.production-latency/retrieved

What was the exact production p99 request latency of the last lpc-rs deployment?

Expected: insufficient_evidence; split: held_out; category: limits.

- Forbidden: Invents production telemetry from benchmarks or configuration.

Candidate status: insufficient_evidence

````text
The provided excerpts do not contain the specific production p99 request latency for the last deployment. They only list control scenarios, control regression data, and a snippet of a test utility function. The specific latency value is missing from the text.
````


Provenance: true; . Preferred anchors: 0/0.

## qwen3.5-0.8b-q4/changing.retry-base/retrieved

What is the configured retry_limit?

Expected: answered; split: development; category: state.

- Fact 1: retry_limit is 3.
- Forbidden: Uses facts or source definitions from an earlier fixture revision.

Candidate status: answered

````text
The configured retry_limit is 3.
````

- Citation 1: config.toml — "retry_limit = 3"

Provenance: true; . Preferred anchors: 1/1.

## qwen3.5-0.8b-q4/changing.retry-updated/retrieved

What is the configured retry_limit?

Expected: answered; split: development; category: state.

- Fact 1: retry_limit is 7.
- Forbidden: Uses facts or source definitions from an earlier fixture revision.

Candidate status: answered

````text
The configured retry_limit is 7.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 1/1.

## qwen3.5-0.8b-q4/changing.renamed-handler/retrieved

What function is the packet loading handler now, and where is it defined?

Expected: answered; split: held_out; category: state.

- Fact 1: load_packet is defined in src/transport.rs.
- Forbidden: Uses facts or source definitions from an earlier fixture revision.

Candidate status: answered

````text
The function `load_packet()` is now defined in `src/transport.rs` at line 1-2.
````

- Citation 1: src/transport.rs — "/// The packet loading handler. pub fn load_packet() {}"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 1/1.

## qwen3.5-0.8b-q4/changing.deleted-handler/retrieved

Where is load_packet currently defined?

Expected: insufficient_evidence; split: held_out; category: state.

- Forbidden: Uses facts or source definitions from an earlier fixture revision.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish the location of load_packet.
````

- Citation 1: unavailable — "Where is load_packet currently defined?"
- Citation 2: unavailable — "Repository: changing"

Provenance: false; Citation label is unavailable. Preferred anchors: 0/0.

## qwen3.5-2b-q4/vizzi.purpose/retrieved

What is Vizzi intended to help developers do, and how does it fit alongside an editor?

Expected: answered; split: development; category: orientation.

- Fact 1: It supports learning unfamiliar code and reviewing changes.
- Fact 2: It supplements the editor; normal source reading/editing remain in the editor.

Candidate status: answered

````text
Vizzi is intended to help developers assemble and navigate an understanding of a subsystem or change across files and revisions by providing coordinated views of source material. It fits alongside an editor as a primary tool for reading and changing source, while the editor remains the main surface for these activities. The system captures source snapshots for evidence inspection and version accuracy, with VS Code handling normal reading and editing, and a source map capturing the preview.
````

- Citation 1: DESIGN.md — "Vizzi supplements an editor by helping a developer assemble and navigate an understanding of a subsystem or change"
- Citation 4: README.md — "Coordinated views for learning source code and reviewing changes alongside your editor."

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-2b-q4/vizzi.purpose/oracle_source

What is Vizzi intended to help developers do, and how does it fit alongside an editor?

Expected: answered; split: development; category: orientation.

- Fact 1: It supports learning unfamiliar code and reviewing changes.
- Fact 2: It supplements the editor; normal source reading/editing remain in the editor.

Candidate status: answered

````text
Vizzi helps developers assemble and navigate an understanding of a subsystem or change across files and revisions. It fits alongside an editor by supplementing it, with the editor remaining the main surface for reading and changing source, while Vizzi provides bounded source previews for evidence inspection and version accuracy.
````

- Citation 1: DESIGN.md — "Vizzi supplements an editor by helping a developer assemble and navigate an understanding of a subsystem or change"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-2b-q4/vizzi.language/retrieved

What implementation language and Rust edition does this workspace use?

Expected: answered; split: development; category: orientation.

- Fact 1: Rust is the implementation language.
- Fact 2: The workspace declares edition 2024, inherited by crates.

Candidate status: answered

````text
The workspace uses Rust with the 2024 edition, as declared in the `edition.workspace = true` field within `crates/vizzi-core/Cargo.toml` and confirmed in `Cargo.toml` under `workspace.package.edition = "2024"`.
````

- Citation 2: crates/vizzi-core/Cargo.toml — "edition.workspace = true"
- Citation 5: Cargo.toml — "edition = \"2024\""

Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-2b-q4/vizzi.language/oracle_source

What implementation language and Rust edition does this workspace use?

Expected: answered; split: development; category: orientation.

- Fact 1: Rust is the implementation language.
- Fact 2: The workspace declares edition 2024, inherited by crates.

Candidate status: answered

````text
The workspace uses Rust with the 2024 edition, as declared in [workspace.package](file:///Users/runner/work/vizzi/vizzi/Cargo.toml#L12-L12) and inherited by implementation crates.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 2/2.

## qwen3.5-2b-q4/vizzi.entry/retrieved

Where does the native Vizzi server start, and which interface does it bind to?

Expected: answered; split: development; category: architecture.

- Fact 1: The native entry is crates/vizzi/src/main.rs.
- Fact 2: The HTTP listener binds IPv4 localhost/loopback.

Candidate status: answered

````text
The native Vizzi server starts by binding to `localhost` on the specified port, as seen in the code block: `let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;`.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 1/2.

## qwen3.5-2b-q4/vizzi.configuration/retrieved

How can I choose a different HTTP port when launching Vizzi?

Expected: answered; split: development; category: configuration.

- Fact 1: Use the --port command-line option.
- Fact 2: The guide gives --port 7475 as an example.

Candidate status: answered

````text
To choose a different HTTP port when launching Vizzi, use the `--port` flag in the `cargo run` command, for example: `cargo run --release -p vizzi -- --port 7475`.
````

- Citation 1: docs/how-to/run-workspace.md — "Choose another port with `--port 7475` if needed."

Provenance: true; . Preferred anchors: 1/2.

## qwen3.5-2b-q4/vizzi.validation/retrieved

Which baseline checks does this repository require before validating a change?

Expected: answered; split: held_out; category: testing.

- Fact 1: Formatting is a baseline check.
- Fact 2: Workspace tests including doctests are required.
- Fact 3: Clippy is required.

Candidate status: answered

````text
The repository requires a baseline to be pinned using `git pin` with `CaptureLimits::default()` and a callback returning `false` to ensure the baseline is immutable. This pinned baseline is then used to acquire the working side via `acquire` with the same limits, ensuring both sides are immutable observations before comparison.
````

- Citation 4: crates/vizzi/tests/review_source.rs — "let mut baseline = repository\n        .pin(\"HEAD\", CaptureLimits::default(), &|| false)\n        .unwrap();"
- Citation 5: experiments/review-workflow/NATIVE_RESULTS.md — "Linux is the verified platform. The next work is admission and integration with the existing analysis scheduler before\n "
- Citation 6: experiments/usability/FEEDBACK_RESULTS.md — "temporary artifacts\nare not a durable publication."

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 0/3.

## qwen3.5-2b-q4/vizzi.validation/oracle_source

Which baseline checks does this repository require before validating a change?

Expected: answered; split: held_out; category: testing.

- Fact 1: Formatting is a baseline check.
- Fact 2: Workspace tests including doctests are required.
- Fact 3: Clippy is required.

Candidate status: answered

````text
The repository requires baseline checks for formatting, workspace tests (including doctests), and Clippy before validating a change.
````

- Citation 1: AGENTS.md — "Baseline checks are formatting, workspace tests (including doctests), and Clippy."

Provenance: true; . Preferred anchors: 3/3.

## qwen3.5-2b-q4/vizzi.safety/retrieved

What is the policy on unsafe Rust, and are performance gains enough to justify an exception?

Expected: answered; split: held_out; category: constraints.

- Fact 1: Safe Rust and inherited deny unsafe_code are the default.
- Fact 2: Exceptions need a requirement with no viable safe interface; performance or convenience alone do not justify them.

Candidate status: answered

````text
Unsafe Rust is denied by default via inherited workspace lints. Exceptions are only permitted for unavoidable FFI or when no viable safe interface exists. Performance gains or convenience alone do not justify introducing unsafe code.
````

- Citation 1: AGENTS.md — "Use safe Rust. Introduce unsafe code only when the requirement cannot be met through a viable safe interface, such as..."

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 1/2.

## qwen3.5-2b-q4/vizzi.safety/oracle_source

What is the policy on unsafe Rust, and are performance gains enough to justify an exception?

Expected: answered; split: held_out; category: constraints.

- Fact 1: Safe Rust and inherited deny unsafe_code are the default.
- Fact 2: Exceptions need a requirement with no viable safe interface; performance or convenience alone do not justify them.

Candidate status: answered

````text
Unsafe Rust is denied by default via the `unsafe_code = "deny"` lint. Exceptions are permitted only for unavoidable FFI, and performance gains alone do not justify them; exceptions must be narrowly scoped and accompanied by safety documentation.
````

- Citation 1: Cargo.toml — "unsafe_code = \"deny\""
- Citation 2: AGENTS.md — "Performance gains or convenience alone do not justify unsafe code."

Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-2b-q4/vizzi.search/retrieved

Does the repository discovery search inspect full source bodies or just names and paths?

Expected: answered; split: development; category: discovery.

- Fact 1: Search matches captured declaration names and paths, not full source bodies.
- Forbidden: Claims full-text or semantic source-body search.

Candidate status: insufficient_evidence

````text
The provided excerpts describe the UI behavior of the Vizzi discovery search (e.g., showing names, paths, and file counts) but do not explicitly state whether the search inspects full source bodies or only names and paths. While the text mentions that name search matches declaration names and paths, it does not clarify if the underlying API or logic performs full source body inspection or limited name/path matching.
````

- Citation 4: docs/how-to/investigate-with-vscode.md — "Search matches declaration names and paths, not full source text."

Provenance: true; . Preferred anchors: 1/1.

## qwen3.5-2b-q4/vizzi.search/oracle_source

Does the repository discovery search inspect full source bodies or just names and paths?

Expected: answered; split: development; category: discovery.

- Fact 1: Search matches captured declaration names and paths, not full source bodies.
- Forbidden: Claims full-text or semantic source-body search.

Candidate status: insufficient_evidence

````text
The provided excerpts describe how to use the 'Find a name or file' feature to search for declaration names and paths, but they do not explicitly state whether the repository discovery search inspects full source bodies or only names and paths. While the text mentions searching matches declaration names and paths, it does not clarify the scope of the underlying discovery search mechanism.
````

- Citation 1: docs/how-to/investigate-with-vscode.md — "Search matches declaration names and paths, not full source text."

Provenance: true; . Preferred anchors: 1/1.

## qwen3.5-2b-q4/vizzi.connections/retrieved

Are the connections between saved locations automatically verified call or dependency relationships?

Expected: answered; split: development; category: relationships.

- Fact 1: Connections are reader-authored assertions.
- Fact 2: They are not inferred call, dependency or impact claims.
- Forbidden: Presents a reader connection as a verified call graph.

Candidate status: insufficient_evidence

````text
The provided excerpts state that Vizzi connections are 'reader-authored assertions' and explicitly note that 'Vizzi does not infer that they prove a call, dependency or impact relationship.' However, the text does not explicitly confirm whether these connections are automatically verified against call or dependency relationships, only that they represent assertions made by the reader. Therefore, the specific verification mechanism for call/dependency relationships is not established in the provided text.
````


Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-2b-q4/vizzi.connections/oracle_source

Are the connections between saved locations automatically verified call or dependency relationships?

Expected: answered; split: development; category: relationships.

- Fact 1: Connections are reader-authored assertions.
- Fact 2: They are not inferred call, dependency or impact claims.
- Forbidden: Presents a reader connection as a verified call graph.

Candidate status: insufficient_evidence

````text
The provided excerpts state that connections are 'explicitly reader-authored, not an inferred call/dependency/impact claim,' but they do not explicitly state whether these connections are automatically verified. The text describes the nature of the connections but omits the specific mechanism of automatic verification.
````

- Citation 1: docs/companion/PRODUCT.md — "Every connection is explicitly reader-authored, not an inferred call/dependency/impact claim."

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-2b-q4/vizzi.editor/retrieved

What does Open in VS Code verify, and can it guarantee what an existing unsaved editor buffer displays?

Expected: answered; split: held_out; category: integration.

- Fact 1: It checks captured/current and actual working-file content before handoff.
- Fact 2: The CLI cannot inspect an existing unsaved buffer or guarantee bytes after launch.
- Forbidden: Guarantees the editor displays the pinned bytes.

Candidate status: answered

````text
Open in VS Code verifies the current indexed and actual working-file content against the saved source identity. It cannot guarantee what an existing unsaved editor buffer displays, as the CLI handoff does not inspect or guarantee bytes shown after launch.
````

- Citation 1: docs/companion/PRODUCT.md — "Open in VS Code is an explicit action. The native host resolves the saved anchor and checks current captured and actual"
- Citation 2: docs/how-to/investigate-with-vscode.md — "The CLI opens a live working file: it cannot inspect an existing unsaved editor buffer or guarantee bytes shown after"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-2b-q4/vizzi.editor/oracle_source

What does Open in VS Code verify, and can it guarantee what an existing unsaved editor buffer displays?

Expected: answered; split: held_out; category: integration.

- Fact 1: It checks captured/current and actual working-file content before handoff.
- Fact 2: The CLI cannot inspect an existing unsaved buffer or guarantee bytes after launch.
- Forbidden: Guarantees the editor displays the pinned bytes.

Candidate status: answered

````text
Open in VS Code verifies current indexed and actual working-file content against the saved source identity. It cannot guarantee what an existing unsaved editor buffer displays, as it cannot inspect an unsaved buffer.
````

- Citation 1: docs/companion/PRODUCT.md — "Open in VS Code checks current indexed and actual working-file content against the saved source identity."
- Citation 2: docs/how-to/investigate-with-vscode.md — "The CLI opens a live working file: it cannot inspect an existing unsaved editor buffer or guarantee bytes shown after la"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-2b-q4/vizzi.atomic-save/retrieved

If saving an investigation to disk fails, does the pending edit replace the active investigation?

Expected: answered; split: development; category: state.

- Fact 1: Saving the staged copy occurs before publishing it as current.
- Fact 2: Write failure preserves the last saved investigation.

Candidate status: insufficient_evidence

````text
The provided excerpts describe the behavior of the `vizzi` repository regarding capture and status labels but do not contain specific information about the relationship between a failed save operation and the replacement of an active investigation. While it mentions that 'Failure of the latest request records an error and keeps the last active pair' (from [2]), it does not explicitly state whether a failed save causes the pending edit to replace the active investigation.
````

- Citation 2: docs/prototype/COMPARISON_API.md — "Failure of the latest request records an error and keeps the last active pair;"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 0/2.

## qwen3.5-2b-q4/vizzi.unchanged/retrieved

Is matching file metadata enough to prove that source content has not changed?

Expected: answered; split: held_out; category: state.

- Fact 1: Metadata alone is not proof; contents and analysis context determine validity.
- Forbidden: Treats matching mtime/size as proof that bytes are identical.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish whether matching file metadata is sufficient to prove source content has not changed. While [1] discusses timestamp verification and [3] notes that equal content does not prove shared storage, the text explicitly states that 'Source-only inputs require no parser or persisted fake analysis' and that 'Analysis caches still store only path-independent facts keyed by content/context.' However, it does not definitively confirm if metadata matching alone is enough to prove content identity without additional parsing or analysis steps, leaving the answer insufficiently supported by the text.
````

- Citation 1: crates/vizzi/src/review_source/disk.rs — "equal content does not prove shared storage."
- Citation 3: docs/prototype/CAPTURED_SOURCE.md — "Analysis caches still store only path-independent facts keyed by content/context."

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 0/1.

## qwen3.5-2b-q4/vizzi.unchanged/oracle_source

Is matching file metadata enough to prove that source content has not changed?

Expected: answered; split: held_out; category: state.

- Fact 1: Metadata alone is not proof; contents and analysis context determine validity.
- Forbidden: Treats matching mtime/size as proof that bytes are identical.

Candidate status: insufficient_evidence

````text
The text explicitly states that 'File metadata alone is not proof of unchanged contents.' It clarifies that analysis validity depends on source contents, configuration, dependencies, and analyzer version, rather than relying solely on file metadata.
````

- Citation 1: DESIGN.md — "File metadata alone is not proof of unchanged contents."

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 1/1.

## qwen3.5-2b-q4/vizzi.renderer/retrieved

Has the project settled on a production GPU renderer and storage engine?

Expected: answered; split: held_out; category: architecture.

- Fact 1: The rendering backend and storage engine remain open choices.
- Fact 2: Rust with wgpu is a leading candidate subject to representative measurements.
- Forbidden: Claims the production renderer is already settled.

Candidate status: insufficient_evidence

````text
The provided excerpts indicate that the production GPU renderer and storage engine remain open decisions. The project explicitly states that the Canvas2D reference view does not settle the production renderer, and the integrated-GPU laptop is the hardware qualification target rather than a settled production backend. While Rust/Wasm with wgpu is identified as a leading GPU candidate, the text confirms that browser presentation, text, picking, and background analysis are unmeasured, and the trial implementation contract and product milestones are still open.
````

- Citation 1: experiments/prototype/RESULTS.md — "The Canvas2D reference view does not settle the production renderer."
- Citation 2: experiments/performance/RESULTS.md — "The integrated-GPU laptop with 16 GB RAM remains the hardware qualification target."
- Citation 4: docs/prototype/TECH.md — "Rust/Wasm with wgpu remains the leading GPU candidate."
- Citation 6: experiments/incremental/FINGERPRINT_RESULTS.md — "Renderer selection and hardware qualification remain separate work; these results do not select a production GPU backend"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 0/2.

## qwen3.5-2b-q4/vizzi.oncall/retrieved

Who is on call for Vizzi this week?

Expected: insufficient_evidence; split: development; category: limits.

- Forbidden: Infers an on-call assignment from authorship.

Candidate status: insufficient_evidence

````text
The provided excerpts contain code tests, configuration files, and documentation regarding the Vizzi repository's navigation, state management, and core logic, but they do not contain any information about personnel, team members, or individuals assigned to Vizzi this week.
````


Provenance: true; . Preferred anchors: 0/0.

## qwen3.5-2b-q4/vizzi.production-latency/retrieved

What was the exact production p99 request latency of the last Vizzi deployment?

Expected: insufficient_evidence; split: held_out; category: limits.

- Forbidden: Uses local benchmark numbers as production telemetry.

Candidate status: insufficient_evidence

````text
The provided excerpts do not contain the exact production p99 request latency of the last Vizzi deployment. While the text mentions a 'last timestamp sample' and a 'race in the existing qualification helper' that could have caused a parse to start/finish between samples, it does not provide the specific numerical value for the p99 latency of the final deployment.
````


Provenance: true; . Preferred anchors: 0/0.

## qwen3.5-2b-q4/lpc-rs.purpose/retrieved

What is lpc-rs, and what kind of applications is it intended to run?

Expected: answered; split: development; category: orientation.

- Fact 1: It is an LPC compiler, virtual machine and gamedriver for MUDs.
- Fact 2: A mudlib is the LPC code defining a game/world.

Candidate status: insufficient_evidence

````text
The provided excerpts describe the `lpc-rs` project as a support crate containing error handling, assembly instructions, function support, and utilities. However, none of the supplied text specifies what applications or environments the project is intended to run.
````


Provenance: true; . Preferred anchors: 0/2.

## qwen3.5-2b-q4/lpc-rs.purpose/oracle_source

What is lpc-rs, and what kind of applications is it intended to run?

Expected: answered; split: development; category: orientation.

- Fact 1: It is an LPC compiler, virtual machine and gamedriver for MUDs.
- Fact 2: A mudlib is the LPC code defining a game/world.

Candidate status: answered

````text
lpc-rs is an LPC compiler, virtual machine, and gamedriver for multi-user dungeons (MUDs). It is intended to run applications that define persistent, multiplayer text worlds using the LPC language, including rooms, creatures, player commands, and game rules.
````

- Citation 1: README.md — "An LPC compiler, virtual machine and gamedriver for multi-user dungeons (MUDs)."

Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-2b-q4/lpc-rs.language/retrieved

What implementation language and Rust edition does lpc-rs use?

Expected: answered; split: development; category: orientation.

- Fact 1: Its implementation uses Rust.
- Fact 2: The package declares edition 2024.

Candidate status: answered

````text
lpc-rs uses Rust as its implementation language and specifies edition 2024 in its Cargo.toml files.
````

- Citation 1: lpc-rs-asm/Cargo.toml — "edition = \"2024\""

Provenance: true; . Preferred anchors: 1/2.

## qwen3.5-2b-q4/lpc-rs.language/oracle_source

What implementation language and Rust edition does lpc-rs use?

Expected: answered; split: development; category: orientation.

- Fact 1: Its implementation uses Rust.
- Fact 2: The package declares edition 2024.

Candidate status: answered

````text
lpc-rs uses the Rust language with the 2024 edition, as specified in the `edition = "2024"` field of the `Cargo.toml` file.
````

- Citation 2: Cargo.toml — "edition = \"2024\""

Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-2b-q4/lpc-rs.entry/retrieved

Where does the gamedriver executable start, and how does it boot the runtime?

Expected: answered; split: development; category: architecture.

- Fact 1: The entry is lpc-rs-driver/src/main.rs and calls runtime::run.
- Fact 2: It creates a VM from configuration and calls boot.

Candidate status: insufficient_evidence

````text
The provided excerpts describe the repository structure, build process, and runtime components (e.g., `lpc-rs-driver`, `interpreter/vm/`), but do not contain specific details about the exact entry point or boot sequence of the gamedriver executable.
````


Provenance: true; . Preferred anchors: 0/2.

## qwen3.5-2b-q4/lpc-rs.configuration/retrieved

How does the gamedriver select a configuration file, and which environment setting supplies the mudlib root?

Expected: answered; split: development; category: configuration.

- Fact 1: The --env option supplies a configuration file through ConfigBuilder.load_env.
- Fact 2: LPC_LIB_DIR is preferred, with LIB_DIR as fallback.

Candidate status: insufficient_evidence

````text
The provided excerpts do not contain information about how the gamedriver selects a configuration file or which environment setting supplies the mudlib root. While `lpc-rs-lsp` documentation mentions `--config` for environment files and `LIB_DIR`/`LPC_LIB_DIR` for the mudlib root, it does not describe the gamedriver's selection logic or specific environment variables used for this purpose.
````


Provenance: true; . Preferred anchors: 0/2.

## qwen3.5-2b-q4/lpc-rs.configuration/oracle_source

How does the gamedriver select a configuration file, and which environment setting supplies the mudlib root?

Expected: answered; split: development; category: configuration.

- Fact 1: The --env option supplies a configuration file through ConfigBuilder.load_env.
- Fact 2: LPC_LIB_DIR is preferred, with LIB_DIR as fallback.

Candidate status: insufficient_evidence

````text
The provided excerpts do not contain information about how the `gamedriver` selects a configuration file or which environment variable supplies the mudlib root. The text only shows the `lpc-rs-driver` source code, which uses `args.env` for configuration, but lacks details on the `gamedriver` logic or specific environment variable names for the mudlib root.
````


Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-2b-q4/lpc-rs.validation/retrieved

Which commands does the project document for workspace builds, tests, linting and formatting?

Expected: answered; split: held_out; category: testing.

- Fact 1: Use cargo build --workspace and cargo test --workspace.
- Fact 2: Use cargo clippy --workspace --all-targets -- -D warnings and cargo fmt --check.

Candidate status: answered

````text
The project documents the following commands for workspace builds, tests, linting, and formatting: `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets`, and `cargo fmt --check`.
````

- Citation 1: README.md — "cargo build --workspace"
- Citation 1: README.md — "cargo test --workspace"
- Citation 1: README.md — "cargo clippy --workspace --all-targets"
- Citation 1: README.md — "cargo fmt --check"

Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-2b-q4/lpc-rs.validation/oracle_source

Which commands does the project document for workspace builds, tests, linting and formatting?

Expected: answered; split: held_out; category: testing.

- Fact 1: Use cargo build --workspace and cargo test --workspace.
- Fact 2: Use cargo clippy --workspace --all-targets -- -D warnings and cargo fmt --check.

Candidate status: answered

````text
The project documents the following commands for workspace builds, tests, linting, and formatting: `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --check`.
````

- Citation 1: README.md — "Run the workspace checks from the repository root: `cargo build --workspace`"
- Citation 1: README.md — "Run the workspace checks from the repository root: `cargo test --workspace`"
- Citation 1: README.md — "Run the workspace checks from the repository root: `cargo clippy --workspace --all-targets -- -D warnings`"
- Citation 1: README.md — "Run the workspace checks from the repository root: `cargo fmt --check`"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-2b-q4/lpc-rs.safety/retrieved

Does the main lpc-rs library permit unsafe Rust?

Expected: answered; split: held_out; category: constraints.

- Fact 1: The library explicitly forbids unsafe code.

Candidate status: insufficient_evidence

````text
The provided excerpts do not explicitly state whether the main `lpc-rs` library permits unsafe Rust. While the code snippets show `#![forbid(unsafe_code)]` in the `lpc-rs-lsp` crate, this applies to that specific crate, not the root `lpc-rs` workspace. The `AGENTS.md` file mentions the compiler is a 'work-in-progress' but does not specify safety constraints for the main library itself.
````


Provenance: true; . Preferred anchors: 0/1.

## qwen3.5-2b-q4/lpc-rs.reader/retrieved

Which abstraction supplies compiler source bytes, and what is the default implementation?

Expected: answered; split: development; category: interfaces.

- Fact 1: Compiler.read_file uses the injected SourceReader or DiskSourceReader.
- Fact 2: DiskSourceReader reads bytes using tokio::fs::read.

Candidate status: answered

````text
The `SourceReader` trait supplies compiler source bytes. Its default implementation is `DiskSourceReader`, which reads bytes directly from disk using `tokio::fs::read`.
````

- Citation 4: src/compiler/source_reader.rs — "pub trait SourceReader: Debug + Send + Sync {"
- Citation 4: src/compiler/source_reader.rs — "impl SourceReader for DiskSourceReader {"

Provenance: true; . Preferred anchors: 1/2.

## qwen3.5-2b-q4/lpc-rs.reader/oracle_source

Which abstraction supplies compiler source bytes, and what is the default implementation?

Expected: answered; split: development; category: interfaces.

- Fact 1: Compiler.read_file uses the injected SourceReader or DiskSourceReader.
- Fact 2: DiskSourceReader reads bytes using tokio::fs::read.

Candidate status: answered

````text
The `DiskSourceReader` abstraction supplies compiler source bytes via the `read` method, which returns `io::Result<Vec<u8>>`. Its default implementation uses `tokio::fs::read(path).await` to read the file contents from disk.
````

- Citation 1: src/compiler/mod.rs — "async fn read_file(&self, path: LpcPath) -> Result<(Arc<CompilerSource>, LpcSource)> {"
- Citation 2: src/compiler/source_reader.rs — "async fn read(&self, path: &Path) -> io::Result<Vec<u8>> {\n        tokio::fs::read(path).await\n      }"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-2b-q4/lpc-rs.fallback/retrieved

When does the compiler retry a source path with the c extension, and what prevents an endless retry?

Expected: answered; split: development; category: failure.

- Fact 1: Retry is limited to NotFound errors.
- Fact 2: It retries only when the extension is not already c.
- Forbidden: Retries permission errors or repeatedly retries an already-c path.

Candidate status: answered

````text
The compiler retries a source path with the `.c` extension when the initial read fails with `ErrorKind::NotFound` and the path does not end in `.c`. This retry loop continues until the file is successfully read or an error occurs (e.g., permission denied), preventing an endless retry by limiting the number of failed attempts via a `failures` counter in the `RecoveringReader` struct.
````

- Citation 2: src/compiler/mod.rs — "if error.kind() == ErrorKind::NotFound && absolute.extension().and_then(OsStr::to_str) != Some(\"c\")"
- Citation 3: src/compiler/mod.rs — "if reads.len() <= self.failures { Err(io::Error::new(self.error, \"source unavailable\")) }"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-2b-q4/lpc-rs.fallback/oracle_source

When does the compiler retry a source path with the c extension, and what prevents an endless retry?

Expected: answered; split: development; category: failure.

- Fact 1: Retry is limited to NotFound errors.
- Fact 2: It retries only when the extension is not already c.
- Forbidden: Retries permission errors or repeatedly retries an already-c path.

Candidate status: answered

````text
The compiler retries a source path with the `.c` extension when the initial read fails with `ErrorKind::NotFound` and the file extension is not `.c`. It prevents an endless retry by checking if the file extension is `.c` before attempting to create a new source object; if it is not `.c`, the loop terminates.
````

- Citation 1: src/compiler/mod.rs — "if error.kind() == ErrorKind::NotFound && absolute.extension().and_then(OsStr::to_str) != Some(\"c\")"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-2b-q4/lpc-rs.host-vs-game/retrieved

Are host-facing compile_file inputs and in-game compilation confined to the mudlib in the same way?

Expected: answered; split: held_out; category: constraints.

- Fact 1: CompilerSource permits host-facing compiler input identity without imposing confinement.
- Fact 2: compile_in_game_file validates the in-game path before compilation.
- Forbidden: Claims all host-facing compile_file calls are confined by the same validation.

Candidate status: answered

````text
Yes, host-facing compile_file inputs and in-game compilation are confined to the mudlib. The compiler validates in-game paths using `validate_in_game_path`, which ensures the path is within the configured mudlib. Outside the mudlib, both path macros yield the opaque label `<outside mudlib>`.
````

- Citation 1: doc/lpc/macros.md — "For a host-facing compile outside the configured mudlib, both path macros yield the opaque label <outside mudlib>."
- Citation 6: doc/apply/master/compile_object.md — "ad_object`, `clone_object`, `file_name`"

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 1/2.

## qwen3.5-2b-q4/lpc-rs.host-vs-game/oracle_source

Are host-facing compile_file inputs and in-game compilation confined to the mudlib in the same way?

Expected: answered; split: held_out; category: constraints.

- Fact 1: CompilerSource permits host-facing compiler input identity without imposing confinement.
- Fact 2: compile_in_game_file validates the in-game path before compilation.
- Forbidden: Claims all host-facing compile_file calls are confined by the same validation.

Candidate status: insufficient_evidence

````text
The provided excerpts define the `CompilerSource` struct and its methods like `compile_in_game_file`, but do not explicitly state whether host-facing inputs are confined to the mudlib in the same way as in-game compilation. While `compile_in_game_file` is shown to be confined to the lib, the behavior of `compile_file` for host-facing inputs is not described in the text.
````


Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-2b-q4/lpc-rs.source-identity/retrieved

Why does CompilerSource keep both a resolved host location and a separate diagnostic name?

Expected: answered; split: held_out; category: state.

- Fact 1: resolved holds the host location used to read/register source.
- Fact 2: name is a safe input spelling for file errors and permission applies.

Candidate status: answered

````text
CompilerSource maintains a resolved host location (`resolved`) for canonical diagnostic locationing and a separate name (`name`) to safely display source names in file errors and inheritance permission contexts, ensuring distinct outside-host inputs are handled internally while preserving safe display names.
````

- Citation 1: src/compiler/source.rs — "/// The host location and canonical diagnostic location used to register source text."
- Citation 2: src/compiler/source.rs — "/// The safe input spelling used in file errors and inheritance permission applies."

Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-2b-q4/lpc-rs.source-identity/oracle_source

Why does CompilerSource keep both a resolved host location and a separate diagnostic name?

Expected: answered; split: held_out; category: state.

- Fact 1: resolved holds the host location used to read/register source.
- Fact 2: name is a safe input spelling for file errors and permission applies.

Candidate status: answered

````text
CompilerSource maintains a resolved host location to register source text canonically while keeping a separate diagnostic name for safe input spelling during file errors and permission checks, ensuring distinct identity for function mangling and configured-file comparisons.
````

- Citation 1: src/compiler/source.rs — "The original identity used by function mangling and configured-file comparisons."
- Citation 2: src/compiler/source.rs — "The safe input spelling used in file errors and inheritance permission applies."

Provenance: true; . Preferred anchors: 2/2.

## qwen3.5-2b-q4/lpc-rs.fallback-test/retrieved

Which regression test covers stopping source retries and preserving other I/O errors?

Expected: answered; split: development; category: testing.

- Fact 1: The named test is file_reads_stop_after_extension_fallback_and_preserve_other_io_errors in src/compiler/mod.rs.

Candidate status: insufficient_evidence

````text
The provided excerpts describe Lima port history, compilation checks, and specific mudlib adaptations (like `set_heart_beat` and `navigation_passage`), but do not contain any information about regression tests or specific test cases for stopping source retries while preserving other I/O errors.
````


Provenance: true; . Preferred anchors: 0/1.

## qwen3.5-2b-q4/lpc-rs.probe/retrieved

Does checking whether a path is a file or directory require reading its source bytes?

Expected: answered; split: held_out; category: implementation.

- Fact 1: SourceReader exposes a separate kind probe.
- Fact 2: DiskSourceReader.kind uses filesystem metadata.

Candidate status: insufficient_evidence

````text
The provided excerpts do not establish whether checking if a path is a file or directory requires reading its source bytes. While `file_size` (Excerpt [2]) reads filesystem metadata to determine if a path is a directory, and `DiskSourceReader::kind` (Excerpt [3]) probes paths without reading contents, the specific logic for the `file_view` or `file_view::read_through` mechanism used in the `read_save` function (Excerpt [1]) is not described. The code shows that `file_view::read_through` is called to obtain bytes, but it does not explicitly state whether this operation is required to perform the initial `kind` check or if metadata-only checks suffice.
````

- Citation 2: doc/efun/file_size.md — "The master's `valid_read` is asked first, and a refusal answers `-1` too."
- Citation 3: src/compiler/source_reader.rs — "Probe a path without reading its contents; missing paths return `NotFound`."

Provenance: false; Citation quote does not match supplied source. Preferred anchors: 2/2.

## qwen3.5-2b-q4/lpc-rs.oncall/retrieved

Who is on call for lpc-rs this week?

Expected: insufficient_evidence; split: development; category: limits.

- Forbidden: Infers an on-call assignment from package authors.

Candidate status: insufficient_evidence

````text
The provided excerpts describe the `lpc-rs` interpreter's `previous_object` mechanism and date/time conversion functions but do not contain information about the current week's callers or scheduled tasks for this repository.
````


Provenance: true; . Preferred anchors: 0/0.

## qwen3.5-2b-q4/lpc-rs.production-latency/retrieved

What was the exact production p99 request latency of the last lpc-rs deployment?

Expected: insufficient_evidence; split: held_out; category: limits.

- Forbidden: Invents production telemetry from benchmarks or configuration.

Candidate status: insufficient_evidence

````text
The provided excerpts do not contain information about the last lpc-rs deployment's production p99 request latency. While the text mentions a baseline commit `5760b740` and discusses latency metrics (p50, p95, p99) for various scenarios in a test environment, it does not specify the production deployment details or the specific p99 value for the final deployment.
````


Provenance: true; . Preferred anchors: 0/0.

## qwen3.5-2b-q4/changing.retry-base/retrieved

What is the configured retry_limit?

Expected: answered; split: development; category: state.

- Fact 1: retry_limit is 3.
- Forbidden: Uses facts or source definitions from an earlier fixture revision.

Candidate status: answered

````text
The configured retry_limit is 3.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 1/1.

## qwen3.5-2b-q4/changing.retry-updated/retrieved

What is the configured retry_limit?

Expected: answered; split: development; category: state.

- Fact 1: retry_limit is 7.
- Forbidden: Uses facts or source definitions from an earlier fixture revision.

Candidate status: answered

````text
The configured retry_limit is 7, as explicitly defined in config.toml:1-1.
````


Provenance: false; An answer needs cited evidence. Preferred anchors: 1/1.

## qwen3.5-2b-q4/changing.renamed-handler/retrieved

What function is the packet loading handler now, and where is it defined?

Expected: answered; split: held_out; category: state.

- Fact 1: load_packet is defined in src/transport.rs.
- Forbidden: Uses facts or source definitions from an earlier fixture revision.

Candidate status: answered

````text
The packet loading handler is defined as `pub fn load_packet()` in the file `src/transport.rs`.
````

- Citation 1: src/transport.rs — "/// The packet loading handler."
- Citation 2: unavailable — "pub fn load_packet() {}"

Provenance: false; Citation label is unavailable. Preferred anchors: 1/1.

## qwen3.5-2b-q4/changing.deleted-handler/retrieved

Where is load_packet currently defined?

Expected: insufficient_evidence; split: held_out; category: state.

- Forbidden: Uses facts or source definitions from an earlier fixture revision.

Candidate status: insufficient_evidence

````text
The provided excerpts do not contain any information regarding the definition of the `load_packet` function or its location within the repository.
````


Provenance: true; . Preferred anchors: 0/0.

