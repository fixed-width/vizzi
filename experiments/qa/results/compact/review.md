# Source-aware response review

For evaluators. Contains gold facts; never index this artifact or send it to candidates. Grading is source-aware agent work, not independent human acceptance.

## v2.vizzi.language/legacy_quotes

What implementation languages are used by the native engine and browser interface, and which language edition is declared for the native workspace?

Expected: answered. Native source checks: False.

- Fact 1 (contradicted): The native implementation uses Rust and the browser adapter uses JavaScript.
- Fact 2 (supported): The workspace declares Rust edition 2024.

````json
{
  "status": "answered",
  "answer": "The native engine uses Rust, while the browser interface uses Python. The native workspace declares the Rust 2024 edition.",
  "citations": [
    {
      "label": 2,
      "quote": "Rust and Python now use one source policy and one bounded analysis queue."
    },
    {
      "label": 1,
      "quote": "Use Rust with the 2024 edition, declared in workspace package metadata and inherited by implementation crates."
    }
  ]
}
````

- [2] experiments/languages/RESULTS.md:1-30 (historical)
- [1] AGENTS.md:1-35 (policy)

Rust/2024 is supported by AGENTS. Python is a mistaken transfer from an analysis-language report; one quote also loses the literal line wrapping.

Unsupported: The browser interface uses Python. The cited report describes supported analysis languages, not the browser implementation.

Native rejection: Citation quote does not match supplied source

## v2.vizzi.language/compact_quotes

What implementation languages are used by the native engine and browser interface, and which language edition is declared for the native workspace?

Expected: answered. Native source checks: True.

- Fact 1 (missing): The native implementation uses Rust and the browser adapter uses JavaScript.
- Fact 2 (missing): The workspace declares Rust edition 2024.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts describe the repository structure, dependencies, and contract interfaces but do not specify the implementation languages used for the native engine or browser interface, nor do they explicitly state the Rust edition declared for the native workspace.",
  "citations": []
}
````


The browser language is absent, but the supplied manifest plainly establishes the edition. Refusal misses both required facts.

Unsupported: Claims that the excerpts do not explicitly state the Rust edition despite Cargo.toml edition = 2024.

## v2.vizzi.language/compact_ids

What implementation languages are used by the native engine and browser interface, and which language edition is declared for the native workspace?

Expected: answered. Native source checks: True.

- Fact 1 (missing): The native implementation uses Rust and the browser adapter uses JavaScript.
- Fact 2 (missing): The workspace declares Rust edition 2024.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts describe the repository structure, dependencies, and contract for the `vizzi` workspace but do not specify the implementation languages used for the native engine or browser interface, nor do they explicitly state the Rust edition declared for the native workspace.",
  "citations": []
}
````


Same material refusal as the compact quote arm. Empty references are structurally valid for insufficient_evidence, not a correct answer.

Unsupported: Claims that the excerpts do not explicitly state the Rust edition despite Cargo.toml edition = 2024.

## v2.lpc-rs.language/compact_quotes

What implementation language and language edition does the project declare?

Expected: answered. Native source checks: True.

- Fact 1 (contradicted): Its implementation uses Rust.
- Fact 2 (supported): The package declares edition 2024.

````json
{
  "status": "answered",
  "answer": "The project declares the implementation language as 'lpc' (LPC compiler and gamedriver suite) and the edition as '2024'.",
  "citations": [
    {
      "label": 1,
      "quote": "edition = \"2024\""
    },
    {
      "label": 1,
      "quote": "description = \"lpc-rs LPC compiler and gamedriver suite.\""
    }
  ]
}
````

- [1] Cargo.toml:1-23 (configuration)
- [1] Cargo.toml:1-23 (configuration)

Correct edition, incorrect language. Both quotations bind exactly but the package-description citation does not establish the claim.

Unsupported: The implementation language is claimed to be LPC; the package description identifies compiler purpose, not its implementation language.

## v2.lpc-rs.language/compact_ids

What implementation language and language edition does the project declare?

Expected: answered. Native source checks: True.

- Fact 1 (contradicted): Its implementation uses Rust.
- Fact 2 (supported): The package declares edition 2024.

````json
{
  "status": "answered",
  "answer": "The project declares the implementation language as 'lpc-rs' and the edition as '2024'.",
  "citations": [
    1,
    2
  ]
}
````

- [1] Cargo.toml:1-23 (configuration)
- [2] lpc-rs-asm/Cargo.toml:1-16 (configuration)

Correct edition. Canonical manifest IDs do not support treating the package name as a language.

Unsupported: The repository/package name lpc-rs is presented as an implementation language.

## v2.lpc-rs.language/legacy_quotes

What implementation language and language edition does the project declare?

Expected: answered. Native source checks: True.

- Fact 1 (contradicted): Its implementation uses Rust.
- Fact 2 (supported): The package declares edition 2024.

````json
{
  "status": "answered",
  "answer": "The project declares the implementation language as LPC and the edition as 2024.",
  "citations": [
    {
      "label": 2,
      "quote": "edition = \"2024\""
    },
    {
      "label": 6,
      "quote": "LPC is a language for building persistent, multiplayer text worlds."
    }
  ]
}
````

- [2] lpc-rs-asm/Cargo.toml:1-16 (configuration)
- [6] README.md:1-34 (guide)

Edition 2024 is explicit. Confuses target language with implementation language despite valid literal citations.

Unsupported: LPC is claimed as the implementation language; the cited description is the language implemented by the compiler.

## v2.lpc-rs.purpose/compact_ids

What is lpc-rs, and what kind of applications is it intended to run?

Expected: answered. Native source checks: True.

- Fact 1 (supported): It is an LPC compiler, virtual machine and gamedriver for MUDs.
- Fact 2 (supported): It runs persistent multiplayer text worlds, including rooms, creatures, commands and game rules.

````json
{
  "status": "answered",
  "answer": "lpc-rs is a Rust-based compiler, virtual machine, and game driver for multi-user dungeons (MUDs). It is intended to run applications that define persistent, multiplayer text worlds using the LPC dialect, including the game server, language server, and runtime components.",
  "citations": [
    1,
    2
  ]
}
````

- [1] README.md:1-16 (guide)
- [2] README.md:200-218 (guide)

The root introduction establishes compiler/VM/gamedriver and persistent multiplayer worlds; the workspace table establishes the named binaries/components. Accept the loose extra list as a project-level component summary, not a claim about their implementation language.

## v2.lpc-rs.purpose/legacy_quotes

What is lpc-rs, and what kind of applications is it intended to run?

Expected: answered. Native source checks: True.

- Fact 1 (missing): It is an LPC compiler, virtual machine and gamedriver for MUDs.
- Fact 2 (missing): It runs persistent multiplayer text worlds, including rooms, creatures, commands and game rules.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts describe the `lpc-rs` project as a support crate containing error handling, assembly instructions, function support, and utilities. However, none of the text explicitly states what applications this crate is intended to run or what the project's primary purpose is beyond being a support library.",
  "citations": []
}
````


Retrieval supplies support crates and an unrelated partial-application test, not the root project introduction. Refusal misses the question.

Unsupported: Treats support-crate descriptions as the overall lpc-rs project purpose.

## v2.lpc-rs.purpose/compact_quotes

What is lpc-rs, and what kind of applications is it intended to run?

Expected: answered. Native source checks: False.

- Fact 1 (supported): It is an LPC compiler, virtual machine and gamedriver for MUDs.
- Fact 2 (supported): It runs persistent multiplayer text worlds, including rooms, creatures, commands and game rules.

