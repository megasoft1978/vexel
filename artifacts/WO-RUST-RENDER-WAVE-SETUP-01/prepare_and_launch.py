from pathlib import Path
import os,json,subprocess,shutil,hashlib,time
from datetime import datetime,timezone
import sys
root=Path(__file__).resolve().parents[2];sys.path.insert(0,str(root))
from scripts.ledger import append_entry
setup=root/'artifacts/WO-RUST-RENDER-WAVE-SETUP-01'
parts=json.loads((setup/'partitions.json').read_text())
assert 'ACCEPT' in (setup/'review.md').read_text()
entries=[json.loads(line) for line in (root/'ledger.jsonl').read_text().splitlines()]
assert any(e['wo_id']=='WO-RUST-RENDER-WAVE-SETUP-01' and e['event']=='accepted' for e in entries)
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
files=set()
for name in ['app','src','tests','docs','work','rust','scripts','bench','packaging','resources','.cargo']:
 start=root/name
 if not start.exists():continue
 for parent,dirs,names in os.walk(start):
  dirs[:]=[d for d in dirs if d not in ['.git','.cache','build','target','__pycache__','node_modules']]
  for name in names:
   if name!='.DS_Store':files.add(str((Path(parent)/name).relative_to(root)))
for p in root.iterdir():
 if p.is_file() and p.name!='.DS_Store':files.add(p.name)
for name in ['ownership-matrix.json','shared-contracts.json','assertion-assignments.json','limits-features.json','accepted-api-signatures.json','coordinator-acceptance.json','source-api-inventory.json','dependency-dag.json']:
 files.add('artifacts/WO-RUST-RENDER-CONTRACT-PLAN-01/'+name)
for name in ['result.md','api-coverage.json','scope-audit.json','recheck-v1-result.md']:
 files.add('artifacts/WO-RUST-KERNEL-CONTRACTS-01/'+name)
for name in ['result.md','recheck.md','verification.json','scope-audit.json']:
 files.add('artifacts/WO-RUST-SCENE-DRAFT-01/'+name)
