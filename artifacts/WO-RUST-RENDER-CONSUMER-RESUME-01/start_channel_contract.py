from pathlib import Path
import json,hashlib,subprocess,sys,shutil,os
from datetime import datetime,timezone
r=Path(__file__).resolve().parents[2];sys.path.insert(0,str(r));from scripts.ledger import append_entry
w='WO-RUST-PAINT-CHANNEL-CONTRACT-01';a=r/'artifacts'/w;a.mkdir(exist_ok=False)
t=r/'.cache/WO-RUST-RENDER-INTEGRATION-CONTRACTS-01/worker'
old=json.loads((r/'artifacts/WO-RUST-RENDER-INTEGRATION-CONTRACTS-01/process-status.json').read_text());assert old['status']=='completed'
assert not subprocess.run(['ps','-p',str(old['child_pid']),'-o','comm='],capture_output=True,text=True).stdout.strip()
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip()
# Shared writer starts only on exact accepted main Scene source/tests.
for p in (r/'rust/crates/vexel-scene').rglob('*'):
 if p.is_file() and (p.relative_to(r/'rust/crates/vexel-scene').parts[0] in ('src','tests') or p.name=='README.md'):
  assert h(p)==h(t/p.relative_to(r)),p
order=f'''---
id: {w}
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: gpt-6-luna
base_sha: {head}
parallel_safe: false
network: false
---
# Explicit paint channel for production pattern planning
The accepted DefinitionPlanner::plan_owned_definition has no PaintChannel, yet
PatternMapping.opacity is one resolved instance value. Full definitions adapter
currently refuses unequal fill-opacity/stroke-opacity, including omitted initial1.
Close this precise shared interface gap on accepted Scene153 without losing
original error priority, source ownership, complete mapping or scene publication.
Provide an additive publicly consumable REQUIRED channel-aware production planner
operation and validated dispatch accepting Fill/Stroke explicitly. There must be
no implicit fill/default conversion/equality restriction or lossy production
fallback. Existing153 tests, old literals/traits and explicit compatibility
interfaces remain byte/API compatible. Document how decoder and definitions
adopt the new production route; any needed typed combined operations/session
interface must be additive and genuinely usable. Nonpaint filter/clip/mask/marker
owners retain their existing semantics; no fake shape for group filter.

Validate canonical source/context/typed owner, element, bounds, transform, style,
returned kind/mapping/post just as accepted production planner does; both routes
must share validation behavior without copied drifting validator implementations.
No unchecked lower planner call, no dropped channel across suspended pattern
request/ticket/result/publication. Mapping already carries resolved opacity;
new fixture planner explicitly reads selected computed style channel, including
initial1. Prove fill .25/stroke .75 and opposite/default1 through actual production
request and completed PatternId fill/stroke publication on real builder/job;
source-span errors, stale/foreign/wrong-target, bad mapping/kind, cancellation,
rollback and original153 assertions preserved. Full lower engine implementation
and app runtime remain downstream, do not invent a backend or claim them here.

# Allowed paths
- rust/crates/vexel-scene/src/** (this additive shared route/refactoring only)
- rust/crates/vexel-scene/tests/paint_channel_contracts.rs (new only)
- rust/crates/vexel-scene/README.md
- artifacts/{w}/** (except coordinator baseline)
Coordinator alone freezes order, registers modules/manifests, integrates main,
seeds peers and updates ledgers/state. A source module needed by existing lib may
be registered within lib.rs under this accepted source-only scope.

# Protected paths
All existing tests/literals, other crates, manifests/lock/config, original603/
157 suites/fixtures/oracles, source C, old evidence/orders, HEAD/index, state/docs/
ledgers. One isolated shared writer only. No unsafe/newdeps/network/install/GPL/
Cshim/unsupportedOS/recursive agents/GUI/broadUI/stage/commit/reset/delete/billing.
Source/tests under800lines; preserve bounded checked fallible ownership/accounting.

# Verification
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py {w} --ready
LunaMedium Standard existing subscription only; stop quota/rate errors, no retry
loop or purchased credits/APIbilling. Candidate only until independent LunaHigh
review, protected audit and fresh main checks. Exact command logs/verification,
APIcoverage.md including downstream adoption and result.md <=350words required.
'''
sha=hashlib.sha256(order.encode()).hexdigest();order=order.replace('parallel_safe:',f'spec_sha256: {sha}\nparallel_safe:',1)
p=r/'work'/f'{w}.md';p.write_text(order);(t/'work'/p.name).write_text(order)
out=t/'artifacts'/w;out.mkdir(exist_ok=False)
files={}
paths=subprocess.check_output(['rg','--files','--hidden','-g','!.git','-g','!.cache','-g','!build','-g','!artifacts','-g','!reports'],cwd=t,text=True).splitlines()
for rel in paths:
 p=t/rel
 if p.is_file():files[rel]=h(p)
