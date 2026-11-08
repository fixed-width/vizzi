# Keep local build and test output small

For contributors maintaining the checkout. Generated Rust output under `target/` and
mutation/Stryker scratch copies are disposable; recorded experiment evidence and private
investigations are separate data.

Default development/test builds keep source line tables but omit full variable debug info,
and disable incremental object caches. This trades some Rust recompilation speed/debugger
variable detail for substantially smaller output. It does not change Vizzi's source-index
incrementality or optimized-build performance measurements. For a full-debug session,
explicitly override `CARGO_PROFILE_DEV_DEBUG=2 CARGO_INCREMENTAL=1` and clean it afterwards.

Use `cargo clean` to remove all workspace compiler output, or
`cargo clean --profile dev` to clear the development/test cache while retaining release
output. Use one build profile consistently where possible; old profile/flag combinations
can leave multiple copies of test executables and dependencies.

After mutation results have been archived/reviewed, remove their ignored `mutants.out*`
logs and `.stryker-tmp` sandboxes. Keep current `node_modules` when browser validation still
needs it. Avoid a broad `git clean -fdx`: private session notes and retained measurement
inputs can be ignored too. Check tracked paths and preserve recorded data before deletion.

Mutation runs can retain multiple compiled variants in the normal development target,
even when their source copies and reports live under `/tmp`. After integration checks
and archiving results, `cargo clean --profile dev` removes those variants while keeping
the optimised viewer. Avoid copying build output elsewhere merely to hide its size.
