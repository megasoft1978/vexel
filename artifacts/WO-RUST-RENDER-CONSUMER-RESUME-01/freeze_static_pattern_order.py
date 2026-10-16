from pathlib import Path
import hashlib,json,subprocess,sys
from datetime import datetime,timezone
r=Path(__file__).resolve().parents[2];w='WO-RUST-PATTERN-RENDER-CONTRACT-01';a=r/'artifacts'/w;a.mkdir(exist_ok=False);head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip()
s=f'''---
id: {w}
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: gpt-6.1-sol
base_sha: {head}
parallel_safe: false
network: false
---
# Actual static pattern input and scoped production handoff
Start ONLY after current cache-publication source accepted/freshmain and its
writer/reviewer terminal. Single isolated shared writer; shape-publication waits.
Coordinator seeds exact current accepted SourceScene/tests and alloriginal
assertions/literals/APIs unchanged. No pending engine adoption.

## Request-minted static child input, not a ready parent
Actual original pattern.c239-345 traverses sealed child and raster_shape_render
BEFORE parentdecode.c214 animationparse; no childanimationparse. Accepted177
pending views cannot enter old readyframe/kernel APIs. Existing deferredchild
fixture proves lifetime survival but postponesrender after animation and cannot
be actual productiondecode timing. See audited call-chain proposal, revalidate
actual current declarations; proposednames are not existing APIs.
Supply opaque request-minted immutable StaticPatternView/input from actual
sealed newlyowned child tables/source/descriptor-matching positive viewport,
with NO AnimationView/OwnedAnimation/fakeemptydata or readyparent conversion.
Private constructors, source/owner/scene serial/generation/liveparentdraft/request/
mapping/ticket/job identity retained. Borrow builder throughout render so no
mutation/rollback/finalization invalidates it; dropview before same actual
complete_mapped publication. Descendant staticpatterns render depthfirst before
containingchildseals; originalcycle/depth/instance/byte constraints remain.
All actual shapes/path metadata/maps/transforms/paints/images/patterns/clips/
masks/groups/filters retained. Child mask traversal may access only validated
sealed mask-content closure, never arbitrary parent SceneId/rawscene escape.
No clone/reparse/alternatebackend/neutralpaint or dropping unsupported tables.

Provide additive common immutable raster-input/control/resource/index/sample/
flatkernel/sessionevent boundary that can consume both ready and StaticPattern
roles using the same table algorithms. Existing ready traits/structliterals and
exhaustive enums must compile unchanged; newrequired combinedproduction surface
may be additive. No silently inherited/defaultfake backend or unsupported static
route in requiredproduction contract. Do not implement wholelowerengines here;
prove actual typed fixture reads complete static data and emits samejob owned
output through existingmappedcompletion BEFORE any parentanimationparse.
Parentpending-to-ready/frame/freeze escape remains forbidden; existing compile-
fail assertions remain byteunchanged. Test childfailure+invalidparentanimation
reportschildfailure withzeroanimationparsercalls; successchildthenactualinvalid
parentanimation reportsanimation; staticanimateddescendants retain original
basegeometry rather than evaluateparentclock; nestedmask/pattern/effect data,
provider order/release, cancel/limit/source/job/stale/replay/rollback, no mutation
whileborrowed and no static/pending-to-readyconversion. Explicit fixturekernel
is not runtime/native/fullrenderer proof.

## Real scoped media token handoff
Current MediaDecodeOperations passes no actual DraftToken needed by accepted
request/admit_text_commands and production mediaStylePublisher. Add complete
required scopedproductionmedia operations/combinedsession handoff carrying the
actual guarded token to both text/image adapters. Oldliterals/traits remain
explicit compatibility only, no token reconstruction/global/threadlocal/unchecked
cleanup/fakecontext. Decoder owns realtoken and can pass disjointborrow toactual
publicmediafunction. Prove lowerfixture uses thattoken for real request/cumulative
budget/latefailure rollback whileprefix/token stays usable. Requiredscoped
production surfaces retain cache/paintchannel/nonpaintplanning/media capability.

## Original bounds convention
Actual definitions/pattern.rs uses bounds[2]-bounds[0] and [3]-[1], asoriginalC:
[minx,miny,maxx,maxy]. Shared productionvalidator/PatternMapping.pixel_inverse
currently rejectnegative maxima asifwidth/height. Correct sharedorderedminmax
validation (finitecoordinates/validextent) withoutrejecting valid allnegative
objects or accepting invertedpositivebounds. Keep originalarrayliterals/API;
centralizecommonguard. Realpatternmapping/planning/mappedpublication withnegative
bounds must succeed, invalid/inverted/nonfinite fail withoutprovider entry or
partial publication. Existing177+cachegate assertions unchanged. No arbitrary
coordinate rebasing or droppedSourceMap.

# Allowed paths
- rust/crates/vexel-scene/src/** (these static/scoped/bounds contracts only)
- rust/crates/vexel-scene/tests/static_pattern_contracts.rs (new only)
- rust/crates/vexel-scene/tests/scoped_media_contracts.rs (new only)
- rust/crates/vexel-scene/README.md
- artifacts/{w}/** (except coordinatorbaseline/seed)
Coordinator alone freezes/changes manifests/lock/registers outsideallowed source,
integrates main/seedspeers/docs/state/ledgers.

# Protected paths
Alloldtests/assertions/literals/othercrates/manifests/lock/config/source603/157/
oldorders/evidence/HEAD/index/state/docs/ledgers. No unsafe/newdeps/network/GPL/
Cshim/install/unsupportedOS/agents/reviewCLI/GUI/broadUI/stage/commit/reset/delete/
APIbilling/credits. Safe checkedfallible bounded ownership/work, source/tests<800.

# Verification
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py {w} --ready
Named Sol6.1Medium cross-engine staticinput/ownership timing, Standard existing
allowanceonly; stopquota/rateerrors no retry/billingchange. Exactlogs/fullAPIcoverage
and result<=500words. Root independentLunaHighreview/protectedaudit/freshmain
BEFOREacceptance/seed. Actual decoder/media/lowerkerneladoption+fullrenderer/native/
UI/app/MCP/performance/Rusttooling/finalQA/release/authenticdemo remain mandatory.
''';sha=hashlib.sha256(s.encode()).hexdigest();s=s.replace('parallel_safe:',f'spec_sha256: {sha}\nparallel_safe:',1);(r/'work'/f'{w}.md').write_text(s)
(a/'evidence.json').write_text(json.dumps({'static_timing_audit':'artifacts/WO-RUST-PATTERN-TIMING-AUDIT-01/call-chain.json','independent_root_source_check':['src/svg/shape/pattern.c:319-334 (noanimation parse beforechildrender)','src/svg/shape/decode.c:214','rust/crates/vexel-scene/src/pattern_publication.rs:86-100 (pendingchildrole)','rust/crates/vexel-svg-definitions/src/pattern.rs:51-52 (minmaxwidthheight)','rust/crates/vexel-scene/src/pattern_publication.rs:24-25 (wrongnegative-maxguard)'],'not_started':True,'prerequisite':'WO-RUST-CACHE-PUBLICATION-01 accepted source and terminal reviewer'},indent=2)+'\n')
sys.path.insert(0,str(r));from scripts.ledger import append_entry
append_entry(r/'ledger.jsonl',dict(ts=datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),wo_id=w,event='ready',revision=1,base_sha=head,head_sha=head,run_id=w+'-ready',session_id='coordinator-static-pattern',model='gpt-6.1-sol',cli_version='0.162.1',duration_s=0,usage={'input_tokens':None,'output_tokens':None},status='ready',artifacts=[f'work/{w}.md',f'artifacts/{w}/evidence.json'],note='Original staticchild render beforeparentlateanimation confirmed; requestmintedstaticinput/additiveproductionresources+realmediaDraftToken handoff and correctminmax bounds READY aftercachegate, notstarted. Existingready/pendingguards/originalassertions protected, no implementationclaim.'))
print(w,'READY, notstarted; cachegate reviewpending')
