# Dependency JavaScript mutation limits

Contributor reference. The broad run covers 1,169 mutations: 1,079 assertion kills, 88 survivors and two timeouts. The focused follow-up covers origin validation, read retry and changed model ranges: 382 mutations, 354 assertion kills, 26 survivors and two timeouts. These scopes overlap; counts must not be added. No exclusions improve the score.

The focused origin validator catches all 59 generated mutations. Remaining findings concern timer/fallback-reason cleanup and backoff budgets, plus model recovery/candidate/link guards. Some alternate guard paths produce equivalent current state, but the report does not classify all survivors as equivalent. They remain behavioral/resource coverage limits. The broader model run also contains existing layout, restore and live-refresh behavior beyond this feature.

A final small follow-up adds bodyless responses, custom cancellation reasons, and round-trip validation after revisiting a recovered dependency. Its raw report is retained separately. Timeouts (including retry-loop removal) are not assertion kills. DOM/native adapters are covered by interface/browser tests, not mutated by this configuration. Coverage analysis is off; this is not line coverage.

## Focused unresolved findings

| File | ID | Outcome | Line | Replacement |
| --- | --- | --- | --- | --- |
| read-retry.js | 2 | Survived | 1 | `{}` |
| read-retry.js | 3 | Survived | 1 | `;` |
| read-retry.js | 5 | Survived | 1 | `signal.reason && new DOMException('Aborted', 'AbortError')` |
| read-retry.js | 6 | Survived | 1 | `""` |
| read-retry.js | 7 | Survived | 1 | `""` |
| read-retry.js | 10 | Survived | 1 | `""` |
| read-retry.js | 13 | Survived | 1 | `""` |
| read-retry.js | 14 | Survived | 1 | `{}` |
| read-retry.js | 15 | Survived | 1 | `false` |
| read-retry.js | 17 | Survived | 1 | `false` |
| read-retry.js | 19 | Survived | 1 | `;` |
| read-retry.js | 29 | Timeout | 4 | `{}` |
| read-retry.js | 32 | Timeout | 4 | `false` |
| read-retry.js | 40 | Survived | 4 | `attempt > 5` |
| read-retry.js | 42 | Survived | 4 | `response.body.cancel` |
| read-retry.js | 43 | Survived | 4 | `50 / 2 ** attempt` |
| visual-model.js | 190 | Survived | 35 | `true` |
| visual-model.js | 192 | Survived | 35 | `s.origin &#124;&#124; existing.state === 'unresolved'` |
| visual-model.js | 193 | Survived | 35 | `true` |
| visual-model.js | 268 | Survived | 45 | `true` |
| visual-model.js | 287 | Survived | 47 | `!target.origin && v.snapshot === this.snapshot` |
| visual-model.js | 289 | Survived | 47 | `false` |
| visual-model.js | 294 | Survived | 47 | `true` |
| visual-model.js | 358 | Survived | 51 | `true` |
| visual-model.js | 360 | Survived | 51 | `this.links[exact].state != null &#124;&#124; this.links[exact].state !== 'current'` |
| visual-model.js | 361 | Survived | 51 | `true` |
| visual-model.js | 363 | Survived | 51 | `true` |
| visual-model.js | 365 | Survived | 51 | `""` |

See [the full focused report](validation/js-focused.json.gz) and [broad report](validation/js-broad.json.gz) for every generated mutation.

The closure follow-up covers 61 cases: 49 assertion kills, two timeouts and ten survivors
(nine in read-retry, one in the selected model line). It catches the bodyless-response
and revisited-link failures that the earlier fixtures missed. Remaining retry timing,
fallback-reason and cleanup alternatives are not proven by the current assertions.
See [js-closure.json.gz](validation/js-closure.json.gz); this overlaps earlier results.
