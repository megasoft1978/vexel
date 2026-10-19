# WO-RUST-MIGRATION-INVENTORY-01 result

Evidence-only inventory candidate. Source evidence and migration references were not modified. All candidate changes are under this artifact directory.

## Methods and scope

- The canonical 603-path set comes from `artifacts/WO-RUST-PLAN-01/source-baseline.json` and `complete-inventory.json` in `legacy-c-final` (`cb9661144907e909a0e7950243e9dc3fcfefb5bc`). The accepted manifests contain the same 603 paths and each SHA-256 matches its tag blob. `dev` is an executable launcher and is included; the non-code `resources/en.lproj/Localizable.strings` path is not in either canonical manifest.
- The canonical per-file line records sum to 137,570, matching newline and `splitlines()` counts for all 603 tag blobs. The five `.inc` paths contribute 1,113 lines. `dev` has blob `d4fec76f6d36ff995d6cd0db5f8ecbd0ea6ac6d5` and 290 lines.
- The accepted complete inventory excludes `generated`, `vendor/cache/build`, `worktrees/clones`, and `artifacts`. The five tag-wide C/Objective-C paths found under archived `artifacts/` are listed individually in `summary.json` and validated against the tag. Current maintained source-like exclusions are separately listed path by path.
- All 115 Python baseline paths have individual roles, treatment, and dispositions in `summary.json`; each remains `pending_port` as either test tooling, benchmark tooling, report/evidence tooling, development/workflow tooling, integration test tooling, or packaging tooling. The retained current Python subset remains listed separately (9 baseline paths, 3 additions). Shell tooling, resources, and native/FFI reference paths retain their separate classifications.
- `inventory_source_sha` identifies the fresh verifier source-tree base (`fbc5191bda8a67026044135e47e9d1309b451211`) whose first-party paths and contents this inventory describes; it does not identify this candidate commit. The original additions scan source is recorded separately as `inventory_enumeration_source_sha` (`6051c42d8708a8ba5533cb8ab985b2a2f9750780`). All 1007 current-addition blob IDs are identical at both revisions.

## Counts

`summary.json` records 603 unique baseline code paths, all `pending_port`, and 1007 current additions (703 code/tooling rows and 304 separately classified asset rows). Baseline paths, blob IDs, SHA-256 values and line counts are validated against the accepted tag evidence. No pending or unresolved path counts as complete.

## Verification

1. `python3 scripts/ledger.py validate ledger.jsonl` — exit 0; `valid ledger: 2087 entries`.
2. `git diff --check` — exit 0; no output.
3. Executable inventory validation — exit 0. It checks the accepted tag manifests, every baseline blob/SHA-256/line count, the 603-path and 137,570-line totals, five `.inc` paths/1,113 lines, `dev` metadata, current additions and Python accounting, and exact path-level exclusions.

