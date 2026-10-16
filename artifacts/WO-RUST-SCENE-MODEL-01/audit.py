from pathlib import Path
import hashlib,json,subprocess
root=Path.cwd();a=root/'artifacts/WO-RUST-SCENE-MODEL-01'
hashfile=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
baseline=json.loads((a/'baseline.json').read_text())
allowed_exact={'Cargo.toml','Cargo.lock','STATE.md','LEDGER.md','ledger.jsonl','docs/product/rust-port-goal-and-plan.md'}
changed=[p for p,h in baseline['files'].items() if not (root/p).is_file() or hashfile(root/p)!=h]
protected_changes=[p for p in changed if p not in allowed_exact and not p.startswith('rust/crates/vexel-scene/')]
old=json.loads((root/'artifacts/WO-RUST-PLAN-01/source-baseline.json').read_text())
legacy_changed=[p for p,h in old['files'].items() if not(root/p).is_file() or hashfile(root/p)!=h]
oracles=json.loads((root/'artifacts/WO-RUST-RENDER-CONTRACT-PLAN-01/assertion-assignments.json').read_text())
oracle_files={}
def collect(value):
 if isinstance(value,dict):
  if 'path' in value and 'sha256' in value:oracle_files[value['path']]=value['sha256']
  for child in value.values():collect(child)
 elif isinstance(value,list):
  for child in value:collect(child)
collect(oracles)
oracle_changes=[p for p,h in oracle_files.items() if not (root/p).is_file() or hashfile(root/p)!=h]
head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
index_path=subprocess.check_output(['git','rev-parse','--git-path','index'],text=True).strip()
index=hashfile(Path(index_path))
review_hash_path=a/'recheck-v2-source-hashes.json'
review_unchanged=None
if review_hash_path.exists():
 review=json.loads(review_hash_path.read_text());review_unchanged=all(hashfile(root/p)==h for p,h in review.items())
result={'status':'pass' if not protected_changes and not legacy_changed and not oracle_changes and head==baseline['head'] and index==baseline['index_sha256'] else 'fail','previous_goal_turn_classification':'no progress: verified an already-saved goal; resumed unfinished scene implementation as the next safe action','baseline_file_count':len(baseline['files']),'changed_baseline_paths':changed,'protected_changes':protected_changes,'legacy_file_count':len(old['files']),'legacy_changes':legacy_changed,'frozen_oracle_file_count':len(oracle_files),'frozen_oracle_changes':oracle_changes,'head':head,'index_sha256':index,'review_snapshot_unchanged':review_unchanged,'source_line_counts':{str(p.relative_to(root)):len(p.read_text().splitlines()) for p in (root/'rust/crates/vexel-scene').rglob('*.rs')}}
(a/'scope-audit.json').write_text(json.dumps(result,indent=2)+'\n')
assert result['status']=='pass',result
assert max(result['source_line_counts'].values())<800
print('PASS protected source/603 legacy/frozen renderer oracles/HEAD/index; all scene source and test files under800')
