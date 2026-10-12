# WO-M4-10 pre-change evidence (recorded by WO-GAP-M4-10)

Test file: `tests/svg/shape/test_decode_budget.c` (CTest `shape-decode-budget`).

## (a) over-16-MiB decode fails at the parent base `5894cb4`

Built from `git archive 5894cb4` under `.cache/WO-GAP-M4-10/base` with the new test file and its CMake target added.
That tree has no token-budget entry (`SHAPE_MAX_TOKEN_BYTES` is undefined there), so (b)-(d) are compiled out by `#ifdef` and only (a), which uses the public `svg_shape_decode`, runs.
Result: fails for the right reason, the old 16 MiB source cap (`limit` at offset 16777216).

```
FAIL: (a) over-16-MiB document decodes
  error=limit offset=16777216
ok: (a) document exceeds 16 MiB
note: token budget entry absent (pre-change tree); (b)-(d) skipped
FAILED
exit=1
```

## (b) tiny token budget maps to XML before the mapping fix

Tree: current working tree with the test-only budget entry and the new tests added but the stream-failure mapping still the original
(`decode.c` as at `27e7781`/HEAD: any nonempty XML diagnostic maps to `SVG_SHAPE_ERROR_XML`). Output:

```
FAIL: (b) tiny budget fails with LIMIT
  got xml
FAIL: (d) failure path is LIMIT
ok: (a) document exceeds 16 MiB
ok: (a) over-16-MiB document decodes
ok: (a) all 1000 shapes preserved
ok: (b) source built
ok: (b) diagnostic category matches
ok: (b) offset is nonzero and within the source
ok: (b) failure leaves the document empty
ok: (c) anonymous mapping of 1 GiB + 1 created
ok: (c) 1 GiB + 1 source is LIMIT
ok: (d) source built
ok: (d) success path decodes
ok: (d) hook saw allocations on success path
ok: (d) live bytes zero after destroy (success)
ok: (d) live bytes zero after failure
FAILED
exit=1
```

(b) and the failure half of (d) fail with `SVG_SHAPE_ERROR_XML` instead of `SVG_SHAPE_ERROR_LIMIT`.
After the mapping change the same binary passes (`./dev verify`, `check-04.txt`).

## (c) and (d) are guards, not regression tests

(c) 1 GiB + 1 source refused with LIMIT, and (d) live bytes returning to zero on the success path and after the failure, already pass before the change (see the `ok:` lines above for the pre-fix tree). They are recorded as guards; no pre-change failure is claimed for them.