```sh
python3 - <<'PY'
import hashlib, json, subprocess
from collections import Counter
from pathlib import Path, PurePosixPath
root=Path('artifacts/WO-RUST-MIGRATION-INVENTORY-01')
run=lambda args,**kw: subprocess.check_output(args,**kw)
allowed={'accepted_rust_replacement','pending_port','historical_data','unresolved'}
source=json.loads(run(['git','show','legacy-c-final:artifacts/WO-RUST-PLAN-01/source-baseline.json'],text=True))
complete=json.loads(run(['git','show','legacy-c-final:artifacts/WO-RUST-PLAN-01/complete-inventory.json'],text=True))
source_hashes=source['files']; complete_rows={r['path']:r for r in complete['first_party_code_files']}
baseline=[json.loads(line) for line in (root/'baseline.jsonl').read_text().splitlines()]
additions=[json.loads(line) for line in (root/'current-additions.jsonl').read_text().splitlines()]
assert len(baseline)==603 and len({r['path'] for r in baseline})==603
assert all(r.get('kind','baseline_code')=='baseline_code' for r in baseline)
assert set(source_hashes)==set(complete_rows)=={r['path'] for r in baseline}
assert all(r['disposition'] in allowed for r in baseline+additions)
tree={line.split('\t',1)[1]:line.split()[2] for line in run(['git','ls-tree','-r','legacy-c-final'],text=True).splitlines()}
sha_matches=line_matches=blob_matches=0
for row in baseline:
    blob_id=tree[row['path']]
    assert row['blob_id']==blob_id
    data=run(['git','cat-file','blob',blob_id])
    sha_matches += hashlib.sha256(data).hexdigest()==source_hashes[row['path']]
    actual_lines=data.count(b'\n')
    assert actual_lines==len(data.splitlines())==complete_rows[row['path']]['lines']==row['lines']
    line_matches += 1
    blob_matches += 1
assert sha_matches==line_matches==blob_matches==603
assert sum(r['lines'] for r in baseline)==137570
incs={r['path'] for r in baseline if r['path'].endswith('.inc')}
assert len(incs)==5 and sum(r['lines'] for r in baseline if r['path'] in incs)==1113
launch=next(r for r in baseline if r['path']=='dev')
assert launch['blob_id']=='d4fec76f6d36ff995d6cd0db5f8ecbd0ea6ac6d5' and launch['lines']==290
s=json.loads((root/'summary.json').read_text())
assert s['baseline']['rows']==603 and s['baseline']['code_rows']==603
assert s['baseline']['line_count_newline_delimited']==137570
assert s['baseline']['canonical_sha256_matches']==603 and s['baseline']['canonical_inventory_line_records_matched']==603
assert {r['path'] for r in s['baseline_python_inventory']}=={r['path'] for r in baseline if r['path'].endswith('.py')}
assert len(s['baseline_python_inventory'])==115 and all(r['role'] and r['treatment'] and r['disposition']=='pending_port' for r in s['baseline_python_inventory'])
assert len(additions)==s['current_additions']['rows']
asset_suffixes={'.svg','.png','.icns','.ppm','.rgba','.rle','.gif','.jpg','.jpeg','.webp','.tiff','.bmp'}
asset_roots=('src/','app/','rust/','resources/','bench/','tests/')
tracked_assets={p for p in run(['git','ls-files'],text=True).splitlines() if p.startswith(asset_roots) and PurePosixPath(p).suffix.lower() in asset_suffixes}
asset_rows=[r for r in additions if r['kind']=='shader_or_asset']
assert {r['path'] for r in asset_rows}==tracked_assets
assert all(r.get('role') and r['disposition']=='historical_data' for r in asset_rows)
assert len(asset_rows)==s['current_additions']['separate_asset_rows']==304
assert s['current_additions']['asset_path_count']==len(tracked_assets)==len(asset_rows)
assert dict(sorted(Counter(r['kind'] for r in additions).items()))==s['current_additions']['kinds']
assert dict(sorted(Counter(r['disposition'] for r in additions).items()))==s['current_additions']['dispositions']
curpy=s['python_tooling_inventory']; baseline_paths={r['path']:r['disposition'] for r in baseline}; add_paths={r['path']:r['disposition'] for r in additions}
assert len(curpy)==12 and sum(r['scope']=='baseline' for r in curpy)==9 and sum(r['scope']=='addition' for r in curpy)==3
assert all(Path(r['path']).is_file() for r in curpy)
assert all((r['scope']=='baseline' and baseline_paths.get(r['path'])==r['disposition']) or (r['scope']=='addition' and add_paths.get(r['path'])==r['disposition']) for r in curpy)
tag_archives=sorted(p for p in tree if p.startswith('artifacts/') and p.endswith(('.c','.m')))
expected_archives=sorted(r['path'] for r in s['baseline']['excluded_tag_artifact_source_paths'])
assert tag_archives==expected_archives and len(tag_archives)==5
legacy_resources=sorted(p for p in tree if p.endswith('.strings'))
expected_resources=sorted(r['path'] for r in s['baseline']['excluded_noncode_resources'])
assert legacy_resources==expected_resources==['resources/en.lproj/Localizable.strings']
source_tree={line.split('\t',1)[1]:line.split()[2] for line in run(['git','ls-tree','-r','6051c42d8708a8ba5533cb8ab985b2a2f9750780'],text=True).splitlines()}
verify_tree={line.split('\t',1)[1]:line.split()[2] for line in run(['git','ls-tree','-r','fbc5191bda8a67026044135e47e9d1309b451211'],text=True).splitlines()}
assert all(source_tree.get(r['path'])==verify_tree.get(r['path']) for r in additions)
assert s['inventory_source_sha']=='fbc5191bda8a67026044135e47e9d1309b451211'
assert s['inventory_enumeration_source_sha']=='6051c42d8708a8ba5533cb8ab985b2a2f9750780'
assert s['inventory_verification_base_sha']==s['inventory_source_sha']
assert s['inventory_revision_evidence']['addition_blob_ids_equal_at_enumeration_and_verifier_base']==1007
tracked=run(['git','ls-files'],text=True).splitlines()
suffixes={'.rs','.py','.sh','.c','.h','.m','.inc','.swift','.mm','.cpp','.cc','.js','.ts','.tsx','.jsx','.go','.rb','.pl'}
current_source={p for p in tracked if PurePosixPath(p).suffix.lower() in suffixes}
excluded=sorted(current_source-(baseline_paths.keys()|add_paths.keys()))
assert excluded==sorted(r['path'] for r in s['current_additions']['excluded_code_paths'])
assert len(excluded)==s['current_additions']['excluded_code_paths_count']==2
print('PASS canonical_paths=603 source_sha256=603/603; lines=137570 per_file_matches=603/603; baseline_blob_ids=603/603; inc=5 lines=1113; dev_blob=d4fec76f6d36ff995d6cd0db5f8ecbd0ea6ac6d5 lines=290; additions=1007 (code=703 assets=304); assets=304 tracked paths with roles and historical_data dispositions; baseline_python=115 roles/treatments/dispositions=verified; retained_python=12 (baseline=9 additions=3); enumeration_head=6051c42d8708a8ba5533cb8ab985b2a2f9750780; inventory_source_head=fbc5191bda8a67026044135e47e9d1309b451211; addition_blobs_equal=1007/1007; archived_artifact_exclusions=5; current_path_exclusions=2')
PY
```

Output:

```text
PASS canonical_paths=603 source_sha256=603/603; lines=137570 per_file_matches=603/603; baseline_blob_ids=603/603; inc=5 lines=1113; dev_blob=d4fec76f6d36ff995d6cd0db5f8ecbd0ea6ac6d5 lines=290; additions=1007 (code=703 assets=304); assets=304 tracked paths with roles and historical_data dispositions; baseline_python=115 roles/treatments/dispositions=verified; retained_python=12 (baseline=9 additions=3); enumeration_head=6051c42d8708a8ba5533cb8ab985b2a2f9750780; inventory_source_head=fbc5191bda8a67026044135e47e9d1309b451211; addition_blobs_equal=1007/1007; archived_artifact_exclusions=5; current_path_exclusions=2
```

The complete artifact change remains below the 250,000-byte verifier input limit. `inventory_source_sha` identifies the verifier source-tree base described by the inventory, not the candidate commit. The original enumeration source is recorded separately; candidate commits contain evidence artifacts only.

## Limitations

No accepted per-file parity map could be substantiated for all paths; no accepted Rust replacement is claimed. Historical tool scripts remain pending port and do not count as completed Rust replacements.
