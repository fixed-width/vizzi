# Question-to-evidence companion trial

Historical contract: the question-search browser workflow is retired. Its UI entry points
and browser modules have been removed. Native retrieval/evaluation interfaces remain
experimental infrastructure; use [the visual workspace](../how-to/follow-a-call-trail.md)
for current usage.

For the maintainer evaluating source search alongside an editor. This opt-in slice tests
navigation usefulness after the known-question ranking work. It generates no answer,
inferred call graph or automatic connection and requires no model.

Open `/investigate?trial=search` to enter the trial; ordinary discovery remains the default.
The existing theme, typography and evidence drawer are retained. One question input and
Search button replace name/file search in this view. Empty state invites a question and
offers a return to repository browsing. No source path, title or relationship form is
required to begin. Query submission is explicit, not a request on every keystroke.

Results are at most eight finite source cards grouped by their suggested roles, such as
definitions/registration, entry/use, checks and examples. Show path/line, captured scope
where available, and a short literal excerpt. Grouping is search guidance, not a claim of
resolved semantics. Inspect opens the existing captured-source drawer. Keep this location
saves exact evidence in the existing investigation; saved evidence can use the verified
VS Code handoff. No editor is launched automatically. Keep question is a separate explicit
action and never overwrites a saved question just because a search ran. Saving stays
disabled until the current investigation revision has loaded; source search/inspection
can complete while that state is still loading.

A secondary trial control compares Original matching with Focused search using the same
question/source capture. Do not display parallel input forms or duplicate result panes.
Changing the control requests results explicitly; results never move under the reader
because background ranking arrived. Saved locations remain visible through the existing view.

The scope is current captured indexed source, not an implied whole repository or README/
documentation corpus. Display searched/available file counts and omissions when bounded
search cannot include every indexed file. No matches means no matches in that scope,
not proof of absence. Unavailable captured bytes fail clearly rather than being replaced
with working-file reads. A source update marks the results stale and requires another
search before keeping a trial result. Old previews/excerpts never silently retarget.

Use one bounded query operation; superseded responses cannot replace newer results.
Errors outside the panes use existing dismissible messages. Stale/empty/partial-scope
states inside results remain until the reader acts. Preserve keyboard operation, focus,
system/explicit theme choice, narrow layouts and reduced-motion behaviour. No new design
mock is supplied: reuse Noto Sans for controls/headings and the existing source monospace.
Base palette remains paper #ffffff, ink #183044, muted #526477, surface #eaf0f7 and
accent #265dc5, with the existing dark-theme equivalents. Structural role headings and
source text provide hierarchy; no decorative card counters, gradients or new motion.

Freeze fresh general/codebase-specific trial questions before searching them. Prefer
maintainer-authored questions where supplied; otherwise label source-aware agent-authored
cases accurately. Keep them out of retrieval tuning. Record whether useful starting points
were present, inspection/keep/handoff actions, navigation effort, source gaps and query cost.
Automated browser replay establishes function, not independent maintainer acceptance.
The eight earlier reserved real-source cases remain unrun.

Success for this slice is a concrete question → source inspection → retained investigation
→ verified editor workflow, with stale/partial states exercised and a reviewable trial
package. Fresh-question usefulness and model integration are separate decisions. Stop
owned test servers/browser instances and use recording-only editor adapters in verification.
