from pathlib import Path
import hashlib,json,subprocess
from datetime import datetime,timezone
r=Path(__file__).resolve().parents[2];a=r/'artifacts/WO-RUST-RENDER-CONSUMER-RESUME-01';now=datetime.now(timezone.utc).isoformat()
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
b=json.loads((r/'artifacts/WO-RUST-PLAN-01/source-baseline.json').read_text());bad=[]
for name,sha in b['files'].items():
 if h(r/name)!=sha:bad.append(name)
old={}
def walk(x):
 if isinstance(x,dict):
  if isinstance(x.get('path'),str) and isinstance(x.get('sha256'),str):old[x['path']]=x['sha256']
  for y in x.values():walk(y)
 elif isinstance(x,list):
  for y in x:walk(y)
walk(json.loads((r/'artifacts/WO-RUST-RENDER-CONTRACT-PLAN-01/assertion-assignments.json').read_text()))
for name,sha in old.items():
 if h(r/name)!=sha:bad.append(name)
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip();index=r/subprocess.check_output(['git','rev-parse','--git-path','index'],cwd=r,text=True).strip()
assert head==b['head'] and h(index)==b['index_sha256'] and not bad,(head,bad)
audit={'recorded_utc':now,'legacy_files_checked':len(b['files']),'frozen_acceptance_files_checked':len(old),'changed':bad,'head':head,'index_sha256':h(index),'fixture_approval':'pending; original unchanged','broad_UI_E2E_run':False}
(a/'scene153-resume-preservation.json').write_text(json.dumps(audit,indent=2)+'\n')
cp=a/'current-goal-checkpoint.json';d=json.loads(cp.read_text());d['recorded_utc']=now;d['goal_status']='usageLimited';d['repository_execution_status']='Owner-authorized continue; no tracker status reset attempted';d['turn_classification']='progress: accepted query13 and shared integration153, preserved terminal candidates and resumed full decoder/media/sample on exact accepted Scene153; new additive paint-channel contract in isolated LunaMedium shared writer.'
workers=[]
for w,f in [('WO-RUST-SVG-DECODE-01','accepted-integration-resume-status.json'),('WO-RUST-SVG-MEDIA-01','accepted-integration-resume-status.json'),('WO-RUST-SVG-SCALAR-SAMPLE-01','accepted-integration-resume-status.json'),('WO-RUST-PAINT-CHANNEL-CONTRACT-01','process-status.json')]:
 p=r/'artifacts'/w/f;s=json.loads(p.read_text());obs=subprocess.run(['ps','-p',str(s['child_pid']),'-o','pid=,etime=,comm='],capture_output=True,text=True).stdout.strip();s.update(wo_id=w,status_file=str(p.relative_to(r)),process_observation=obs or 'not live',confirmed_live_at_utc=now if obs else None,source_candidate_accepted=False);workers.append(s)