````json
{
  "status": "answered",
  "answer": "lpc-rs is a Rust-based compiler, virtual machine, and game driver for multi-user dungeons (MUDs). It is intended to run applications that define persistent, multiplayer text worlds using the LPC dialect, including game servers, language servers, and executable crates for running LPC files and managing TLS certificates.",
  "citations": [
    {
      "label": 1,
      "quote": "An LPC compiler, virtual machine and gamedriver for multi-user dungeons (MUDs)."
    },
    {
      "label": 2,
      "quote": "The project is under active development. It has a working compiler and networked server, but its LPC dialect and... are"
    }
  ]
}
````

- [1] README.md:1-16 (guide)
- [2] README.md:200-218 (guide)

Both material purpose facts are correctly stated from the root introduction. Extra executable names come from the workspace table. The second quotation is fabricated/ellipsized and assigned to a different excerpt, so strict reference binding and citation support fail.

Native rejection: Citation quote does not match supplied source

## v2.vizzi.validation/legacy_quotes

Which checks must a contributor run before submitting a change?

Expected: answered. Native source checks: False.

- Fact 1 (supported): Formatting is a baseline check.
- Fact 2 (supported): Workspace tests including doctests are required.
- Fact 3 (supported): Clippy is required.

````json
{
  "status": "answered",
  "answer": "Contributors must run TDD tests for empty/binary/Unicode inputs, arbitrary partitions, exact/short/extra bytes, same-length changes, backing-independent verification, and irreversible length mismatch. They must also run workspace/doctest/fmt/Clippy/browser checks, complete relevant core/native mutation scopes, and report compiler failures, timeouts, and exclusions separately from test catches.",
  "citations": [
    {
      "label": 4,
      "quote": "Run workspace/doctest/fmt/Clippy/browser checks plus complete relevant core/native mutation scopes."
    },
    {
      "label": 5,
      "quote": "Run real Chromium checks and focused/full relevant Rust/JS mutation scopes."
    }
  ]
}
````

- [4] docs/plans/verified-source-reuse.md:23-50 (guide)
- [5] docs/plans/windowed-inspection.md:41-57 (guide)

Names all baseline checks, but imports a specialised implementation-plan test matrix as global contributor policy. The second quote also fails exact binding.

Unsupported: Feature-specific test matrices from implementation plans are presented as requirements for every contributor change.

Native rejection: Citation quote does not match supplied source

## v2.vizzi.validation/compact_quotes

Which checks must a contributor run before submitting a change?

Expected: answered. Native source checks: True.

- Fact 1 (supported): Formatting is a baseline check.
- Fact 2 (supported): Workspace tests including doctests are required.
- Fact 3 (supported): Clippy is required.

````json
{
  "status": "answered",
  "answer": "Contributors must run baseline checks including formatting, workspace tests (doctests), and Clippy, as well as mutation testing referencing [cargo-mutants](https://mutants.rs/) and testing changes in a diff.",
  "citations": [
    {
      "label": 1,
      "quote": "Baseline checks are formatting, workspace tests (including doctests), and Clippy."
    }
  ]
}
````

- [1] AGENTS.md:62-70 (policy)

Accurately gives formatting, workspace tests including doctests and Clippy from AGENTS, with a literal source quote. Mutation-tool references are also present in the supplied policy excerpt.

## v2.vizzi.validation/compact_ids

Which checks must a contributor run before submitting a change?

Expected: answered. Native source checks: True.

- Fact 1 (supported): Formatting is a baseline check.
- Fact 2 (supported): Workspace tests including doctests are required.
- Fact 3 (supported): Clippy is required.

````json
{
  "status": "answered",
  "answer": "Contributors must run baseline checks (formatting, workspace tests, and Clippy), validate native and browser implementations, and perform mutation testing using tools like cargo-mutants.",
  "citations": [
    1
  ]
}
````

- [1] AGENTS.md:62-70 (policy)

Correct baseline checks plus affected-target validation and cargo-mutants. Workspace tests are accepted as the documented baseline including doctests; no omission/bypass is claimed.

## v2.vizzi.search/compact_quotes

Which text does 'Find a name or file' match: paths and declaration names, arbitrary function-body text, or both?

Expected: answered. Native source checks: True.

- Fact 1 (supported): Search matches captured declaration names and paths, not full source bodies.

````json
{
  "status": "answered",
  "answer": "The 'Find a name or file' feature matches both declaration names and file paths, not arbitrary function-body text.",
  "citations": [
    {
      "label": 2,
      "quote": "Search matches declaration names and paths, not full source text."
    }
  ]
}
````

- [2] docs/how-to/investigate-with-vscode.md:19-29 (guide)

Exactly distinguishes declaration/path matching from arbitrary body text and quotes the relevant guide.

## v2.vizzi.search/compact_ids

Which text does 'Find a name or file' match: paths and declaration names, arbitrary function-body text, or both?

Expected: answered. Native source checks: True.

- Fact 1 (supported): Search matches captured declaration names and paths, not full source bodies.

````json
{
  "status": "answered",
  "answer": "The 'Find a name or file' action matches both declaration names and file paths, not arbitrary function-body text.",
  "citations": [
    2
  ]
}
````

- [2] docs/how-to/investigate-with-vscode.md:19-29 (guide)

The guide at label 2 explicitly supports both fields and the exclusion of full source text.

## v2.vizzi.search/legacy_quotes

Which text does 'Find a name or file' match: paths and declaration names, arbitrary function-body text, or both?

Expected: answered. Native source checks: True.

- Fact 1 (missing): Search matches captured declaration names and paths, not full source bodies.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts do not specify whether the 'Find a name or file' input matches paths, declaration names, or both. While the code in `crates/vizzi-core/src/discovery.rs` uses `path` and `search` fields in `DiscoveryQuery`, and `docs/prototype/C_FAMILY_SYNTAX.md` describes how to select inputs, neither excerpt explicitly states the exact behavior of the 'Find a name or file' field.",
  "citations": []
}
````


Refuses rather than establish the match fields; the selected control context lacks the direct guide statement used by the compact arms.

## v2.vizzi.connections/compact_ids

What does a saved connection mean in Vizzi: a resolved call/dependency relationship, or the reader's interpretation?

Expected: answered. Native source checks: True.

- Fact 1 (supported): Connections are reader-authored assertions.
- Fact 2 (supported): They are not inferred call, dependency or impact claims.

````json
{
  "status": "answered",
  "answer": "A saved connection in Vizzi represents the reader's interpretation, not a resolved call or dependency relationship. The documentation explicitly states that connections are 'reader-authored' and serve as assertions of relationships, which are not proof of runtime dependencies or impact.",
  "citations": [
    1,
    2
  ]
}
````

- [1] docs/how-to/investigate-with-vscode.md:29-39 (guide)
- [2] docs/companion/PRODUCT.md:28-39 (policy)

Correctly gives reader-authored interpretation and excludes resolved/inferred dependency proof; guide and contract directly support both.

## v2.vizzi.connections/legacy_quotes

What does a saved connection mean in Vizzi: a resolved call/dependency relationship, or the reader's interpretation?

Expected: answered. Native source checks: False.

- Fact 1 (supported): Connections are reader-authored assertions.
- Fact 2 (supported): They are not inferred call, dependency or impact claims.

````json
{
  "status": "answered",
  "answer": "A saved connection in Vizzi represents the reader's interpretation, not a resolved call or dependency relationship. The documentation explicitly states that 'Dashed connections and their list entries represent your assertions; Vizzi does not infer that they prove a call, dependency or impact relationship' (docs/how-to/investigate-with-vscode.md:24-50).",
  "citations": [
    {
      "label": 1,
      "quote": "Dashed connections and their list entries represent your assertions; Vizzi does not infer that they prove a call, a call"
    }
  ]
}
````

- [1] docs/how-to/investigate-with-vscode.md:24-50 (guide)

Body correctly states the distinction, but the quote repeats/invents the ending (a call, a call) and fails exact source binding.

