# Review task

Review a proposed improvement to source-read diagnostics in the Rust implementation
of lpc-rs. The change should explain that lookup stopped, preserve the source name and
underlying I/O error, and preserve existing source-lookup behaviour.

Compare the supplied working tree with baseline
`1a4f5b65dd2a269298748df2dfd68092a169a8d9`, including staged edits, unstaged edits and
untracked files. Decide whether the change meets its intent. Explain any behavioural
regression you find, or why you believe the lookup rules remain intact. Cite the
before/after source and relevant unchanged contracts or tests. Record uncertainty and
any questions the views cannot answer.
