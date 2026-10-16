"""Coordinator: preserve terminal candidates, seed accepted Scene153, resume.
No manifests, original fixtures, candidate source, HEAD or index are rewritten.
"""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,sys,time
from datetime import datetime,timezone
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT))
from scripts.ledger import append_entry
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
HEAD=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
items=[('WO-RUST-SVG-DECODE-01','vexel-svg-decode','full-repair-status.json','01a13bc2-b2ae-74b0-bd2f-570357eaf8b6','gpt-6.1-sol','medium','complex',2),('WO-RUST-SVG-MEDIA-01','vexel-svg-media','parity-repair-status.json','01a13bc2-c530-75f2-bbf6-65b821addd14','gpt-6.1-sol','medium','complex',2),('WO-RUST-SVG-SCALAR-SAMPLE-01','vexel-svg-scalar-sample','job-contract-resume-status.json','01a13bc2-d60a-7963-b7d4-df333e5dbc0f','gpt-6-luna','high','economy',1)]
focus={
'vexel-svg-decode':'''Adopt typed Shape/Group DefinitionOwner, actual production CachedPaintResolver/DefinitionPlanner handoff, PatternMapping, MaskPostTarget, and DecodeQuantum soft atomic boundaries. Preserve and now pass the existing smooth-after-Z original assertion, without changing parser or metadata. Complete remaining capacity leases (plan/stack/child-draft/temporary-clip), lower-plan flavor/post validation, hidden/display:none/eager-animation priority, nested gradient/pattern/media/marker flows and all assigned pure predicates. 32 passing plus one failing old test is NOT complete. Prefer production delegates on the actual production path; keep intentional fixture legacy interfaces explicit. Definition paint-channel handoff is a separate coordinator gap: report precise dependency and do independent work, never infer fill/stroke equality.''',
'vexel-svg-media':'''Use builder.begin_request/commit_request/rollback_request with the caller real active DraftToken for both image and text. Preserve the outer prefix and token usability on every late allocation/limit/cancellation failure. Restore external TextState/cache admissions too. Use accepted cumulative TextState document text-command budget on every run including cached reuse, independent of cache command cap. Existing eight defect reproducers must become meaningful successful atomicity/limit regression assertions (do not retain assertions that require a bug or merely rename them). Preserve all previous functional predicates; document the assertion transition from bug demonstration to required correct result. Prove 17th cached 262144-command use fails at original 4Mi document bound and request failure restores counters. Keep corrected Unsupported priority/source offset, compound contours, font/style/spacing/anchor/cache proofs.''',
'vexel-svg-scalar-sample':'''The exact reported whole-job cache-read versus row-write gap is now solved by RasterJob::compose and RasterComposition (borrow disjoint admitted buffers, validate cache once). Adopt it on real controlled sampling paths. Complete dash/grid construction and fill-row scratch through admitted same-job buffers; do not leave legacy unmetered Vec production paths, clone read caches, revalidate full cache per sample, or invent another backend. Preserve actual original bounded-work math, global highbit join indices, seam/cap/join/fallback/pattern/image/paint-order/clip/4x4 semantics and finish all original 11 pure predicates. Original 9 focused tests remain meaningful and unchanged unless additive improvements explicitly mapped. Actual capacities/old-new overlap/cancel/panic cleanup need proofs.'''
}
def ledger(w,event,revision,model,note):
 stamp=datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
 append_entry(ROOT/'ledger.jsonl',dict(ts=stamp,wo_id=w,event=event,revision=revision,base_sha=HEAD,head_sha=HEAD,run_id=w+'-scene153-'+event,session_id='coordinator-scene153',model=model,cli_version='0.162.1',duration_s=0,usage=dict(input_tokens=None,output_tokens=None),status=event,artifacts=[f'artifacts/{w}/accepted-integration-source-seed.json'],note=note))