Native rejection: Citation quote does not match supplied source

## v2.vizzi.connections/compact_quotes

What does a saved connection mean in Vizzi: a resolved call/dependency relationship, or the reader's interpretation?

Expected: answered. Native source checks: False.

- Fact 1 (supported): Connections are reader-authored assertions.
- Fact 2 (supported): They are not inferred call, dependency or impact claims.

````json
{
  "status": "answered",
  "answer": "A saved connection in Vizzi represents the reader's interpretation, not a resolved call or dependency relationship. The documentation explicitly states that connections are 'explicitly reader-authored' and serve as assertions of relationships, not proof of runtime dependencies or impact.",
  "citations": [
    {
      "label": 1,
      "quote": "Dashed connections and their list entries represent your assertions; Vizzi does not infer that they prove a call, or a, "
    },
    {
      "label": 2,
      "quote": "Every connection is explicitly reader-authored, not an inferred call/dependency/impact claim."
    }
  ]
}
````

- [1] docs/how-to/investigate-with-vscode.md:29-39 (guide)
- [2] docs/companion/PRODUCT.md:28-39 (policy)

Correct interpretation and lack of inferred dependency proof, but both copied references are edited or miscopied; one repeats fragments and the other flattens source wrapping.

Native rejection: Citation quote does not match supplied source

## v2.lpc-rs.configuration/legacy_quotes

How can I select the gamedriver environment file, and which environment setting has precedence for its mudlib root?

Expected: answered. Native source checks: False.

- Fact 1 (contradicted): Use --env to select the gamedriver environment file, passed to ConfigBuilder.load_env.
- Fact 2 (supported): LPC_LIB_DIR has precedence, with LIB_DIR as fallback.

````json
{
  "status": "answered",
  "answer": "To select the gamedriver environment file, use `--config <path/to/driver.env>` in the launch command. The `LPC_`-prefixed settings have precedence over their unprefixed forms.",
  "citations": [
    {
      "label": 1,
      "quote": "Pass `--config /path/to/driver.env` to load the same explicit environment file as the compiler."
    },
    {
      "label": 2,
      "quote": "`LPC_`-prefixed settings take precedence over their unprefixed forms."
    }
  ]
}
````

- [1] lpc-rs-lsp/README.md:1-53 (guide)
- [2] README.md:90-133 (guide)

The general LPC_-prefix precedence rule is accepted as establishing LPC_LIB_DIR over LIB_DIR in this context. The environment-file flag is for the wrong executable, and exact quote binding fails.

Unsupported: Uses the language-server --config option as the gamedriver option; the driver uses --env.

Native rejection: Citation quote does not match supplied source

## v2.lpc-rs.configuration/compact_quotes

How can I select the gamedriver environment file, and which environment setting has precedence for its mudlib root?

Expected: answered. Native source checks: False.

- Fact 1 (supported): Use --env to select the gamedriver environment file, passed to ConfigBuilder.load_env.
- Fact 2 (contradicted): LPC_LIB_DIR has precedence, with LIB_DIR as fallback.

````json
{
  "status": "answered",
  "answer": "To select the gamedriver environment file, use `lpc-rs-driver --env <file>`, where `<file>` is the name of the `.env` file (e.g., `driver.env`). The precedence for mudlib root is determined by the `LIB_DIR` environment variable, which must be set to the host filesystem path pointing to the mudlib directory.",
  "citations": [
    {
      "label": 1,
      "quote": "Edit `LIB_DIR` to point to your mudlib and `MASTER_OBJECT` to name its master object."
    },
    {
      "label": 2,
      "quote": "lpc-rs-driver --env <file>"
    }
  ]
}
````

- [1] README.md:102-125 (guide)
- [2] lpc-rs-lsp/README.md:68-78 (guide)

Correct --env selection, incorrect precedence despite the excerpt explicitly stating the prefix rule. The abstract command quote is also assigned to an unrelated excerpt.

Unsupported: Says LIB_DIR determines mudlib-root precedence, overlooking the higher-precedence LPC_LIB_DIR form.

Native rejection: Citation quote does not match supplied source

## v2.lpc-rs.configuration/compact_ids

How can I select the gamedriver environment file, and which environment setting has precedence for its mudlib root?

Expected: answered. Native source checks: True.

- Fact 1 (supported): Use --env to select the gamedriver environment file, passed to ConfigBuilder.load_env.
- Fact 2 (contradicted): LPC_LIB_DIR has precedence, with LIB_DIR as fallback.

````json
{
  "status": "answered",
  "answer": "To select the gamedriver environment file, use `lpc-rs-driver --env <file>`, where `<file>` is the name of the `.env` file (e.g., `driver.env`). The precedence for mudlib root is determined by the `LIB_DIR` environment variable, which must be set to the host filesystem path pointing to the mudlib directory.",
  "citations": [
    1,
    3
  ]
}
````

- [1] README.md:102-125 (guide)
- [3] specs/ulib/PRODUCT.md:7-21 (policy)

Same substantive answer as the compact quote arm. Canonical labels are available but do not establish the claimed precedence.

Unsupported: Says LIB_DIR determines mudlib-root precedence, overlooking the higher-precedence LPC_LIB_DIR form.

## v2.lpc-rs.host-vs-game/compact_quotes

Can the host-facing compiler accept a source path outside the mudlib, and how does in-game compilation enforce its boundary?

Expected: answered. Native source checks: False.

- Fact 1 (supported): CompilerSource permits host-facing compiler input identity without imposing confinement.
- Fact 2 (contradicted): compile_in_game_file validates the in-game path before compilation.

````json
{
  "status": "answered",
  "answer": "Yes, the host-facing compiler accepts source paths outside the mudlib, but in-game compilation enforces this boundary by rejecting any source file that is not part of the configured mudlib. This is enforced because in-game source identity is derived solely from the normalized path and compilation-local source ID, which are not part of the file's identity. Defining a class in a host source outside the mudlib is rejected due to this lack of in-game source identity.",
  "citations": [
    {
      "label": 1,
      "quote": "For a host-facing compile outside the configured mudlib, both path macros yield the opaque label `<outside mudlib>`."
    },
    {
      "label": 2,
      "quote": "Defining a class in a host source outside the mudlib is rejected because it has no in-game source identity."
    }
  ]
}
````

- [1] doc/lpc/macros.md:54-65 (guide)
- [2] doc/lpc/classes.md:46-67 (guide)

The coarse host/outside and game/boundary distinction is partly right, but the claimed enforcement mechanism is unsupported. Literal reference binding also fails.

Unsupported: Explains in-game confinement through class/source-identity rules rather than the configured-root path validation; its account of compilation-local IDs and identity is also inconsistent with the class excerpt.

Native rejection: Citation quote does not match supplied source

## v2.lpc-rs.host-vs-game/compact_ids

Can the host-facing compiler accept a source path outside the mudlib, and how does in-game compilation enforce its boundary?

Expected: answered. Native source checks: True.

- Fact 1 (missing): CompilerSource permits host-facing compiler input identity without imposing confinement.
- Fact 2 (missing): compile_in_game_file validates the in-game path before compilation.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts confirm that host-facing compilers outside the configured mudlib yield the opaque label `<outside mudlib>` for path macros, but they do not specify whether the compiler accepts such paths or how in-game compilation enforces the boundary.",
  "citations": [
    1
  ]
}
````

- [1] doc/lpc/macros.md:54-65 (guide)

Faithfully limits itself to the macro-label statement and identifies the missing path-validation evidence. It does not answer either material part of the repository task.

## v2.lpc-rs.host-vs-game/legacy_quotes

Can the host-facing compiler accept a source path outside the mudlib, and how does in-game compilation enforce its boundary?

