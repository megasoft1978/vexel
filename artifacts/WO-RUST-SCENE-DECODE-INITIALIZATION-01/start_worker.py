from pathlib import Path
import hashlib,json,subprocess,sys,shutil,os
from datetime import datetime,timezone
r=Path(__file__).resolve().parents[2];w='WO-RUST-SCENE-DECODE-INITIALIZATION-01';a=r/'artifacts'/w;t=r/'.cache/WO-RUST-RENDER-INTEGRATION-CONTRACTS-01/worker';out=t/'artifacts'/w;out.mkdir(exist_ok=False);h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip()
latest=None
for l in (r/'ledger.jsonl').read_text().splitlines():
 d=json.loads(l)
 if d['wo_id']=='WO-RUST-PAINT-CHANNEL-CONTRACT-01':latest=d
assert latest['event']=='accepted' and latest['revision']==3
for f in ['process-status.json','review-status.json']:
 s=json.loads((r/'artifacts/WO-RUST-PAINT-CHANNEL-CONTRACT-01'/f).read_text());assert s['status']=='completed';assert not subprocess.run(['ps','-p',str(s['child_pid']),'-o','comm='],capture_output=True,text=True).stdout.strip()
for crate in ['vexel-scene','vexel-svg-animation']:
 for p in (r/'rust/crates'/crate).rglob('*'):
  if p.is_file() and (p.relative_to(r/'rust/crates'/crate).parts[0] in ('src','tests') or p.name in ('README.md','Cargo.toml')):assert h(p)==h(t/p.relative_to(r)),p
shutil.copy2(r/'work'/f'{w}.md',t/'work'/f'{w}.md')
for name in ['coordinator-result.md','main-verification.json']:
 shutil.copy2(r/'artifacts/WO-RUST-PAINT-CHANNEL-CONTRACT-01'/name,out/('accepted-channel-'+name))
proof={'prerequisite':'WO-RUST-PAINT-CHANNEL-CONTRACT-01 r3 accepted','main_scene_checks':156,'original_animation_checks':21,'latest_ledger_accepted':latest,'prior_writers_and_reviewer_terminal':True,'current_source_equal_to_accepted_main':True,'historical_candidate_reports':'Prior reports are preserved history; the coordinator acceptance above is current authority.'};(out/'accepted-prerequisite.json').write_text(json.dumps(proof,indent=2)+'\n');shutil.copy2(out/'accepted-prerequisite.json',a/'accepted-prerequisite.json')
files={}
for name in subprocess.check_output(['rg','--files','--hidden','-g','!.git','-g','!.cache','-g','!build','-g','!artifacts','-g','!reports'],cwd=t,text=True).splitlines():
 p=t/name
 if p.is_file():files[name]=h(p)
b={'head':head,'worktree':str(t),'files':files,'accepted_inputs':proof};(a/'input-baseline.json').write_text(json.dumps(b,indent=2)+'\n');shutil.copy2(a/'input-baseline.json',out/'input-baseline.json')
(a/'worker.prompt.txt').write_text('''Implement frozen work/WO-RUST-SCENE-DECODE-INITIALIZATION-01.md completely. Read OWNER/AGENTS, that exact order and own accepted-prerequisite.json FIRST. Coordinator accepted channelR3 after root independentreview and freshmain156checks; preceding writers/reviewer all terminal and current Scene/animation source equals accepted main. Old worker reports saying candidate pending are historical. You are sole shared writer now.\nNamed Sol6.1Medium architecture problem: actual C skips viewport on display:none root and parses animation AFTER traversal. Current SceneBuilder constructor eagerly requires validated viewport+parsed animation. Provide actual safe source-owned explicit provisional initialization/finalization preserving original error priority, accepted existing APIs/literals/assertions156+21 unchanged, real media/planner/request/child ownership usability, strict uninitialized/finalized/source/accounting/work/cancel/failure/hidden-root guards. No invented viewport, alternate source parse or apparently finished empty animation. Keep actual ready graph/render inaccessible until finalized. Full order requirements apply; read bounded actual original source lines and relevant SceneBuilder/model/view/frame/animation APIs rather than whole repo/history. Own only frozen allowed Scene source/newtest and minimal owninganimation API/newtest if needed, README and ownreports. All manifests/deps/peers/oldtests/state/ledgers protected; no unsafe/network/agents/reviewCLI/GUI/broadUI/billing/stage/reset/commit/delete. Source/tests<800. Standard existingsubscription only, stopquota/rateerrors no retries/credits. Implement, run ONLY named offline check/test/WO, report exact verification/APIcoverage/result. Root independent LunaHigh review/audit/freshmain before acceptance or seed.\n''')
runner=f'''from pathlib import Path
import os,json,subprocess,time
r=Path(__file__).resolve().parents[2];a=r/'artifacts'/{w!r};t=r/'.cache/WO-RUST-RENDER-INTEGRATION-CONTRACTS-01/worker';out=t/'artifacts'/{w!r};started=time.time();env=os.environ.copy()
for key in ('OPENAI_API_KEY','CODEX_API_KEY'):env.pop(key,None)
env.update(CARGO_HOME=str(t/'.cache/cargo'),TMPDIR=str(t/'.cache/tmp'),CARGO_TARGET_DIR=str(t/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
s={{'pid':os.getpid(),'status':'running','model':'gpt-6.1-sol','effort':'medium','worktree':str(t),'started_epoch':started}};p=a/'process-status.json';p.write_text(json.dumps(s,indent=2)+'\\n')
cmd=['codex','exec','--ignore-user-config','--profile','complex','--model','gpt-6.1-sol','-c','model_reasoning_effort="medium"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--color','never','--json','--cd',str(t),'--output-last-message',str(out/'last-message.md'),'-']
with (a/'worker.prompt.txt').open('rb') as i,(a/'worker.events.jsonl').open('wb') as o,(a/'worker.stderr.log').open('wb') as e:
 c=subprocess.Popen(cmd,stdin=i,stdout=o,stderr=e,env=env);s['child_pid']=c.pid;p.write_text(json.dumps(s,indent=2)+'\\n');code=c.wait()
s.update(status='completed' if code==0 else 'failed',exit_code=code,duration_seconds=time.time()-started);p.write_text(json.dumps(s,indent=2)+'\\n')
''';p=a/'runner.py';p.write_text(runner)
sys.path.insert(0,str(r));from scripts.ledger import append_entry
append_entry(r/'ledger.jsonl',dict(ts=datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),wo_id=w,event='run_started',revision=1,base_sha=head,head_sha=head,run_id=w+'-source-start',session_id='coordinator-initialization',model='gpt-6.1-sol',cli_version='0.162.1',duration_s=0,usage={'input_tokens':None,'output_tokens':None},status='running',artifacts=[f'artifacts/{w}/input-baseline.json',f'artifacts/{w}/accepted-prerequisite.json'],note='Starts only after channelR3 accepted/fresh156mainchecks and terminal source/reviewer; exact accepted Scene+animation inputs verified. Named SolMedium source-owned hidden-root/deferred-animation initialization; original assertions protected, no product claim.'))
log=(a/'supervisor.log').open('ab');proc=subprocess.Popen([sys.executable,str(p)],stdin=subprocess.DEVNULL,stdout=log,stderr=log,start_new_session=True);print({'supervisor_pid':proc.pid,'protected_input_hashes':len(files),'order':w})