for w,crate,oldstatus,thread,model,effort,profile,revision in items:
 a=ROOT/'artifacts'/w;t=ROOT/'.cache'/w/'worker';out=t/'artifacts'/w
 old=json.loads((a/oldstatus).read_text());assert old['status'] in ('completed','failed')
 for key in ('pid','child_pid'):
  pid=old.get(key)
  if pid and subprocess.run(['ps','-p',str(pid),'-o','comm='],capture_output=True,text=True).stdout.strip():raise RuntimeError(f'prior PID still exists: {w} {pid}')
 archive=a/'pre-accepted-integration-resume';assert not archive.exists()
 shutil.copytree(t/'rust'/'crates'/crate,archive/'crate')
 shutil.copytree(out,archive/'reports')
 files={}
 shared=ROOT/'rust/crates/vexel-scene'
 for p in sorted(shared.rglob('*')):
  if not p.is_file() or not (p.relative_to(shared).parts[0] in ('src','tests') or p.name=='README.md'):continue
  rel=p.relative_to(ROOT);dst=t/rel
  files[str(rel)]={'before':H(dst) if dst.exists() else None,'after':H(p)}
  dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dst)
 evidence={'authorizing_order':'WO-RUST-RENDER-INTEGRATION-CONTRACTS-01','accepted_main_checks':153,'only_after_terminal':old,'files':files,'preserved_candidate':str(archive.relative_to(ROOT)),'frozen_scope_unchanged':True}
 for dst in (a/'accepted-integration-source-seed.json',out/'accepted-integration-source-seed.json'):dst.write_text(json.dumps(evidence,indent=2)+'\n')
 dest=t/'artifacts/WO-RUST-RENDER-INTEGRATION-CONTRACTS-01';dest.mkdir(parents=True,exist_ok=True)
 for name in ('coordinator-result.md','main-verification.json'):
  shutil.copy2(ROOT/'artifacts/WO-RUST-RENDER-INTEGRATION-CONTRACTS-01'/name,dest/name)
 prompt=f'''Resume the SAME frozen work order work/{w}.md and entire immutable scope.json, not a subset. Coordinator verified prior writer terminal, archived complete candidate/tests/reports, and seeded exact accepted Scene153 source/tests/README only; read own accepted-integration-source-seed.json. Original scope, source603, suites157, all original assertions/manifests/locks/peer interfaces stay protected. No old seed rollback.\n\n{focus[crate]}\n\nComplete only original owned crate source/new focused tests/README and own reports. Use actual current public source APIs. Report a genuinely missing shared contract before substituting semantics, then continue independent work. No fake providers/empty-success/reduced scope. Source/tests <800 lines, std safe bounded fallible ownership, no shared edits/manifest/dependency/network/agents/GUI/broad UI/stage/commit/reset/delete. Existing subscription Standard only; stop on quota/rate errors without retry or billing changes. Run only frozen offline check/test/WO validations; report exact commands, full parity-map/coverage/verification/result <=400words including remaining pure cases. Successful process exit is candidate only, independent review/main checks still mandatory. Avoid repeatedly rereading whole tree/logs. This is an implementation continuation; implement the work rather than restating a plan.\n'''
 (a/'accepted-integration-resume.prompt.txt').write_text(prompt)
 runner=f'''from pathlib import Path
import os,json,subprocess,time
root=Path(__file__).resolve().parents[2];a=root/'artifacts'/{w!r};tree=root/'.cache'/{w!r}/'worker';out=tree/'artifacts'/{w!r};started=time.time();env=os.environ.copy()
for key in ('OPENAI_API_KEY','CODEX_API_KEY'):env.pop(key,None)
env.update(CARGO_HOME=str(tree/'.cache/cargo'),TMPDIR=str(tree/'.cache/tmp'),CARGO_TARGET_DIR=str(tree/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
s={{'pid':os.getpid(),'status':'running','model':{model!r},'effort':{effort!r},'worktree':str(tree),'started_epoch':started}};p=a/'accepted-integration-resume-status.json';p.write_text(json.dumps(s,indent=2)+'\\n')
cmd=['codex','exec','--ignore-user-config','--profile',{profile!r},'--model',{model!r},'-c','model_reasoning_effort="{effort}"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--color','never','--json','--cd',str(tree),'--output-last-message',str(out/'accepted-integration-resume-last-message.md'),'resume',{thread!r},'-']
with (a/'accepted-integration-resume.prompt.txt').open('rb') as i,(a/'accepted-integration-resume.events.jsonl').open('wb') as o,(a/'accepted-integration-resume.stderr.log').open('wb') as e:
 c=subprocess.Popen(cmd,stdin=i,stdout=o,stderr=e,env=env);s['child_pid']=c.pid;p.write_text(json.dumps(s,indent=2)+'\\n');code=c.wait()
s.update(status='completed' if code==0 else 'failed',exit_code=code,duration_seconds=time.time()-started);p.write_text(json.dumps(s,indent=2)+'\\n')
'''
 rp=a/'accepted_integration_resume_runner.py';rp.write_text(runner)
 latest=None
 for line in (ROOT/'ledger.jsonl').read_text().splitlines():
  row=json.loads(line)
  if row['wo_id']==w:latest=row['event']
 if latest=='run_started':ledger(w,'run_finished',revision,model,'Prior candidate terminal/incomplete; original source/tests/reports preserved before exact accepted Scene153 seed. No source acceptance.')
 ledger(w,'run_started',revision,model,'Adopt accepted Scene153 on same full frozen scope; coordinator-only exact shared seed after terminal, candidate archived. Focused checks only; no UI/E2E or fixture correction.')
 log=(a/'accepted-integration-supervisor.log').open('ab')
 proc=subprocess.Popen([sys.executable,str(rp)],cwd=ROOT,stdin=subprocess.DEVNULL,stdout=log,stderr=log,start_new_session=True)
 print(json.dumps({'wo_id':w,'supervisor_pid':proc.pid,'model':model,'effort':effort,'seeded_files':len(files)}))