d['worker_status']=workers;d['total_confirmed_source_writers']=sum(bool(x['process_observation']!='not live') for x in workers)
d['source_candidate_policy']='Accepted shared153/query13/frame31/paint26 gates are source-only. Active candidates are not accepted until complete parity, independent review/protected audit/fresh main checks. Original incomplete candidates retained.'
d['next_gate']='Finish/review media request atomicity/document command cap, decoder full production traversal/accounting, scalar same-job admitted caches/rows. Accept additive explicit fill/stroke pattern-channel contract then resume definitions. Index numeric fixture decision pending. Native readiness incomplete. Full renderer/app/UI/MCP/performance/Rust tooling/finalQA/release/live demos remain.'
d['awaiting_shared_prerequisites']={'media':'Resumed SolMedium on accepted153; cumulative counter is SceneBuilder-owned, not TextState. Must replace bug reproducers with correct atomicity proofs.','decoder':'Resumed SolMedium on accepted153; original smooth-after-Z assertion unchanged, remaining capacity/error-priority/full pure cases required.','scalar_sample':'Resumed LunaHigh on accepted153 RasterComposition; full dash/grid/fill admission and remaining pure cases required.','definitions':'Terminal36-test incomplete candidate preserved; requires explicit channel-aware production planning now in new shared LunaMedium order.','scalar_index':'Terminal incomplete candidate; one frozen compact-number negative conflicts with SVG grammar. Concrete correction prepared, explicit owner approval pending.','native_readiness':'R2 independently rejected: selected cached API/minOS declaration proof and exact individual test ownership incomplete; no implementation acceptance.'}
d['independent_review_blockers']={'definitions':'Existing production planner lacks explicit Fill/Stroke; distinct opacity/default1 unsupported.','scalar_index':'Frozen negative fixture conflict awaiting owner approval; parser unchanged.','native_readiness':'Cached declarations/feature/macOS13 and individual frozen assertion ownership not fully evidenced.'}
d['evidence'].append('scene153-resume-preservation.json');d['fixture_correction']={'status':'pending explicit owner approval','proposal':'artifacts/WO-RUST-SVG-SCALAR-INDEX-01/path-separator-proposal/decision.md','original_unchanged':True}
cp.write_text(json.dumps(d,indent=2)+'\n')
p=r/'STATE.md';s=p.read_text();start=s.index('Latest continuation:');end=s.index('The initial consolidation snapshot',start)
s=s[:start]+'''Latest continuation — 2026-10-15: **720 recorded focused entries across31
accepted main-tree packages**. Query13 and sharedScene153 are now independently
reviewed, protected-audited and freshly main-checked. Scene153 preserves135
prior assertions and adds16runtime plus2ownership compile-fail entries.
Its request cleanup/cumulative text budget, same-job cache read/scratch write,
canonical post-close topology, mask target and soft quantum APIs are accepted.
Three isolated full-scope implementation sessions now adopt those exact inputs:
decoder and media at Sol6.1Medium, scalar sampling at LunaHigh. A separate
LunaMedium single shared writer supplies explicit fill/stroke pattern planning;
full definitions36-test candidate remains incomplete until adoption/review.
Scalar index remains incomplete pending the owner's explicit frozen-fixture
correction decision; original parser and acceptance files are unchanged.
Native readiness R2 remains rejected for incomplete API/minOS/test evidence.
Full renderer/app/runtime/performance/MCP/tooling/finalQA/release/README+website
authentic video remain required; broad UI/E2E has not run.
''' +s[end:]
s=s.replace('The owner resumed the tracked goal; the latest goal API read confirms **active**.','The latest goal API read on2026-10-15 reports **usageLimited**. The owner has\nauthorized continuation; repository execution is recorded separately, with no\nattempt to reset the tracker or purchase usage.')
p.write_text(s)
p=r/'docs/product/rust-port-goal-and-plan.md';s=p.read_text();s=s.replace('No finished\nRust port or performance gain is claimed. The owner resumed the tracked goal;\nits current status is **active**, verified through the goal API on 2026-10-14.','No finished\nRust port or performance gain is claimed. The latest tracker read on2026-10-15\nreports **usageLimited**; owner-authorized repository continuation is tracked\nseparately. No tracker reset, billing change or additional-credit purchase is\nattempted.')
s+='''\n## Latest accepted source checkpoint — 2026-10-15\n\n720 recorded focused entries across31 main packages. Scene query13 and shared\nScene153 source gates are accepted after independent review, exact protected\naudits and fresh main checks.153includes135unchanged plus16runtime/2compilefail.\nThe coordinator preserved terminal candidates before seeding exact accepted\nScene153 to full decoder/media/scalar-sample sessions; two named SolMedium\nrepairs and one LunaHigh implementation run independently. An additive required\nFill/Stroke production-planning contract runs under LunaMedium as the sole\nshared writer; definitions adoption/review remains pending. No pending candidate\nis accepted by process exit or source registration alone.\n\nThe index compact-number negative fixture correction is a concrete proposal\nawaiting explicit owner approval. All603legacy files and157frozen acceptance\nfiles/fixtures/goldens/manifests, HEAD and index are unchanged. Native readiness\nR2 is still rejected for incomplete actual cached API/minOS and individual\nassertion mappings. Broad UI/E2E has not run. Every full-product outcome and\nrelease/demo completion gate above remains required. Current process snapshots\nand preservation evidence are in the current-goal checkpoint, not old process\nrows in historical sections.\n'''
p.write_text(s)
p=r/'LEDGER.md';s=p.read_text();s+='''\n| 2026-10-15 | Rust accepted153 consumer continuation | implementation running; no new candidate accepted | `19234a036ed6a8d40c2ccdaec9b8186806d70eca` | — |720recorded focused entries/31accepted packages. Preserved terminal decoder/media/sample candidates, exactScene153seed then resumed twoSolMedium+oneLunaHigh sessions; isolated LunaMedium required paint-channel shared route.603legacy/157acceptance/HEAD/index preserved. Fixture correction pending explicit approval; no broad UI/E2E/full-product claim. Evidence:`artifacts/WO-RUST-RENDER-CONSUMER-RESUME-01/current-goal-checkpoint.json`. |\n''';p.write_text(s)
print(json.dumps({'audit':audit,'live_workers':d['total_confirmed_source_writers'],'checkpoint':str(cp.relative_to(r))}))
