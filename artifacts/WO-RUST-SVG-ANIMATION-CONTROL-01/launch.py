from pathlib import Path
from datetime import datetime,timezone
import hashlib,json,os,shutil,subprocess,sys
root=Path(__file__).resolve().parents[2];sys.path.insert(0,str(root))
from scripts.ledger import append_entry
setup='WO-RUST-NATIVE-BINDINGS-SETUP-01';a=root/'artifacts'/'WO-RUST-SVG-ANIMATION-CONTROL-01';batch=json.loads((a/'batch.json').read_text());head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip();h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
latest={json.loads(l)['wo_id']:json.loads(l) for l in (root/'ledger.jsonl').read_text().splitlines()};assert latest[setup]['event']=='accepted';assert latest['WO-RUST-WORK-CONTROL-01']['event']=='accepted';assert latest['WO-RUST-SVG-ANIMATION-OWNED-01']['event']=='accepted'
assert all(h(root/p)==v for p,v in json.loads((root/'artifacts/WO-RUST-PLAN-01/source-baseline.json').read_text())['files'].items())
index=h(root/'.git/index')
for x in batch.values():
 wo=x['wo'];assert latest[wo]['event']=='ready'
 entry={'ts':datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),'wo_id':wo,'event':'run_started','revision':1,'base_sha':head,'head_sha':head,'run_id':wo+'-run_started-wave4','session_id':'rust-native-bindings-2026-10-14','model':'gpt-6-luna','cli_version':'0.162.1','duration_s':0,'usage':{'input_tokens':None,'output_tokens':None},'status':'running','artifacts':['work/'+wo+'.md','artifacts/'+setup+'/batch.json'],'note':'Isolated current-source overlay and actual Standard Luna '+x['effort']+' CLI writer, scoped package checks only. Candidate requires independent review and fresh main integration audit. No API billing or recursive agents.'}
 append_entry(root/'ledger.jsonl',entry)
 out=root/'artifacts'/wo;out.mkdir(exist_ok=True);(out/'run_started-entry.json').write_text(json.dumps(entry,indent=2)+'\n')
# All source/config/fixture roots including previously ignored reference files;
# preserve historical git artifacts as seeded immutable blobs, not entire logs.
rels=set(json.loads((root/'artifacts/WO-RUST-SVG-PATH-01/overlay-baseline.json').read_text())['files'])
rels={p for p in rels if not p.startswith('artifacts/') and (root/p).is_file()}
for name in ['app','src','tests','docs','work','rust','scripts','bench','reports','packaging','resources','.github','.cargo']:
 start=root/name
 if not start.exists():continue
 for parent,dirs,files in os.walk(start):
  dirs[:]=[d for d in dirs if d not in ['.git','.cache','build','.worktrees','target','__pycache__','node_modules']]
  for f in files:
   if f!='.DS_Store':rels.add(str((Path(parent)/f).relative_to(root)))
for p in root.iterdir():
 if p.is_file() and p.name!='.DS_Store':rels.add(p.name)
processes=[]
for name,x in batch.items():
 tree=Path(x['worktree']);assert not tree.exists(),tree
 r=subprocess.run(['git','worktree','add','--detach',str(tree),head],cwd=root,capture_output=True,text=True);assert r.returncode==0,r.stdout+r.stderr
 own='artifacts/'+x['wo']+'/run_started-entry.json';files=sorted(rels|{own,'artifacts/WO-RUST-PLAN-01/source-baseline.json'})
 hashes={}
 for rel in files:
  dest=tree/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(root/rel,dest);hashes[rel]=h(root/rel)
 assert all(h(tree/p)==v for p,v in hashes.items())
 for d in ['.cache/cargo','.cache/tmp','build/rust']:(tree/d).mkdir(parents=True,exist_ok=True)
 shutil.copytree(root/'.cache/cargo/registry',tree/'.cache/cargo/registry')
 out=root/'artifacts'/x['wo'];(out/'overlay-baseline.json').write_text(json.dumps({'root':str(root),'worktree':str(tree),'head':head,'root_index_sha256':index,'files':hashes},indent=2)+'\n')
 ca=tree/'artifacts'/x['wo'];ca.mkdir(exist_ok=True)
 prompt='Implement ONLY '+x['wo']+' in this isolated current-source tree. Read OWNER-DIRECTIVES.md, AGENTS.md and work/'+x['wo']+'.md before scoped source reads. Complete the ENTIRE named C scope/API, with accepted crate contracts and original assertions preserved. Your source writer owns rust/crates/'+x['crate']+'/src and tests plus artifacts/'+x['wo']+' only. Shared manifests/config/ledger/docs/peers/legacy are protected. Full goal is Rust vector/cartoon app; this work order is one dependency-ready part. No recursive agents/network/install/dependencies/C shim/new unsafe/staging/commit/delete/reset, or broad UI/E2E. Use exact installed Rust1.99 via +stable, locked offline checks/test named by order, repo-local inherited Cargo/cache/tmp. Do not install missing toolchain components. Std safe implementation, bounded allocations/checked sizes, explicit source/ownership semantics. Do not substitute smaller limits/schema or fake backend. If a real contract is missing stop with specific dependency, not convenient subset. Produce result.md <=400words, parity-map.json and verification.txt under own artifacts with exact results/adaptations; successful exit is only candidate, root separately reviews/integrates. Use existing subscription only; stop on allowance/rate error, no retryloop.\n'
 (ca/'worker.prompt.txt').write_text(prompt)
 # local ledger snapshot must include all referenced orders.
 subprocess.run(['python3','scripts/ledger.py','validate','ledger.jsonl'],cwd=tree,check=True,capture_output=True)
 with (a/(name+'-supervisor.log')).open('wb') as log:p=subprocess.Popen(['python3',str(a/'runner.py'),name],cwd=root,stdout=log,stderr=log,start_new_session=True)
 processes.append({'name':name,'wo':x['wo'],'pid':p.pid,'worktree':str(tree),'model':'gpt-6-luna','effort':x['effort'],'status':'running','overlay_file_count':len(hashes)})
 (a/'processes.json').write_text(json.dumps(processes,indent=2)+'\n')
 print(name,p.pid,'started;',len(hashes),'current source files',flush=True)
assert h(root/'.git/index')==index
(a/'launch-verification.txt').write_text('Accepted setup before launch; one isolated actualCLI processes/current-source overlays. Local ledgers valid; original source/index preserved; no application/UI checks.\n')
