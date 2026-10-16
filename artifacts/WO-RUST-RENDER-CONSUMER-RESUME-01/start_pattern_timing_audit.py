from pathlib import Path
import hashlib,json,subprocess,sys,os
from datetime import datetime,timezone
r=Path(__file__).resolve().parents[2];w='WO-RUST-PATTERN-TIMING-AUDIT-01';a=r/'artifacts'/w;a.mkdir(exist_ok=False);head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip()
s=f'''---
id: {w}
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: gpt-6.1-sol
base_sha: {head}
parallel_safe: true
network: false
---
# Bounded cross-engine static-child timing audit
Read-only integration architecture audit, not another broad rendering plan.
Original pattern.c:239-345 traverses a static child then calls raster_shape_render
BEFORE parent decode.c:214 parses root animation. The child is not passed through
svg_anim_parse. Accepted provisional Scene177 requires actual late parent
animation before ready SceneView; request_mapped_traversal yields pending view.
Determine exact actual safe path needed to render that child during traversal
without fake emptyanimation, earlyparentparse, parentreadyescape, lost source/
mask/style/opacity, or delayed errorpriority. Inspect actual accepted Scene APIs,
current decoder candidate and source-original flow, not historical signatures.
Propose MINIMAL complete implementable boundary/adapter with precise types,
ownedsource/viewport/draft/ticket/job/lifetime/control guards and original child
renderfailure-before-parent-lateanimation proof. Preserve177accepted Scene tests/
compilefail pendingparent guard; explicit static role is different from a ready
animated parent, no weakened originalguard or invented backend. Consider nested
patterns and media callbacks transaction/deferred effect ordering ONLY as needed
for this actual same call chain. No unrelatednewarchitecture/fullengine rewrite.
If existing APIs already suffice, show real exact call path/types and why. If not,
return concrete source-gap patchscope/requirements and required regressioninputs.
Source implementation happens in separate coordinator-owned frozen order.

# Allowed paths
- artifacts/{w}/** (reports only; input-baseline/runner/order coordinator-owned)
# Protected paths
ALL sources/tests/manifests/locks/workorders/docs/state/ledgers/oldreports/input/
603/157/HEAD/index. READONLY no build/test/network/GUI/agents/reviewCLI/unsafe/
sourceedits/packages/stage/commit/reset/delete/billing. StandardSolMedium named
crossengine timing design, existing allowanceonly; stopquota/rateerror no retry.
# Verification
Exact source/type/call-chain references and declaration hashes; read-only proof.
No test/build/runtime results claimed. Root independent review of evidence before
future sourcework. Avoid whole-tree/history scans, read only relevant functions.
# Output
result.md<=500words, call-chain.json with line/symbol/hash evidence and exact
minimal typed API/adoption/test proposal or proved existing path. Full product
outcomes and remaining gates unchanged; no implementation/acceptance claim.
''';sha=hashlib.sha256(s.encode()).hexdigest();s=s.replace('parallel_safe:',f'spec_sha256: {sha}\nparallel_safe:',1);(r/'work'/f'{w}.md').write_text(s)
names=['src/svg/shape/pattern.c','src/svg/shape/decode.c','rust/crates/vexel-scene/src/pattern_publication.rs','rust/crates/vexel-scene/src/decode_initialization.rs','rust/crates/vexel-scene/src/frame.rs','rust/crates/vexel-scene/tests/decode_initialization.rs'];h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();(a/'input-baseline.json').write_text(json.dumps({'head':head,'files':{n:h(r/n) for n in names},'decoder_candidate':'.cache/WO-RUST-SVG-DECODE-01/worker','sourceonly':True},indent=2)+'\n')
(a/'worker.prompt.txt').write_text('''Perform ONLY frozen WO-RUST-PATTERN-TIMING-AUDIT-01 audit. Read order then exact original pattern.c239-345/decode.c198-218 and accepted Scene177 patternrequest/provisional/frame types; current decoder candidate pattern steps/read4relevantfiles as needed. Original static child renders before parent animation parse, no childanimationparse. Current pending child cannot enter readySceneView/frame. Determine exactminimal safe static child input/role or prove existing route, preserve parentunresolvedguards/originalerrorpriority/nested pattern/draft/ticket/job/source/typedlifetimes. Avoid fakeemptyanimation, earlyparseparent, delayingchildfailure, scratchwrapper/reparse/missing style/callback semantic. Read only actualsource enough to settle coupling; no fullrepo/logs. Reports ONLY ownartifact allowed. No Source/tests/manifests/ledgers/workorder edits, build/test/network/GUI/agents/reviewCLI/billing. Standard SolMedium subscriptiononly stopquota/rateerrors no retries/credits. Deliver concise result and exact callchain/API/regression proposal, not newroadmap or implementation. Root decides future sharedscope from evidence.\n''')
runner=f'''from pathlib import Path
import os,json,subprocess,time
r=Path(__file__).resolve().parents[2];a=r/'artifacts'/{w!r};started=time.time();env=os.environ.copy()
for key in ('OPENAI_API_KEY','CODEX_API_KEY'):env.pop(key,None)
s={{'pid':os.getpid(),'status':'running','model':'gpt-6.1-sol','effort':'medium','worktree':str(r),'started_epoch':started}};p=a/'process-status.json';p.write_text(json.dumps(s,indent=2)+'\\n')
cmd=['codex','exec','--ignore-user-config','--profile','complex','--model','gpt-6.1-sol','-c','model_reasoning_effort="medium"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--color','never','--json','--cd',str(r),'--output-last-message',str(a/'last-message.md'),'-']
with (a/'worker.prompt.txt').open('rb') as i,(a/'worker.events.jsonl').open('wb') as o,(a/'worker.stderr.log').open('wb') as e:
 c=subprocess.Popen(cmd,stdin=i,stdout=o,stderr=e,env=env);s['child_pid']=c.pid;p.write_text(json.dumps(s,indent=2)+'\\n');code=c.wait()
s.update(status='completed' if code==0 else 'failed',exit_code=code,duration_seconds=time.time()-started);p.write_text(json.dumps(s,indent=2)+'\\n')
''';p=a/'runner.py';p.write_text(runner)
sys.path.insert(0,str(r));from scripts.ledger import append_entry
for ev in ['ready','run_started']:
 append_entry(r/'ledger.jsonl',dict(ts=datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),wo_id=w,event=ev,revision=1,base_sha=head,head_sha=head,run_id=w+'-'+ev,session_id='coordinator-pattern-timing',model='gpt-6.1-sol',cli_version='0.162.1',duration_s=0,usage={'input_tokens':None,'output_tokens':None},status=ev,artifacts=[f'work/{w}.md',f'artifacts/{w}/input-baseline.json'],note='Bounded read-only audit: original staticpattern child renders before lateparentanimation; acceptedScene177 pendingchild cannot use readyframe. Need precise actualtyped integration boundary preservingparentguards/errorpriority. Ownreports only, no source/build/test/runtime claim.'))
log=(a/'supervisor.log').open('ab');proc=subprocess.Popen([sys.executable,str(p)],stdin=subprocess.DEVNULL,stdout=log,stderr=log,start_new_session=True);print('SolMedium bounded static pattern timing audit',proc.pid)
