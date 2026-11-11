# Vizzi documentation

Vizzi is a local workspace for reading source in connected views. The call workspace
keeps earlier functions visible while you follow a call or explore a branch.

## Start here

1. [Build and install Vizzi](how-to/install.md).
2. [Follow a Python call trail](tutorials/python-call-trail.md) in the included example.
3. [Run Vizzi on your own project](how-to/run-workspace.md) and
   [configure semantic navigation](how-to/configure-language-servers.md).

Prefer Rust? [Follow a Rust call trail](tutorials/first-call-trail.md) covers the same
workflow with a small Cargo project and source-local method navigation.

## Complete a task

- [Follow calls, callers and references](how-to/follow-a-call-trail.md)
- [Arrange bubbles, groups and saved tasks](how-to/manage-tasks.md)
- [Follow saved changes or pin a view](how-to/follow-live-changes.md)
- [Configure language servers and local dependencies](how-to/configure-language-servers.md)
- [Browse files and declarations](how-to/browse-source.md)
- [Review working changes or two Git commits](how-to/review-local-changes.md)
- [Save source locations and use VS Code](how-to/investigate-with-vscode.md)
- [Troubleshoot a first run](how-to/troubleshoot.md)

## Look something up

- [Command-line options](reference/cli.md)
- [Language support and analysis limits](reference/languages.md)
- [Views, controls and keyboard shortcuts](reference/workspace.md)
- [Storage and persistence](reference/storage.md)

## Understand the behaviour

- [Call destinations, candidates and references](explanation/call-destinations.md)
- [Live source, pinned views and saved layouts](explanation/source-and-layout.md)

Vizzi is an early preview. The [language reference](reference/languages.md) distinguishes
current capabilities from unavailable analysis; the [changelog](../CHANGELOG.md) records
notable changes. Editing, refactoring and debugging remain in your editor.

## Develop Vizzi

[Contributing](../CONTRIBUTING.md) and the [contributor index](development/README.md)
cover implementation, verification, design contracts and historical experiments.