Expected: answered. Native source checks: True.

- Fact 1 (supported): CompilerSource permits host-facing compiler input identity without imposing confinement.
- Fact 2 (missing): compile_in_game_file validates the in-game path before compilation.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts confirm that the compiler accepts paths outside the mudlib (e.g., `src/compiler/source.rs`), but they do not specify how in-game compilation enforces this boundary. The code shows that `DiskSourceReader` reads files directly from disk without validating paths against the mudlib directory. No excerpt describes the enforcement mechanism or the specific validation logic used during in-game compilation.",
  "citations": []
}
````


Explicitly accepts host paths outside the mudlib but cannot establish the in-game path validation entry point; status is insufficient_evidence and references are empty.

## v2.lpc-rs.reader/compact_ids

Which abstraction supplies compiler source bytes, and what is the default implementation?

Expected: answered. Native source checks: True.

- Fact 1 (supported): Compiler.read_file uses the injected SourceReader or DiskSourceReader.
- Fact 2 (missing): DiskSourceReader reads bytes using tokio::fs::read.

````json
{
  "status": "answered",
  "answer": "The `SourceReader` trait supplies compiler source bytes, and `DiskSourceReader` is the default implementation.",
  "citations": [
    1
  ]
}
````

- [1] src/compiler/source_reader.rs:5-30 (code)

Accurately answers the actual wording with the trait, concrete implementation and host-facing disk doc comment at label 1. Strict frozen scoring still fails the unasked I/O API fact; no post-hoc rubric edit is applied.

Rubric issue: The frozen second fact requires the tokio::fs::read API although the question asks only for the abstraction and default implementation.

## v2.lpc-rs.reader/legacy_quotes

Which abstraction supplies compiler source bytes, and what is the default implementation?

Expected: answered. Native source checks: True.

- Fact 1 (supported): Compiler.read_file uses the injected SourceReader or DiskSourceReader.
- Fact 2 (supported): DiskSourceReader reads bytes using tokio::fs::read.

````json
{
  "status": "answered",
  "answer": "The `SourceReader` trait supplies compiler source bytes, with `DiskSourceReader` as the default implementation, which reads bytes directly from disk using `tokio::fs::read`.",
  "citations": [
    {
      "label": 4,
      "quote": "pub trait SourceReader: Debug + Send + Sync {"
    },
    {
      "label": 4,
      "quote": "impl SourceReader for DiskSourceReader {"
    }
  ]
}
````

- [4] src/compiler/source_reader.rs:9-45 (code)
- [4] src/compiler/source_reader.rs:9-45 (code)

Correct trait, concrete disk implementation and tokio::fs::read; the full supplied source-reader excerpt includes the implementation body. The host-facing disk doc comment is accepted as sufficient for the default-role interpretation.

## v2.lpc-rs.reader/compact_quotes

Which abstraction supplies compiler source bytes, and what is the default implementation?

Expected: answered. Native source checks: False.

- Fact 1 (supported): Compiler.read_file uses the injected SourceReader or DiskSourceReader.
- Fact 2 (missing): DiskSourceReader reads bytes using tokio::fs::read.

````json
{
  "status": "answered",
  "answer": "The `SourceReader` trait supplies compiler source bytes, and the default implementation is `DiskSourceReader`.",
  "citations": [
    {
      "label": 1,
      "quote": "pub trait SourceReader: Debug + Send + Sync {"
    },
    {
      "label": 2,
      "quote": "impl SourceReader for DiskSourceReader {"
    }
  ]
}
````

- [1] src/compiler/source_reader.rs:5-30 (code)
- [2] src/compiler/mod.rs:236-255 (code)

Correct requested trait/default. The frozen extra API detail is not stated, and the implementation quote is assigned to the unrelated preprocess_source excerpt instead of label 1.

Rubric issue: The frozen second fact requires the tokio::fs::read API although the question asks only for the abstraction and default implementation.

Native rejection: Citation quote does not match supplied source

## v2.lpc-rs.fallback/legacy_quotes

When does the compiler retry a source path with the c extension, and what prevents an endless retry?

Expected: answered. Native source checks: False.

- Fact 1 (supported): Retry is limited to NotFound errors.
- Fact 2 (supported): It retries only when the extension is not already c.

````json
{
  "status": "answered",
  "answer": "The compiler retries a source path with the `.c` extension when the initial read fails with `ErrorKind::NotFound` and the path does not end in `.c`. This retry loop continues until the file is successfully read or an error occurs, such as `ErrorKind::PermissionDenied`, which terminates the loop and prevents an endless retry.",
  "citations": [
    {
      "label": 2,
      "quote": "if error.kind() == ErrorKind::NotFound && absolute.extension().and_then(OsStr::to_str) != Some(\"c\")"
    }
  ]
}
````

- [2] src/compiler/mod.rs:148-195 (code)

The first sentence correctly includes NotFound and the not-already-c guard, which bounds the retry. The additional PermissionDenied example is valid. The copied combined guard loses literal source wrapping and fails binding.

Native rejection: Citation quote does not match supplied source

## v2.lpc-rs.fallback/compact_quotes

When does the compiler retry a source path with the c extension, and what prevents an endless retry?

Expected: answered. Native source checks: False.

- Fact 1 (supported): Retry is limited to NotFound errors.
- Fact 2 (contradicted): It retries only when the extension is not already c.

````json
{
  "status": "answered",
  "answer": "The compiler retries a source path with the `.c` extension when the initial read fails with `ErrorKind::NotFound`. An endless retry is prevented because the retry loop terminates if the file extension is not `.c`, as indicated by the condition `&& absolute.extension().and_then(OsStr::to_str) != Some(\"c\")`. This ensures the loop exits after attempting to read a non-C file.",
  "citations": [
    {
      "label": 1,
      "quote": "match reader.read(absolute).await.map(LpcSource::from_bytes) { ... Err(error) if error.kind() == ErrorKind::NotFound &&\""
    }
  ]
}
````

- [1] src/compiler/mod.rs:151-164 (code)

Finds the actual production read_file scope but misinterprets the guard. The excerpt also ends before the final error branch; do not attribute this solely to model reasoning. The quote is fabricated/ellipsized.

Unsupported: Reverses the extension guard: it says a non-c extension terminates the loop, whereas that condition permits the extension fallback.

Native rejection: Citation quote does not match supplied source

## v2.lpc-rs.fallback/compact_ids

When does the compiler retry a source path with the c extension, and what prevents an endless retry?

Expected: answered. Native source checks: True.

- Fact 1 (supported): Retry is limited to NotFound errors.
- Fact 2 (contradicted): It retries only when the extension is not already c.

````json
{
  "status": "answered",
  "answer": "The compiler retries a source path with the `.c` extension when the initial read fails with `ErrorKind::NotFound`. An endless retry is prevented because the retry loop terminates if the file extension is not `.c`, ensuring the compiler does not loop indefinitely on non-C files.",
  "citations": [
    1
  ]
}
````

- [1] src/compiler/mod.rs:151-164 (code)

Same guard inversion as compact quotes despite canonical source binding. The selected excerpt omits the final error branch.

Unsupported: Reverses the extension guard, claiming that the retry stops when the extension is not c.

## v2.lpc-rs.fallback-test/compact_quotes

Which regression test covers stopping source retries and preserving other I/O errors?

Expected: answered. Native source checks: True.

- Fact 1 (supported): The named test is file_reads_stop_after_extension_fallback_and_preserve_other_io_errors in src/compiler/mod.rs.

````json
{
  "status": "answered",
  "answer": "The test `file_reads_stop_after_extension_fallback_and_preserve_other_io_errors` covers stopping source retries and preserving other I/O errors.",
  "citations": [
    {
      "label": 1,
      "quote": "async fn file_reads_stop_after_extension_fallback_and_preserve_other_io_errors()"
    }
  ]
}
````

