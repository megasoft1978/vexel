# Handoff

WO-M3-05's proceed/pause/pivot/stop owner decision remains OPEN. M4/M5 have not started.

WO-M3-06 revision 2 is machine-complete on branch `wo/WO-M3-06`; independent read-only review is pending. The 58-file external corpus is pinned by manifest and baseline hashes in `reports/M3/head-to-head.json`; its SVG files remain outside this repository. The 116 fresh Release trials found 2 supported previews, 56 unsupported outcomes, no limits or crashes. The 100 MiB first-preview wedge fails; max main-thread step and the per-file RSS allowance pass. Exact metrics, limitations, and checks are in `reports/M3/head-to-head.md` and `reports/M3/head-to-head-validation.txt`.

Next: review the committed candidate with `./scripts/wo-verify.sh WO-M3-06 <candidate-sha>`. If the fresh review passes and changed-path scope is clean, record acceptance, fast-forward merge to `main`, update this handoff with the owner gate, and stop. Keep all M4/M5 work stopped pending owner review.
