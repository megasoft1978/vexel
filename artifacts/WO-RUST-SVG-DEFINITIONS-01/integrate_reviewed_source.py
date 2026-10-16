from pathlib import Path
import json,hashlib,shutil,subprocess
r=Path(__file__).resolve().parents[2];a=r/'artifacts/WO-RUST-SVG-DEFINITIONS-01';t=r/'.cache/WO-RUST-SVG-DEFINITIONS-01/worker';base=Path('rust/crates/vexel-svg-definitions');h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
for name in ['marker-repair-status.json','marker-recheck-status.json','full-review-status.json']:
 d=json.loads((a/name).read_text());assert d['status']=='completed';assert not subprocess.run(['ps','-p',str(d['child_pid']),'-o','comm='],capture_output=True,text=True).stdout.strip()
assert (a/'marker-recheck.md').read_text().lstrip().startswith('**ACCEPT')
assert json.loads((a/'accepted-input-terminal-audit.json').read_text())['verdict']=='PASS'
full=json.loads((a/'full-review-source-hashes.json').read_text());delta=json.loads((a/'marker-review-diff.json').read_text());assert delta['existing_changed']==['rust/crates/vexel-svg-definitions/src/marker.rs'] and delta['new']==['rust/crates/vexel-svg-definitions/tests/marker_dimension_errors.rs']
for name,sha in full.items():
 if name not in delta['existing_changed']:assert h(t/name)==sha,name
snapshot=json.loads((a/'marker-recheck-source-hashes.json').read_text())
for name,sha in snapshot.items():assert h(t/name)==sha,name
arc=a/'main-before';arc.mkdir(exist_ok=False);shutil.copytree(r/base,arc/'crate');shutil.copytree(t/'artifacts/WO-RUST-SVG-DEFINITIONS-01',a/'candidate-reports')
copied={}
for folder in ['src','tests']:
 for p in (t/base/folder).rglob('*'):
  if p.is_file():
   rel=p.relative_to(t);dst=r/rel;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dst);copied[str(rel)]=h(dst)
if (t/base/'README.md').exists():shutil.copy2(t/base/'README.md',r/base/'README.md');copied[str(base/'README.md')]=h(r/base/'README.md')
(a/'integration-source-hashes.json').write_text(json.dumps({'copied':copied,'full_source_review':'full-review.md','only_marker_delta_after_full_review':delta,'scoped_marker_recheck':'marker-recheck.md','root_confirmed_audit_and_diff':True,'fresh_main_checks_required':True},indent=2)+'\n')
print(json.dumps({'source_files_copied':len(copied),'protected_hashes':1685,'composite_review':'fullLunaHigh+markerLunaLow; rootexacthash/diff','fresh_main_required':True}))
