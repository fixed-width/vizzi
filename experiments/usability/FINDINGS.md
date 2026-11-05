# First maintainer observations

Operator role: maintainer. Viewer baseline: `66ebde8`; trial kit: `a59890e`.
Source: direct operator feedback during the prepared local learning/review trial.
These are interaction findings, not independent comprehension or productivity results.

| ID | Reported observation/request | Response to validate |
| --- | --- | --- |
| M1 | File-map tooltips end in `syntax syntax`. | One clear coverage phrase. |
| M2 | Code needs syntax colouring. | Captured, bounded colouring in both source modes. |
| M3 | Declaration kinds need immediate visual distinction; line numbers are ambiguous. | Labelled kind colours and explicit `Line N` meaning. |
| M4 | Map opens as a column; repeated panning is cumbersome. Wheel zoom alone is insufficient, and changing wheel to scroll is not the requested answer. | Balanced layout, overview navigator and explicit zoom controls. |
| M5 | Sections must be resizable. | Pointer/keyboard separators and usable narrow layouts. |
| M6 | Invalid base/path input gives no visible error. | Persistent actionable form error, including initial capture and empty path scope. |
| M7 | It is unclear whether subtree accepts a revision. | Repository-path wording and a separate target-revision input. |
| M8 | Compare any two existing commits. | Two-revision mode using pinned native revision inputs. |
| M9 | Bottom-bar text is explanatory noise. | Remove ordinary debug footer; retain useful operation/error state. |
| M10 | Shown/matching numbers appear redundant. | Collapse equal counts; retain distinct capped/page information. |

The [behaviour](../../docs/prototype/TRIAL_PRODUCT.md) and
[implementation contract](../../docs/prototype/TRIAL_TECH.md) define the response.
Implementation and automated verification are complete; see [the validation record](FEEDBACK_RESULTS.md).
A maintainer retry of the reported interactions is still pending. Preserve the operator
report separately from implementation claims and unfamiliar-user acceptance.

## Second maintainer observations

After `24d48df`, the maintainer reported revision-expression/short-ID failures in the
trial viewer, requested closable informational/error rows, found the map unintuitive,
and requested directory reorganisation/collapse, a system-default dark/light preference
and stronger syntax colours. The trial's missing history was confirmed; see
[the orientation response and validation](ORIENTATION_RESULTS.md). The literature
review and project-specific hypotheses are in [the layout explanation](../../docs/explanation/repository-layouts.md).

## Third maintainer corrections

The maintainer limited close controls to external full-width ephemeral banners, reported
vertically clipped map titles, requested better declaration structure without deep
indentation, and reported distracting readiness/progress text changes. See
[the refinement response](REFINEMENT_RESULTS.md). This supersedes the earlier broad
dismissal interpretation.
