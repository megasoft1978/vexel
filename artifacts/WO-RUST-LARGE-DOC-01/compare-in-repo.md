## Comparison with the legacy C baselines

Sources (sha256 of the compared files):

- baseline: sha256 `fd9dd9a5958866a9495932a861160c76034e6f956da2656dc3a0d21f85879466`
- current Rust run: sha256 `08761f59f6e677a3e44d1549cb1b366eeac455abd541940bd0acca98286ef3db`

| category | baseline | current Rust run |
|---|---|---|
| ok | 17 | 17 |
| unsupported | 7 | 7 |
| **ok / attempted** | 17 / 24 | 17 / 24 |

Legacy `path` and `viewport` have no Rust verdict of that name. In the tables above they stay separate rows (never merged). For the per-file deltas below they are compared as `xml` (Rust reports malformed path data and viewport as `Malformed`, verdict `xml`); every such pair is printed with its raw legacy category.

### Per-file deltas vs baseline

New ok: none

Regressions: none

Changed category: none

Unchanged (including same-after-mapping): 24. Ids not in current report: 0.

