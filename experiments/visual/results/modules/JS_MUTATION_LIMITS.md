# JavaScript mutation limits

Contributor reference for this module-cue slice. The final full helper run has **336 killed, 46 survived, one timeout, zero runner/build errors** out of 383. Coverage analysis is off; these are behavioral mutation results, not line coverage. The vendor library and DOM/worker adapters are not mutated.

Six survivors are equivalent with respect to the current contract or pinned library; they remain in the raw score. No source exclusions suppress them:

- `235`: Palette probing in the opposite direction still assigns distinct stable colours to the first six groups; direction is unspecified.
- `283`: The empty root is independently rejected by the path-component check.
- `284`: A nonnegative length guard is equivalent here; empty roots still fail the component check.
- `348`: The final whole-root candidate equals the preselected fallback, so omitting that iteration changes no label.
- `85`: Fewer than two known views cannot produce any group with two members; the per-group check still omits them.
- `148`: The pinned library already defaults pixelGroup to 4; omitting this setter leaves it unchanged.

The other **40 surviving contour mutations remain coverage gaps**, not proven equivalents. They concern raw polygon boundary validation, normalization/raster resolution, iteration budgets, smoothing and output-size/nonfinite guards. Public SVG tests cover ordinary membership, offsets, scaling, edge placement and obstacles; they do not force every malformed or degenerate vendor result. Resource budgets need separate qualification. The one-second worker stop limits UI impact but does not prove these guards correct.

Mutation `350` changes the disambiguation loop to decrement its counter and times out. It is recorded as a timeout, not a test-assertion kill.

Browser mutation activation is verified by an instrumented positive control. The earlier browser-inactive report is historical evidence only. The descriptor follow-up caught the missing language/provenance pairing case; it is also covered in this final full run.

## Remaining contour findings

| Source line | Surviving IDs | Interpretation |
| --- | --- | --- |
| 4 | 39, 43, 46, 47, 50, 53 | Boundary/degenerate polygon checks are incompletely exercised. |
| 7 | 85 | Redundant early known-count check (equivalent). |
| 8 | 89, 91, 96, 98, 99 | Normalization and raster/resource behavior need broader inputs. |
| 12 | 148, 149, 150 | One default setter is equivalent; iteration-budget reductions are not proven by functional tests. |
| 13 | 152, 153, 154, 155, 157, 158 | Raw output size bounds are not forced by the fixtures. |
| 15 | 163, 164, 165, 166, 167, 168, 170, 171, 172, 179 | Malformed/nonfinite/vendor containment guards need fault/degeneracy coverage. |
| 16 | 184 | Smoothing appearance is not fixed by membership assertions. |
| 17 | 186, 187, 188, 189, 190, 191, 193, 194, 196, 197 | Output length/nonfinite/aggregate budgets are not forced by the fixtures. |

The compressed [full report](validation/js-final.json.gz) retains every generated mutation and its outcome. These limits do not establish full mutation coverage of the feature.
