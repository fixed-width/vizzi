# Contributor documentation

For people extending Vizzi or investigating its implementation. The
[user documentation](../README.md) describes current operation; the
[contribution guide](../../CONTRIBUTING.md) covers development entry points.

## Design and implementation contracts

- [Current project direction and architecture](../../DESIGN.md)
- [Domain language](../../CONTEXT.md) and [contributor standards](../../AGENTS.md)
- [Visual workspace behaviour](../visual/PRODUCT.md) and [architecture](../visual/TECH.md)
- [Live source](../visual/LIVE_TECH.md), [Rust navigation](../visual/SEMANTIC_TECH.md),
  [dependency source](../visual/DEPENDENCIES_TECH.md), [modules](../visual/MODULES_TECH.md),
  [task working sets](../visual/WORKING_SETS_TECH.md) and [shared header](../visual/HEADER_PRODUCT.md)
- [Ruby analysis and navigation](../prototype/RUBY.md)
- [Language-server integration](../visual/LANGUAGE_SERVERS_TECH.md) and
  [detailed Rust call-resolution rules](call-resolution.md)
- [Launch configuration and editor sharing behaviour](../configuration/PRODUCT.md),
  [implementation](../configuration/TECH.md) and [VS Code plugin](../../editors/vscode/README.md)
- [Native host architecture](../prototype/TECH.md), [API](../prototype/API.md),
  [source policy](../prototype/SOURCE_POLICY.md), [comparison architecture](../prototype/COMPARISON_TECH.md)
  and [comparison API](../prototype/COMPARISON_API.md)

The existing [prototype contracts](../prototype/), [visual contracts](../visual/),
[companion contracts](../companion/), [structural trial](../structure/) and
[planning records](../plans/) retain their established paths. Some document intermediate
experiments; use DESIGN.md and the current user guides to determine the active scope.

## Verification and research

- [CI, action locking and release protections](ci.md)
- [Publish a release](../how-to/release.md)

- [Ruby validation](../../experiments/languages/RUBY_RESULTS.md)
- [Browser validation](../../experiments/browser/README.md)
- [Python/C-family language-server validation](../../experiments/visual/LANGUAGE_SERVER_RESULTS.md)
- [Configuration, VS Code sharing and VSIX validation](configuration-validation.md)
- [Reference-repository qualification](../../experiments/qualification/README.md)
- [Review fixture and walkthrough](../../experiments/review-workflow/README.md)
- [Keep local build output small](../how-to/keep-build-output-small.md)
- [Performance budgets](../performance/BUDGETS.md)
- [Archived project history and experiment index](project-history.md)

The archive preserves the former README's research and measurement links, including
source-capture/retention work, performance scopes, earlier learning/review trials and
the retired repository-question experiments. These are evidence and historical context,
not product setup instructions or current support guarantees. The [question-experiment
archive](../../experiments/qa/README.md) records how to retrieve the retired native tool
and reproduce that work. Its [contracts](../qa/) remain available; Vizzi has no current
question/chat UI.

For further design rationale, see [visual working sets](../explanation/visual-working-sets.md),
[source reflow](../explanation/source-reflow.md), [repository layouts](../explanation/repository-layouts.md)
and [C-family analysis](../explanation/c-family-analysis.md). The older
[maintainer return trial](../how-to/maintainer-return-trial.md) is a contributor protocol,
not an onboarding tutorial.
