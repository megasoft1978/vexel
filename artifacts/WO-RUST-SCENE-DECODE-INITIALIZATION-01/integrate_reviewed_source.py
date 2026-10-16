from pathlib import Path
import hashlib,json,subprocess,shutil
r=Path(__file__).resolve().parents[2];a=r/'artifacts/WO-RUST-SCENE-DECODE-INITIALIZATION-01';t=r/'.cache/WO-RUST-RENDER-INTEGRATION-CONTRACTS-01/worker';base=Path('rust/crates/vexel-scene');h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
for f in ['process-status.json','review-status.json','viewport-recheck-status.json']:
 d=json.loads((a/f).read_text());assert d['status']=='completed';assert not subprocess.run(['ps','-p',str(d['child_pid']),'-o','comm='],capture_output=True,text=True).stdout.strip()
assert (a/'viewport-recheck.md').read_text().lstrip().startswith('**ACCEPT');assert json.loads((a/'accepted-input-terminal-audit.json').read_text())['verdict']=='PASS'
full=json.loads((a/'review-source-hashes.json').read_text());delta=json.loads((a/'viewport-review-diff.json').read_text());assert set(delta['changed'])=={str(base/'README.md'),str(base/'tests/decode_initialization.rs'),str(base/'src/decode_initialization.rs')} and not delta['new']
for name,sha in full.items():
 if name not in delta['changed']:assert h(t/name)==sha,name
snap=json.loads((a/'viewport-recheck-source-hashes.json').read_text())
for name,sha in snap.items():assert h(t/name)==sha,name
oldtests=[]
for crate in ['vexel-scene','vexel-svg-animation']:
 for p in (r/'rust/crates'/crate/'tests').rglob('*'):
  if p.is_file():assert h(p)==h(t/p.relative_to(r)),p;oldtests.append(str(p.relative_to(r)))
for name in ['validation_tests.rs','paint_publication_tests.rs']:
 p=r/base/'src'/name;assert h(p)==h(t/p.relative_to(r))
# No animation source change was necessary; explicitly verify entire existing crate.
for p in (r/'rust/crates/vexel-svg-animation').rglob('*'):
 if p.is_file():assert h(p)==h(t/p.relative_to(r)),p
arc=a/'main-before';arc.mkdir(exist_ok=False);shutil.copytree(r/base,arc/'crate');shutil.copytree(t/'artifacts/WO-RUST-SCENE-DECODE-INITIALIZATION-01',a/'candidate-reports');copied={}
for p in (t/base/'src').rglob('*'):
 if p.is_file():
  rel=p.relative_to(t);dst=r/rel;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dst);copied[str(rel)]=h(dst)
for rel in [base/'README.md',base/'tests/decode_initialization.rs']:
 if rel.suffix=='.rs':assert not (r/rel).exists()
 shutil.copy2(t/rel,r/rel);copied[str(rel)]=h(r/rel)
(a/'integration-source-hashes.json').write_text(json.dumps({'copied':copied,'prior_test_files_byte_preserved':oldtests,'unchanged_animation_crate':True,'full_review':'review.md','viewport_delta_recheck':'viewport-recheck.md','root_confirmed_exact_diff':delta,'fresh_main_required':True},indent=2)+'\n');print(json.dumps({'copied_files':len(copied),'prior_test_files_byte_preserved':len(oldtests),'animation_source_changed':False,'fresh_main_required':True}))