baseline={'head':head,'prior_shared_writer':old,'files':files,'accepted_scene_checks':153,'worktree':str(t)}
(a/'input-baseline.json').write_text(json.dumps(baseline,indent=2)+'\n');shutil.copy2(a/'input-baseline.json',out/'input-baseline.json')
prompt=f'''Implement frozen work/{w}.md completely. Read OWNER/AGENTS and that order, then current scene production_decode.rs/pattern_publication.rs and existing tests as needed. Accepted input153 assertions all immutable. Implement additive explicit channel-aware real production dispatch; preserve old compatibility traits/literals without default fake/fill route. Shared validation should be centralized. Use actual builder/job mapped pattern completion proving distinct .25/.75 and omitted1 independently. Limit reads to relevant files; no whole repository scan. Isolated shared writer authorized after prior writer terminal and exact accepted source verified. Own only allowed source/newtest/README/ownreports. No manifests or peer engines. Full owned definition adapter reports at {r}/.cache/WO-RUST-SVG-DEFINITIONS-01/worker/artifacts/WO-RUST-SVG-DEFINITIONS-01 and planner.rs are read-only reference; do not modify them or rely on them compiling. Only named offline focused check/test/WO commands. Standard LunaMedium existing subscription no credits/APIbilling; stop quota/rateerrors. Output real implementation, exact verification/APIcoverage/result, no product acceptance claim.\n'''
(a/'worker.prompt.txt').write_text(prompt)
runner=f'''from pathlib import Path
import os,json,subprocess,time
r=Path(__file__).resolve().parents[2];a=r/'artifacts'/{w!r};t=r/'.cache/WO-RUST-RENDER-INTEGRATION-CONTRACTS-01/worker';out=t/'artifacts'/{w!r};started=time.time();env=os.environ.copy()
for key in ('OPENAI_API_KEY','CODEX_API_KEY'):env.pop(key,None)
env.update(CARGO_HOME=str(t/'.cache/cargo'),TMPDIR=str(t/'.cache/tmp'),CARGO_TARGET_DIR=str(t/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
s={{'pid':os.getpid(),'status':'running','model':'gpt-6-luna','effort':'medium','worktree':str(t),'started_epoch':started}};p=a/'process-status.json';p.write_text(json.dumps(s,indent=2)+'\\n')
cmd=['codex','exec','--ignore-user-config','--profile','economy','--model','gpt-6-luna','-c','model_reasoning_effort="medium"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--color','never','--json','--cd',str(t),'--output-last-message',str(out/'last-message.md'),'-']
with (a/'worker.prompt.txt').open('rb') as i,(a/'worker.events.jsonl').open('wb') as o,(a/'worker.stderr.log').open('wb') as e:
 c=subprocess.Popen(cmd,stdin=i,stdout=o,stderr=e,env=env);s['child_pid']=c.pid;p.write_text(json.dumps(s,indent=2)+'\\n');code=c.wait()
s.update(status='completed' if code==0 else 'failed',exit_code=code,duration_seconds=time.time()-started);p.write_text(json.dumps(s,indent=2)+'\\n')
'''
runnerpath=a/'runner.py';runnerpath.write_text(runner)
for event in ('ready','run_started'):
 append_entry(r/'ledger.jsonl',dict(ts=datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),wo_id=w,event=event,revision=1,base_sha=head,head_sha=head,run_id=w+'-'+event,session_id='coordinator-channel',model='gpt-6-luna',cli_version='0.162.1',duration_s=0,usage={'input_tokens':None,'output_tokens':None},status=event,artifacts=[f'work/{w}.md',f'artifacts/{w}/input-baseline.json'],note='Explicit required paint-channel production planning; additive API preserves accepted153 assertions. LunaMedium isolated single shared writer, focused checks only.'))
log=(a/'supervisor.log').open('ab');proc=subprocess.Popen([sys.executable,str(runnerpath)],cwd=r,stdin=subprocess.DEVNULL,stdout=log,stderr=log,start_new_session=True)
print(json.dumps({'wo_id':w,'supervisor_pid':proc.pid,'protected_input_hashes':len(files),'model':'gpt-6-luna','effort':'medium'}))