- [1] src/compiler/mod.rs:2012-2034 (test)

Correctly names and quotes the actual regression test; selecting a test is appropriate for this question.

## v2.lpc-rs.fallback-test/compact_ids

Which regression test covers stopping source retries and preserving other I/O errors?

Expected: answered. Native source checks: True.

- Fact 1 (supported): The named test is file_reads_stop_after_extension_fallback_and_preserve_other_io_errors in src/compiler/mod.rs.

````json
{
  "status": "answered",
  "answer": "The test `file_reads_stop_after_extension_fallback_and_preserve_other_io_errors` covers stopping source retries and preserving other I/O errors.",
  "citations": [
    1
  ]
}
````

- [1] src/compiler/mod.rs:2012-2034 (test)

Correct test name, supported by the raw lexical test scope at label 1.

## v2.lpc-rs.fallback-test/legacy_quotes

Which regression test covers stopping source retries and preserving other I/O errors?

Expected: answered. Native source checks: True.

- Fact 1 (missing): The named test is file_reads_stop_after_extension_fallback_and_preserve_other_io_errors in src/compiler/mod.rs.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts describe Lima port history, compilation checks, and specific mudlib adaptations (like `set_heart_beat` and navigation), but do not contain any information about regression tests or the specific test case covering source retries and I/O errors.",
  "citations": []
}
````


Control retrieval supplies historical port reports and an LSP guide, so it refuses instead of identifying the test.

## v2.changing.retry-base/compact_ids

What is the configured retry_limit?

Expected: answered. Native source checks: True.

- Fact 1 (supported): retry_limit is 3.

````json
{
  "status": "answered",
  "answer": "The configured retry_limit is 3, as specified in the config.toml file at line 1-1.",
  "citations": [
    1
  ]
}
````

- [1] config.toml:1-1 (configuration)

Correct current captured value/name with a valid source reference.

## v2.changing.retry-base/legacy_quotes

What is the configured retry_limit?

Expected: answered. Native source checks: True.

- Fact 1 (supported): retry_limit is 3.

````json
{
  "status": "answered",
  "answer": "The configured retry_limit is 3.",
  "citations": [
    {
      "label": 1,
      "quote": "retry_limit = 3"
    }
  ]
}
````

- [1] config.toml:1-1 (configuration)

Correct current captured value/name with a valid source reference.

## v2.changing.retry-base/compact_quotes

What is the configured retry_limit?

Expected: answered. Native source checks: True.

- Fact 1 (supported): retry_limit is 3.

````json
{
  "status": "answered",
  "answer": "The configured retry_limit is 3.",
  "citations": [
    {
      "label": 1,
      "quote": "retry_limit = 3"
    }
  ]
}
````

- [1] config.toml:1-1 (configuration)

Correct current captured value/name with a valid source reference.

## v2.changing.retry-updated/legacy_quotes

What is the configured retry_limit?

Expected: answered. Native source checks: False.

- Fact 1 (supported): retry_limit is 7.

````json
{
  "status": "answered",
  "answer": "The configured retry_limit is 7, as explicitly defined in config.toml:1-1.",
  "citations": []
}
````


Correct current captured value/name, but the updated answer omits citations or the rename answer invents a second source label.

Native rejection: An answer needs cited evidence

## v2.changing.retry-updated/compact_quotes

What is the configured retry_limit?

Expected: answered. Native source checks: True.

- Fact 1 (supported): retry_limit is 7.

````json
{
  "status": "answered",
  "answer": "7",
  "citations": [
    {
      "label": 1,
      "quote": "retry_limit = 7"
    }
  ]
}
````

- [1] config.toml:1-1 (configuration)

Correct current captured value/name with a valid source reference.

## v2.changing.retry-updated/compact_ids

What is the configured retry_limit?

Expected: answered. Native source checks: True.

- Fact 1 (supported): retry_limit is 7.

````json
{
  "status": "answered",
  "answer": "The configured retry_limit is 7.",
  "citations": [
    1
  ]
}
````

- [1] config.toml:1-1 (configuration)

Correct current captured value/name with a valid source reference.

## v2.changing.renamed-handler/compact_quotes

What function is the packet loading handler now, and where is it defined?

Expected: answered. Native source checks: True.

- Fact 1 (supported): load_packet is defined in src/transport.rs.

````json
{
  "status": "answered",
  "answer": "The packet loading handler is `load_packet`, defined in `src/transport.rs` at line 2.",
  "citations": [
    {
      "label": 1,
      "quote": "pub fn load_packet() {}"
    }
  ]
}
````

- [1] src/transport.rs:2-2 (code)

Correct current captured value/name with a valid source reference.

## v2.changing.renamed-handler/compact_ids

What function is the packet loading handler now, and where is it defined?

Expected: answered. Native source checks: True.

- Fact 1 (supported): load_packet is defined in src/transport.rs.

````json
{
  "status": "answered",
  "answer": "The packet loading handler is `load_packet`, defined in `src/transport.rs` at line 2.",
  "citations": [
    1
  ]
}
````

- [1] src/transport.rs:2-2 (code)

Correct current captured value/name with a valid source reference.

## v2.changing.renamed-handler/legacy_quotes

What function is the packet loading handler now, and where is it defined?

Expected: answered. Native source checks: False.

- Fact 1 (supported): load_packet is defined in src/transport.rs.

````json
{
  "status": "answered",
  "answer": "The packet loading handler is defined as a function named `load_packet` located in the file `src/transport.rs` at line 1-2.",
  "citations": [
    {
      "label": 1,
      "quote": "/// The packet loading handler."
    },
    {
      "label": 2,
      "quote": "pub fn load_packet() {}"
    }
  ]
}
````

- [1] src/transport.rs:1-2 (code)
- [2] unavailable label

Correct current captured value/name, but the updated answer omits citations or the rename answer invents a second source label.

Native rejection: Citation label is unavailable

## v2.changing.deleted-handler/compact_ids

Where is load_packet currently defined?

Expected: insufficient_evidence. Native source checks: True.


````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts do not contain the definition of the `load_packet` function or any code where it is defined. The repository snapshot and the question text alone do not provide the source code or file locations required to answer this.",
  "citations": []
}
````


Correctly refuses to invent a definition after the handler source file is removed.

## v2.changing.deleted-handler/legacy_quotes

Where is load_packet currently defined?

Expected: insufficient_evidence. Native source checks: True.


````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts do not contain the definition of the `load_packet` function or its location within the repository.",
  "citations": []
}
````


Correctly refuses to invent a definition after the handler source file is removed.

## v2.changing.deleted-handler/compact_quotes

Where is load_packet currently defined?

Expected: insufficient_evidence. Native source checks: True.


````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts do not contain the definition of the `load_packet` function or its location within the repository.",
  "citations": []
}
````


Correctly refuses to invent a definition after the handler source file is removed.

## v2.vizzi.fresh-review-startup/legacy_quotes

Does opening the review page start live indexing of the whole workspace?

Expected: answered. Native source checks: False.

- Fact 1 (supported): A focused review can open without starting the live source index.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts do not explicitly state whether opening the review page starts live indexing of the whole workspace. While [STARTUP_PRODUCT.md](docs/prototype/STARTUP_PRODUCT.md) mentions that opening `/review` serves an interface without starting live discovery, and [STARTUP_RESULTS.md](experiments/review-workflow/STARTUP_RESULTS.md) notes that focused review can open without waiting for a whole-workspace analysis, it does not definitively confirm if the review page itself initiates live indexing of the entire workspace or if live indexing is only started upon opening the exploration page (`/`).",
  "citations": [
    {
      "label": 1,
      "quote": "Review uses the same source policy, logical workspace, analysis cache and aggregate parser-worker budget. No separate..."
    }
  ]
}
````

