"""Coordinator copy only. Acceptance occurs separately after fresh main checks."""
from pathlib import Path
import json,hashlib,subprocess,shutil
r=Path(__file__).resolve().parents[2];a=r/'artifacts/WO-RUST-PAINT-CHANNEL-CONTRACT-01';t=r/'.cache/WO-RUST-RENDER-INTEGRATION-CONTRACTS-01/worker';base=Path('rust/crates/vexel-scene')
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
s=json.loads((a/'process-status.json').read_text());v=json.loads((a/'review-status.json').read_text());assert s['status']==v['status']=='completed'
for x in (s,v):assert not subprocess.run(['ps','-p',str(x['child_pid']),'-o','comm='],capture_output=True,text=True).stdout.strip()
verdict=(a/'review.md').read_text();assert verdict.lstrip().startswith(('**ACCEPT','ACCEPT','# ACCEPT')), 'manual ACCEPT source review required'
assert json.loads((a/'accepted-input-terminal-audit.json').read_text())['verdict']=='PASS'
snapshot=json.loads((a/'review-source-hashes.json').read_text())
for name,sha in snapshot.items():assert h(t/name)==sha, name
# All accepted prior assertions remain byte-for-byte identical.
oldtests=[]
for p in (r/base/'tests').rglob('*'):
 if p.is_file():assert h(p)==h(t/p.relative_to(r)),p;oldtests.append(str(p.relative_to(r)))
for name in ['validation_tests.rs','paint_publication_tests.rs']:
 p=r/base/'src'/name;assert h(p)==h(t/p.relative_to(r))
archive=a/'main-before';archive.mkdir(exist_ok=False);shutil.copytree(r/base,archive/'crate')
shutil.copytree(t/'artifacts/WO-RUST-PAINT-CHANNEL-CONTRACT-01',a/'candidate-reports')
copied={}
for p in (t/base/'src').rglob('*'):
 if p.is_file():
  rel=p.relative_to(t);dst=r/rel;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dst);copied[str(rel)]=h(dst)
for rel in [base/'tests/paint_channel_contracts.rs',base/'README.md']:
 assert not (r/rel).exists() if rel.suffix=='.rs' else True
 shutil.copy2(t/rel,r/rel);copied[str(rel)]=h(r/rel)
(a/'integration-source-hashes.json').write_text(json.dumps({'copied':copied,'preserved_existing_test_files':oldtests,'review':'review.md','not_accepted_until_fresh_main_checks':True},indent=2)+'\n')
print(json.dumps({'copied_source_files':len(copied),'existing_test_files_byte_preserved':len(oldtests),'review':'ACCEPT source only','fresh_main_checks':'required'}))