for p in parts:files.add('artifacts/'+p['wo_id']+'/scope.json')
registry=[]
for part in parts:
 wo=part['wo_id'];own=root/'artifacts'/wo;tree=root/'.cache'/wo/'worker';tree.parent.mkdir(parents=True,exist_ok=True)
 if tree.exists():
  assert wo=='WO-RUST-SVG-DECODE-01' and not (own/'process-status.json').exists(), 'refuse existing or live worker'
  assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=tree,text=True).strip()==head
 existing_events={e['event'] for e in entries if e['wo_id']==wo}
 for event,status in [('ready','ready'),('run_started','running')]:
  if event in existing_events:continue
  append_entry(root/'ledger.jsonl',{'ts':datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),'wo_id':wo,'event':event,'revision':1,'base_sha':head,'head_sha':head,'run_id':wo+'-'+event,'session_id':'render-source-wave-2026-10-14','model':'gpt-6-luna','cli_version':'0.162.1','duration_s':0,'usage':{'input_tokens':None,'output_tokens':None},'status':status,'artifacts':['work/'+wo+'.md','artifacts/'+wo+'/scope.json','artifacts/'+wo+'/overlay-baseline.json'],'note':'Full accepted owner partition, actual isolated Standard Luna'+part['effort'].capitalize()+' source writer against accepted shared/draft contracts. Protect all manifests/peers/603/157/index; no fake/partial engine, GUI or extra billing. Candidate requires independent review/audit/main scoped checks.'})
 if not tree.exists():subprocess.run(['git','worktree','add','--detach',str(tree),head],cwd=root,check=True,capture_output=True)
 hashes={}
 for rel in sorted(files):
  dest=tree/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(root/rel,dest);hashes[rel]=hashlib.sha256((root/rel).read_bytes()).hexdigest()
 for name in ['.cache/cargo','.cache/tmp','build/rust']:(tree/name).mkdir(parents=True,exist_ok=True)
 shutil.copytree(root/'.cache/cargo/registry',tree/'.cache/cargo/registry')
 out=tree/'artifacts'/wo;out.mkdir(parents=True,exist_ok=True)
 (own/'overlay-baseline.json').write_text(json.dumps({'root':str(root),'worktree':str(tree),'head':head,'files':hashes},indent=2)+'\n')
 prompt=('Implement ENTIRE frozen work/'+wo+'.md in this isolated current-source tree. Read OWNER/AGENTS/order and immutable artifacts/'+wo+'/scope.json first. Own only rust/crates/'+part['crate']+'/src/**, new tests/**, README and own artifacts except scope.json. Protect all manifests/locks/config/shared/peers/old tests/source603/157oracles/previous orders/index/state/docs/ledgers. Catalog full source/privatehelper/algorithm/features/original assertion scope, not smaller first pass. Shared plan historical proposal spelling is superseded by current accepted vexel-scene APIs and real sceneDraft gate; source-only data/plans not mutable context borrows. Use only accepted dependencies, no unfinishedpeer/defaultfake/provider/empty-success or TODO required engine. Later coordinator supplies actual DecodeOperations/KernelResources/FlatKernel adapters. All owned functions public where needed for real composition; native providers explicit actual implementation downstream, fixture injection only in tests. Source assignment at append time and safe actual capacity leases replace C ownership hooks, not loose mutable escape/RSS claims. Actual algorithms/limits/errors/source offsets/work/fallback/math/rounding preserved. Std-only safe Rust; no unsafe/network/download/install/GPL/unsupportedOS/Cshim/stage/commit/reset/delete/recursiveworkers/GUI/broadUIE2E. Keep eachsource/test<800. Report exact accepted-contract blocker before replacing semantics; root repairs sharedonly. Run scoped lockedoffline check/test and WO validation; port assigned original pure assertions meaningfully, preserve full mixed-oracle input and identify later integration proof. Output complete result.md<=400words/parity-map.json for every catalog symbol/algorithm/coverage.json original cases/verification.json exact evidence. Standard Luna'+part['effort'].capitalize()+' existing subscription only, no credits/APIbilling, stop quota/rate errors. Successful exit onlycandidate, root independent review/exact scope/protected audit/mainfocusedchecks before acceptance. Use scoped reads, avoid huge logs.\n')
 (own/'worker.prompt.txt').write_text(prompt)
 runner='''from pathlib import Path
import os,json,subprocess,time
root=Path(__file__).resolve().parents[2];own=root/'artifacts'/%r;tree=root/'.cache'/%r/'worker';out=tree/'artifacts'/%r;started=time.time();env=os.environ.copy()
for key in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(key,None)
env.update(CARGO_HOME=str(tree/'.cache/cargo'),TMPDIR=str(tree/'.cache/tmp'),CARGO_TARGET_DIR=str(tree/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
status={'pid':os.getpid(),'status':'running','model':'gpt-6-luna','effort':%r,'worktree':str(tree),'started_epoch':started};p=own/'process-status.json';p.write_text(json.dumps(status,indent=2)+'\\n')
cmd=['codex','exec','--ignore-user-config','--profile','economy','--model','gpt-6-luna','-c',%r,'-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--color','never','--json','--cd',str(tree),'--output-last-message',str(out/'last-message.md'),'-']
with (own/'worker.prompt.txt').open('rb') as stdin,(own/'worker.events.jsonl').open('wb') as stdout,(own/'worker.stderr.log').open('wb') as stderr:
 child=subprocess.Popen(cmd,stdin=stdin,stdout=stdout,stderr=stderr,env=env);status['child_pid']=child.pid;p.write_text(json.dumps(status,indent=2)+'\\n');code=child.wait()
status.update(status='completed' if code==0 else 'failed',exit_code=code,duration_seconds=time.time()-started);p.write_text(json.dumps(status,indent=2)+'\\n')
'''%(wo,wo,wo,part['effort'],'model_reasoning_effort="'+part['effort']+'"')
 (own/'runner.py').write_text(runner)
 with (own/'supervisor.log').open('wb') as log:process=subprocess.Popen([sys.executable,str(own/'runner.py')],cwd=root,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
 registry.append(dict(part,runner_pid=process.pid,worktree=str(tree),root_artifacts=str(own.relative_to(root))))
 (setup/'worker-registry.json').write_text(json.dumps(registry,indent=2)+'\n')
 print('Launched',wo,'Luna',part['effort'],'runner',process.pid,flush=True)
print('All eight isolated full-scope writers launched.',flush=True)