- [1] docs/prototype/STARTUP_PRODUCT.md:1-29 (policy)

The body states the negative startup fact but then refuses the actual yes/no question. Also fabricates/ellipsizes the quote. Record factual content separately from the wrong status and contradictory caveat.

Unsupported: Claims the excerpts do not establish the review-only/live-index distinction while explicitly quoting that distinction.

Native rejection: Citation quote does not match supplied source

## v2.vizzi.fresh-review-startup/compact_quotes

Does opening the review page start live indexing of the whole workspace?

Expected: answered. Native source checks: True.

- Fact 1 (supported): A focused review can open without starting the live source index.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts do not confirm whether opening the review page starts live indexing. While [1] states that opening `/review` 'permits explicit scoped comparison without first capturing the live workspace' and 'does not start live discovery, watchers, periodic reconciliation', it does not explicitly state that the review page itself initiates the indexing process. The distinction between the review interface and the live workspace is described, but the direct causal link between opening the review page and starting live indexing is not established in the text.",
  "citations": []
}
````


Repeats the required negative fact, then refuses; the supplied guide and contract are sufficient.

Unsupported: Claims the causal distinction is not established despite the explicit review-only rule and guide statement without starting the live index.

## v2.vizzi.fresh-review-startup/compact_ids

Does opening the review page start live indexing of the whole workspace?

Expected: answered. Native source checks: True.

- Fact 1 (supported): A focused review can open without starting the live source index.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts state that opening `/review` serves the interface and permits scoped comparison without starting live discovery, watchers, or reconciliation. However, the text does not explicitly confirm whether the `/review` page itself initiates live indexing of the entire workspace, only that it does not start them. The distinction between the review page and the exploration page (`/`) is unclear in the text, as both are described as using the same source policy and budget, but the specific action of opening `/review` is not definitively linked to starting live indexing in the available snippets.",
  "citations": [
    1
  ]
}
````

- [1] docs/prototype/STARTUP_PRODUCT.md:4-16 (policy)

Correct negative fact appears in the body, but the status and subsequent caveat contradict the available evidence.

Unsupported: Claims the review/exploration distinction is unclear despite the explicit contract and guide statements.

## v2.vizzi.fresh-header-language/compact_quotes

How is an ambiguous .h header treated when C-family source is selected but no language assignment is provided?

Expected: answered. Native source checks: True.

- Fact 1 (supported): It is retained source-only with language unassigned.
- Fact 2 (missing): The language is not guessed from the nearest C/C++ source or majority language.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts describe the classification of `.h` files as 'Source-only: C/C++ language is unassigned' (Source Policy) and note that an explicit assignment can override a supported suffix hint, but they do not specify the exact behavior or outcome when C-family source is selected without an explicit language assignment.",
  "citations": []
}
````


Acknowledges source-only/unassigned classification but fails the task and does not establish the no-nearest-file/majority-guess rule.

Unsupported: Claims no exact outcome is supplied even though the policy table explicitly describes an enabled include/shared.h without assignment.

## v2.vizzi.fresh-header-language/compact_ids

How is an ambiguous .h header treated when C-family source is selected but no language assignment is provided?

Expected: answered. Native source checks: True.

- Fact 1 (supported): It is retained source-only with language unassigned.
- Fact 2 (missing): The language is not guessed from the nearest C/C++ source or majority language.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts describe the classification of source-only files and the meaning of 'language_unassigned' for headers like `.h`, but they do not contain specific examples or rules detailing the behavior when C-family source is selected with an ambiguous `.h` header and no language assignment provided.",
  "citations": []
}
````


Acknowledges the source-only classification in reported context, then refuses and omits the no-guessing behavior.

Unsupported: Claims there are no specific examples/rules, overlooking the explicit ambiguous-header example and suffix table.

## v2.vizzi.fresh-header-language/legacy_quotes

How is an ambiguous .h header treated when C-family source is selected but no language assignment is provided?

Expected: answered. Native source checks: False.

- Fact 1 (supported): It is retained source-only with language unassigned.
- Fact 2 (supported): The language is not guessed from the nearest C/C++ source or majority language.

````json
{
  "status": "answered",
  "answer": "An ambiguous .h header is classified as source-only with the reason `language_unassigned`. This occurs because the policy does not automatically assign a language to headers; it only assigns languages to files with explicit suffixes or profiles. Ambiguous headers remain unassigned until an explicit assignment is provided or the file is excluded.",
  "citations": [
    {
      "label": 3,
      "quote": "Ambiguous headers, unassigned fragments, and CUDA remain source-only."
    },
    {
      "label": 4,
      "quote": "An explicit assignment can override a supported suffix hint, for example C++ for a `.c` input."
    }
  ]
}
````

- [3] docs/prototype/SOURCE_POLICY.md:1-31 (policy)
- [4] crates/vizzi/tests/c_family.rs:24-70 (test)

Correct source-only/unassigned behavior and no automatic header assignment. The quote assigned to the test excerpt belongs to policy text instead, so source binding fails.

Native rejection: Citation quote does not match supplied source

## v2.vizzi.fresh-cache-source/compact_ids

Where does preview source text come from, and is the analysis cache authoritative for it?

Expected: answered. Native source checks: True.

- Fact 1 (missing): CapturedSource owns immutable captured source bytes and source identity.
- Fact 2 (missing): Analysis cache data is not authoritative source text.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts do not specify where preview source text originates or confirm if the analysis cache is authoritative for it. While `Cache` is described as native JSON persistence, the specific role of the cache regarding preview text and its authority are not explicitly defined in the supplied text.",
  "citations": []
}
````


The origin evidence is incomplete, but the cache-authority statement is explicitly available and overlooked.

Unsupported: Claims cache authority is not explicitly defined despite the Cache documentation saying it is never authoritative source text.

## v2.vizzi.fresh-cache-source/legacy_quotes

Where does preview source text come from, and is the analysis cache authoritative for it?

Expected: answered. Native source checks: True.

- Fact 1 (contradicted): CapturedSource owns immutable captured source bytes and source identity.
- Fact 2 (supported): Analysis cache data is not authoritative source text.

````json
{
  "status": "answered",
  "answer": "Preview source text comes from the indexed working file, not the analysis cache. The cache is not authoritative; it only stores checksummed records of analysis results, not the source text itself. The cache's `load` method reads JSON files containing schema, checksum, and payload, but the actual source code is retrieved from the working directory via the editor adapter.",
  "citations": [
    {
      "label": 1,
      "quote": "Native JSON persistence; cache data is never authoritative source text."
    },
    {
      "label": 2,
      "quote": "cache data is never authoritative source text."
    }
  ]
}
````

- [1] crates/vizzi/src/cache.rs:36-83 (code)
- [2] crates/vizzi/src/cache.rs:1-59 (code)

Correctly rejects the analysis cache as authoritative, but gives the wrong source ownership/acquisition mechanism despite valid cache quotes.

Unsupported: Says preview source is retrieved from the current working directory through the editor adapter instead of immutable captured source.

## v2.vizzi.fresh-cache-source/compact_quotes

Where does preview source text come from, and is the analysis cache authoritative for it?

Expected: answered. Native source checks: True.

- Fact 1 (missing): CapturedSource owns immutable captured source bytes and source identity.
- Fact 2 (supported): Analysis cache data is not authoritative source text.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts do not specify where preview source text originates or confirm if the analysis cache is authoritative for it. While `Cache` is described as native JSON persistence, the text explicitly states that 'cache data is never authoritative source text' and that 'Deferral is distinct from source_only'. The specific mechanism for generating preview text and the cache's role in it remains undefined in the supplied data.",
  "citations": []
}
````


