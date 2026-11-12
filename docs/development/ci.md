# Continuous integration

Contributor reference for the workflows in `.github/workflows/`.

| Workflow | Trigger | Checks |
| --- | --- | --- |
| CI | Pull requests, pushes to `main`/`master`, manual dispatch | Stable Rust formatting, workspace tests/doctests and Clippy with all features on Linux and Apple Silicon macOS; Linux browser and four-language tutorial checks; release helper tests |
| Verify action lockfile | Pull requests, branch pushes, weekly schedule, manual dispatch | Online action provenance and complete lock coverage |
| Release | `v[0-9]*` tag pushes | Version validation, native checks, four platform builds, executable smoke tests, complete checksummed bundle and immutable publication |

Release targets are Linux x86-64/ARM64 with musl and macOS Intel/Apple Silicon. Linux
archives contain a static executable. macOS executables are ad-hoc signed; Developer ID
signing/notarization is not configured. The workflow must pass on each native runner
before an archive is published; authoring the matrix does not itself qualify those platforms.

The native checks share `scripts/ci/check-native.sh`. Tests run serially with a symlinked
temporary directory to exercise canonical-path contracts on both platforms. All test
targets run even after a failure so one failing binary cannot hide later failures.
Hosted compiler jobs use two tasks; local contributor and mutation limits remain in
AGENTS.md. Browser checks use the locked npm development dependencies and installed
Playwright Chromium.
The browser job also runs the VS Code plugin's deterministic Node protocol/provider tests.
The isolated real VS Code extension-host check remains a separate local integration check.
Optional real-language-server integrations remain opt-in because they need separately
installed tools and dependency source. Performance/mutation runs remain separate from
normal correctness CI.

The retired `vizzi-qa` experiment tool is outside the active Cargo workspace and CI/release
checks. Its source history and reproduction procedure are in the
[question-experiment archive](../../experiments/qa/README.md). The app still uses and tests
the shared `vizzi-search` library.

## Action dependencies

The [gh-actions-lock skill](../../.agents/skills/gh-actions-lock/SKILL.md) governs updates.
The workflows use the latest stable action tags verified when this change was prepared:
checkout 7.0.1, setup-node/setup-python 7.1.0, setup-ruby 1.338.0, rust-cache 2.9.3,
upload-artifact 7.0.2 and download-artifact 8.0.2. `actions.lock` records their commits
and repository identities. There were no transitive `uses:` dependencies in these manifests.
Rust is installed with rustup rather than an additional action.

After changing any workflow dependency:

```sh
gh actions-lock
gh actions-lock --verify-local
gh actions-lock --verify
```

Commit the generated lockfile and any workflow rewrites together. Use `--relock` to
advance an already-recorded ref deliberately. Never edit `actions.lock` manually or use
`--accept-moved` in automation. Offline verification checks coverage only; online CI also
checks upstream provenance. See [GitHub's lockfile reference](https://docs.github.com/en/actions/concepts/security/workflow-dependency-lockfiles).

## Release protections

Build jobs have read-only repository access. Only the final publication job can write
release contents, after every matrix build and the browser/tutorial checks have passed. It checks that the existing tag
still identifies the built commit and that the commit is on the default branch.
Every archive must contain unchanged copies of the tagged source's project licenses.
Archives and checksums are attached to a draft; uploaded bytes are downloaded and
verified before publication. Existing releases are refused, with no asset replacement.

Repository release immutability must stay enabled. GitHub locks published assets and
tags and creates a release attestation; see [immutable releases](https://github.blog/changelog/2025-10-28-immutable-releases-are-now-generally-available/).
Its administrative status endpoint cannot be read by the normal workflow token, so the
job verifies the release's immutable flag after publication. If an administrator disables
that setting, the check fails but cannot retroactively make a mutable publication immutable.
No administrative credential is stored in CI. The setting was enabled separately during setup.

The release process is in [the maintainer guide](../how-to/release.md). No release is
created by committing these workflows; a maintainer must push a matching version tag.
