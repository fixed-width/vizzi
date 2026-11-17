# Function inventory finding

Published as [issue #12](https://github.com/fixed-width/vizzi/issues/12).

While qualifying #5 on Vizzi `d9fefdf42344fde91b674b4c4087cb91439917a2` and PyTorch `1f4d2d5e820dd870626dca4f9603e3a49879dbde`, full source capture succeeds but an explicit function inventory for a captured file returns no matches. The source browser's paged outline contains the requested declaration.

Observed input uses the full raw-language policy in `experiments/qualification/profiles.json` (PyTorch), four analysis workers, no language server and no project build. Initial capture contains 10,620 files / 186,250,450 source bytes. Both Python targets have syntax coverage.

Reproduction after source is ready:

```json
{"snapshot":"<current /api/status snapshot>","query":{"kind":"inventory","area":"torch/_dynamo/convert_frame.py","search":"_compile","offset":0,"functions_only":true}}
```

POST that request to `/api/structure`. `matching` is 0. The same request scoped to `torch/nn/modules/module.py` and `_call_impl` also returns 0. Obtain each captured file/version through `/api/view?projection=summary&filter=<path>`, then page `/api/window` with those exact pins: `_compile` is outline row 61 (lines 1711–2391), and `Module._call_impl` is row 60 (lines 1791–1923).

Inventory coverage reports `{"available_files": 10616, "indexed_files": 4096, "omitted_areas": 0, "omitted_calls": 364644, "omitted_files": 6520, "omitted_symbols": 14907, "partial_files": 1084, "unavailable_call_files": 411}`. `Structure::with_module_cache` admits the first 4,096 files globally and bounds symbols before applying the inventory's area/search. Selecting an omitted file therefore cannot bring its functions into the index. This prevents Add bubble from finding these functions; native history/comparison endpoints can still use their captured outline identities.

Expected: an explicitly selected captured file/area can supply function lookup within bounded work, while coverage continues to describe omitted global results. Raising global caps alone is not a requirement. Keep cross-file historical resolution outside this work (#7).

Suggested validation: a captured repository beyond 4,096 files and a separate symbol-limit case, with a function in the explicitly selected omitted file. Confirm it can be found/opened without dropping source identity checks or claiming complete global coverage.

The qualification report and raw repeated observations are being added under `experiments/function-history/` for #5. This issue concerns reachability through the live inventory, not a failed history operation or a synthetic latency budget.