No captured-source ownership answer; the correct cache-authority negative appears inside an otherwise contradictory refusal.

Unsupported: Claims the excerpts do not confirm cache authority, then quotes the explicit statement that the cache is never authoritative.

## v2.vizzi.fresh-working-input/legacy_quotes

Which staged, unstaged and untracked changes are included in a working-tree comparison?

Expected: answered. Native source checks: False.

- Fact 1 (contradicted): The working capture includes staged, unstaged and non-ignored untracked source observed on disk.

````json
{
  "status": "answered",
  "answer": "A working-tree comparison includes staged, unstaged, and non-ignored untracked source files as observed on disk, excluding staged and unstaged changes from the working tree itself.",
  "citations": [
    {
      "label": 1,
      "quote": "The working capture includes staged, unstaged and non-ignored untracked source as observed on disk."
    }
  ]
}
````

- [1] docs/how-to/review-local-changes.md:1-32 (guide)

The first clause is right, but the final exclusion contradicts it. The literal quote also flattens a line break.

Unsupported: After correctly listing included edits, explicitly says staged and unstaged changes are excluded from the working tree comparison.

Native rejection: Citation quote does not match supplied source

## v2.vizzi.fresh-working-input/compact_quotes

Which staged, unstaged and untracked changes are included in a working-tree comparison?

Expected: answered. Native source checks: False.

- Fact 1 (supported): The working capture includes staged, unstaged and non-ignored untracked source observed on disk.

````json
{
  "status": "answered",
  "answer": "A working-tree comparison includes tracked edits/deletions, staged edits, unstaged edits, and non-ignored untracked additions as observed on disk.",
  "citations": [
    {
      "label": 2,
      "quote": "The working capture includes staged, unstaged and non-ignored untracked source as observed on disk."
    }
  ]
}
````

- [2] docs/how-to/review-local-changes.md:1-17 (guide)

Correctly lists staged/unstaged edits, tracked deletions and non-ignored untracked source as observed on disk. The copied guide quote fails exact wrapping.

Native rejection: Citation quote does not match supplied source

## v2.vizzi.fresh-working-input/compact_ids

Which staged, unstaged and untracked changes are included in a working-tree comparison?

Expected: answered. Native source checks: True.

- Fact 1 (supported): The working capture includes staged, unstaged and non-ignored untracked source observed on disk.

````json
{
  "status": "answered",
  "answer": "The working-tree comparison includes tracked edits/deletions, staged edits, unstaged edits, and non-ignored untracked additions as observed on disk.",
  "citations": [
    1,
    2,
    3
  ]
}
````

- [1] docs/prototype/COMPARISON_PRODUCT.md:12-23 (policy)
- [2] docs/how-to/review-local-changes.md:1-17 (guide)
- [3] docs/plans/product-milestones.md:128-141 (guide)

Complete and supported working-input description; all three supplied documents support the answer.

## v2.lpc-rs.fresh-source-decoding/compact_quotes

How are non-UTF-8 source bytes decoded before compilation, and what happens to a missing trailing newline?

Expected: answered. Native source checks: False.

- Fact 1 (contradicted): LpcSource falls back to Latin-1, mapping each byte to a code point and setting latin1.
- Fact 2 (supported): A trailing newline is appended only when missing.

````json
{
  "status": "answered",
  "answer": "Non-UTF-8 source bytes are decoded by converting the bytes into a string using `String::from_utf8`, which returns an error if the input is not valid UTF-8. If decoding fails, the bytes are converted to a string of ASCII characters. A missing trailing newline is handled by appending a newline character (`\\n`) to the text if it does not already end with one.",
  "citations": [
    {
      "label": 1,
      "quote": "pub fn from_bytes(bytes: Vec<u8>) -> Self { let (mut text, latin1) = match String::from_utf8(bytes) { Ok(text) => (text,"
    },
    {
      "label": 1,
      "quote": "Ok(text) => (text, false), Err(e) => (e.into_bytes().into_iter().map(|b| b as char).collect(), true,"
    }
  ]
}
````

- [1] lpc-rs-utils/src/lib.rs:23-38 (code)
- [1] lpc-rs-utils/src/lib.rs:23-38 (code)

The complete short decoding method is supplied. Newline handling is right; decoding semantics and quote copying are wrong.

Unsupported: Describes invalid UTF-8 bytes as converted to ASCII, whereas byte-to-char mapping preserves the Latin-1 byte values including 128-255.

Native rejection: Citation quote does not match supplied source

## v2.lpc-rs.fresh-source-decoding/compact_ids

How are non-UTF-8 source bytes decoded before compilation, and what happens to a missing trailing newline?

Expected: answered. Native source checks: True.

- Fact 1 (contradicted): LpcSource falls back to Latin-1, mapping each byte to a code point and setting latin1.
- Fact 2 (supported): A trailing newline is appended only when missing.

````json
{
  "status": "answered",
  "answer": "Non-UTF-8 source bytes are decoded by converting them to a string using `String::from_utf8`, which returns an error if the bytes are invalid UTF-8. The error is converted to a string representation, and the original bytes are collected into a `Vec<u8>`. If the resulting string does not end with a newline character, one is appended to the string before returning the `LpcSource`.",
  "citations": [
    1
  ]
}
````

- [1] lpc-rs-utils/src/lib.rs:23-38 (code)

Correct newline handling, incorrect interpretation of the fully supplied Latin-1 fallback.

Unsupported: Claims an error string representation is used and bytes are collected into Vec<u8>; the code recovers original bytes and maps them directly to characters collected into a String.

## v2.lpc-rs.fresh-source-decoding/legacy_quotes

How are non-UTF-8 source bytes decoded before compilation, and what happens to a missing trailing newline?

Expected: answered. Native source checks: False.

- Fact 1 (supported): LpcSource falls back to Latin-1, mapping each byte to a code point and setting latin1.
- Fact 2 (supported): A trailing newline is appended only when missing.

````json
{
  "status": "answered",
  "answer": "Non-UTF-8 source bytes are decoded as Latin-1, and a missing trailing newline is appended automatically.",
  "citations": [
    {
      "label": 1,
      "quote": "pub struct LpcSource { ... pub latin1: bool, }"
    },
    {
      "label": 1,
      "quote": "/// Decode UTF-8 or Latin-1 bytes and ensure a trailing newline."
    },
    {
      "label": 1,
      "quote": "if !text.ends_with('\\n') { text.push('\\n'); }"
    }
  ]
}
````

- [1] lpc-rs-utils/src/lib.rs:11-70 (mixed)
- [1] lpc-rs-utils/src/lib.rs:11-70 (mixed)
- [1] lpc-rs-utils/src/lib.rs:11-70 (mixed)

Correct Latin-1 fallback and newline normalization. Ellipsized/flattened code quotes are not literal and fail source binding.

Native rejection: Citation quote does not match supplied source

## v2.lpc-rs.fresh-driver-config-failure/compact_ids

What does the gamedriver executable do if building its configuration fails?

Expected: answered. Native source checks: True.

- Fact 1 (missing): It emits diagnostics and exits with status 1.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts do not contain information about the behavior of the `gamedriver` executable when its configuration fails to build. While the code shows that `ConfigBuilder::build` is called and errors are caught with `and_then(ConfigBuilder::build)?`, the specific handling of the resulting error state for the `gamedriver` executable is not described in the supplied text.",
  "citations": [
    1,
    2,
    3
  ]
}
````

- [1] lpc-rs-driver/src/main.rs:17-48 (code)
- [2] lpc-rs-lpcc/src/main.rs:29-60 (code)
- [3] lpc-rs-lpcc/src/main.rs:85-105 (code)

The supplied gamedriver function explicitly emits diagnostics and exits 1. The response instead discusses the different lpcc question-mark path.

Unsupported: Claims the driver error handling is not described despite the complete Err branch at label 1.

## v2.lpc-rs.fresh-driver-config-failure/legacy_quotes

What does the gamedriver executable do if building its configuration fails?

Expected: answered. Native source checks: True.

- Fact 1 (missing): It emits diagnostics and exits with status 1.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts describe the gamedriver installation and usage but do not contain information about the behavior of the gamedriver executable if its configuration fails during building.",
  "citations": []
}
````


Refuses on control excerpts that do not show the driver error branch.

## v2.lpc-rs.fresh-driver-config-failure/compact_quotes

What does the gamedriver executable do if building its configuration fails?

Expected: answered. Native source checks: False.

- Fact 1 (missing): It emits diagnostics and exits with status 1.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts do not contain information about the behavior of the `gamedriver` executable when its configuration fails to build. While the code shows that `ConfigBuilder::build` is called and errors are caught, it does not specify what actions are taken if the build fails (e.g., whether the program exits, logs an error, or attempts a fallback).",
  "citations": [
    {
      "label": 1,
      "quote": "let config = match built { Ok(c) => c, Err(e) => { e.emit_diagnostics(); std::process::exit(1); } };"
    }
  ]
}
````

- [1] lpc-rs-driver/src/main.rs:17-48 (code)

The full driver failure branch is available. Its narrative refuses the task; copied code in a citation is not treated as an affirmative answer and also fails literal wrapping.

Unsupported: Claims no failure action is specified even while its own citation contains emit_diagnostics and process::exit(1).

Native rejection: Citation quote does not match supplied source

## v2.lpc-rs.fresh-relative-source-bases/legacy_quotes

Which relative-path bases does CompilerSource keep for include directives and inheritance?

Expected: answered. Native source checks: False.

- Fact 1 (missing): include_cwd is derived using root.source_cwd for relative includes.
- Fact 2 (missing): inheritance_cwd is derived using root.inheritance_cwd for relative inheritance.

````json
{
  "status": "answered",
  "answer": "CompilerSource keeps the `include_cwd` and `inheritance_cwd` paths as relative-path bases for include directives and inheritance, respectively. These are stored as `PathBuf` fields within the `CompilerSource` struct, allowing relative path resolution during compilation.",
  "citations": [
    {
      "label": 1,
      "quote": "pub struct CompilerSource {\n    resolved: ResolvedPath,\n    input: Arc<LpcPath>,\n    program: Arc<LpcPath>,\n    name: \\\""
    },
    {
      "label": 2,
      "quote": "/// The base for relative include directives.\n    pub fn include_cwd(&self) -> &Path {\n        &self.include_cwd\n    }"
    }
  ]
}
````

- [1] src/compiler/source.rs:1-61 (code)
- [2] src/compiler/source.rs:52-101 (mixed)

Correct include_cwd/inheritance_cwd names, roles and PathBuf fields. Strict scoring retains the additional derivation details as missing; quotes are also malformed.

Rubric issue: The frozen facts additionally specify root.source_cwd/root.inheritance_cwd derivation helpers, while the question asks which stored relative bases are kept.

Native rejection: Citation quote does not match supplied source

## v2.lpc-rs.fresh-relative-source-bases/compact_quotes

Which relative-path bases does CompilerSource keep for include directives and inheritance?

Expected: answered. Native source checks: True.

- Fact 1 (missing): include_cwd is derived using root.source_cwd for relative includes.
- Fact 2 (missing): inheritance_cwd is derived using root.inheritance_cwd for relative inheritance.

````json
{
  "status": "answered",
  "answer": "CompilerSource keeps `include_cwd` for relative include directives and `inheritance_cwd` for relative inherit paths.",
  "citations": [
    {
      "label": 1,
      "quote": "/// The base for relative include directives."
    },
    {
      "label": 1,
      "quote": "/// The source directory used as the base for relative inherit paths."
    }
  ]
}
````

- [1] src/compiler/source.rs:48-65 (code)
- [1] src/compiler/source.rs:48-65 (code)

Correctly answers the asked field/role mapping with valid getter-doc citations. It does not state the extra root.source_cwd/root.inheritance_cwd derivation details, so strict frozen scoring fails.

Rubric issue: The frozen facts additionally specify construction helper names that the question does not ask for.

## v2.lpc-rs.fresh-relative-source-bases/compact_ids

Which relative-path bases does CompilerSource keep for include directives and inheritance?

Expected: answered. Native source checks: True.

- Fact 1 (missing): include_cwd is derived using root.source_cwd for relative includes.
- Fact 2 (missing): inheritance_cwd is derived using root.inheritance_cwd for relative inheritance.

````json
{
  "status": "answered",
  "answer": "CompilerSource keeps `include_cwd` for relative include directives and `inheritance_cwd` for relative inherit paths.",
  "citations": [
    1,
    2
  ]
}
````

- [1] src/compiler/source.rs:48-65 (code)
- [2] src/compiler/source.rs:1-28 (code)

Same correct requested field/role mapping as compact quotes, with canonical references; preserves the strict extra derivation requirement as missing.

Rubric issue: The frozen facts additionally specify construction helper names that the question does not ask for.

## v2.lpc-rs.fresh-compiled-port/compact_quotes

Is the LPC __PORT__ constant fixed when a program is compiled, or looked up from a different runtime configuration each time?

Expected: answered. Native source checks: False.

- Fact 1 (supported): It is fixed by compilation; a later VM runtime port does not replace the compiled integer.

````json
{
  "status": "answered",
  "answer": "The LPC __PORT__ constant is fixed when a program is compiled and retains its literal value until recompiled, regardless of runtime configuration changes.",
  "citations": [
    {
      "label": 1,
      "quote": "Each compiled object retains its literal value until recompiled, including when executed with a different runtime config"
    }
  ]
}
````

- [1] doc/lpc/macros.md:65-77 (guide)

Correct compile-time literal behavior supported by the macro reference, but the copied quote is not a literal supplied substring.

Native rejection: Citation quote does not match supplied source

## v2.lpc-rs.fresh-compiled-port/compact_ids

Is the LPC __PORT__ constant fixed when a program is compiled, or looked up from a different runtime configuration each time?

Expected: answered. Native source checks: True.

- Fact 1 (supported): It is fixed by compilation; a later VM runtime port does not replace the compiled integer.

````json
{
  "status": "answered",
  "answer": "The LPC __PORT__ constant is fixed when a program is compiled and retains its literal value until recompiled, regardless of runtime configuration changes.",
  "citations": [
    1
  ]
}
````

- [1] doc/lpc/macros.md:65-77 (guide)

Correctly states compile-time capture and retention until recompilation, exactly supported by the supplied macro reference.

## v2.lpc-rs.fresh-compiled-port/legacy_quotes

Is the LPC __PORT__ constant fixed when a program is compiled, or looked up from a different runtime configuration each time?

Expected: answered. Native source checks: False.

- Fact 1 (supported): It is fixed by compilation; a later VM runtime port does not replace the compiled integer.

````json
{
  "status": "answered",
  "answer": "The LPC __PORT__ constant is fixed when a program is compiled and retains its literal value even when executed with a different runtime configuration.",
  "citations": [
    {
      "label": 1,
      "quote": "Each compiled object retains its literal value until recompiled, including when executed with a different runtime config"
    },
    {
      "label": 2,
      "quote": "A configured zero expands to zero, even if a later listener binds an ephemeral port"
    }
  ]
}
````

- [1] doc/lpc/macros.md:55-93 (guide)
- [2] src/compiler/mod.rs:704-753 (test)

Correct compile-time retention across runtime configurations; exact copied quotes fail wrapping/label matching.

Native rejection: Citation quote does not match supplied source
