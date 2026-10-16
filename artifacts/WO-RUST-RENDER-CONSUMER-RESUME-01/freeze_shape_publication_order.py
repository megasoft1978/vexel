from pathlib import Path
import hashlib,json,subprocess,sys
from datetime import datetime,timezone
r=Path(__file__).resolve().parents[2];w='WO-RUST-SHAPE-PUBLICATION-01';a=r/'artifacts'/w;a.mkdir(exist_ok=False);head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip()
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
# Actual shape admission and canonical geometry source
Start single isolated shared writer ONLY after scene decode initialization
source is independently accepted/fresh-main-checked and its writer/reviewer are
terminal. Coordinator seeds exact accepted source/tests; no pending code and no
concurrent shared writer. This closes TWO coupled append/publication boundaries,
not whole lower decoder/media engines. Every original accepted assertion and
literal/API remains byte/source compatible, including latestScene156and later
accepted initialization entries. Source603/157frozen inputs unchanged.

## Pre-geometry real shape capacity admission
Original src/svg/shape/shape_custom.c:53 calls builder_begin_shape after fill/
fill-rule validation, BEFORE fill-opacity/stroke and geometry/provider work.
shape_geometry.c:55 admits shape count and actual metadata capacity. Current
Scene append_shape only admits after completed geometry; probe bytes or dummy
empty shape changes actual failure priority. Supply real public draft/request-
scoped source-offset-bearing shape admission token/lease (or equivalent safe
owning state). Check actual original shape cap/size arithmetic before provider,
reserve actual builder shape/serial metadata with old-new capacity/history
accounting, and consume it once when appending the REAL geometry. Do not publish
a fake shape or clone/rebuild graph. Keep capacity growth honestly retained and
logical reservation restored on request failure; no cancel-sensitive cleanup.
Existing append_shape API/semantics unchanged; real production companion uses
same validation/publication implementation. No double-count, cross-scene/source/
draft/request/stale/replay/abandoned escape. Multiple reservations, interleaved
appends, nested child drafts and request begin/commit/rollback/abort have explicit
correct semantics. Failed commit/provider/geometry and canceled/panicking caller
cannot leave an apparently finished scene containing unresolved admission.
Prove original shape-limit/OOM/admission vs malformed stroke/provider priority
through actual public API, actual old-new capacities, source error offset,
successful consumption, failure/retry and outer-prefix/token usability. Global
allocator OOM injection is not claimed absent a safe accepted hook.

## Canonical geometry source versus use-host annotation
Accepted SourceMap.element can be an independent use host. Parser-owned path
segment/number spans remain in the referenced canonical path, while Scene
append/sealed/frame validators currently compare them against the instance host
span. Decoder preserves a failing real use-host path case. Supply explicit
source-coherent canonical geometry ownership/element proof distinct from host
SourceMap annotation, usable by new shape admission/append route and through
sealed graph/frame/query. Validate actual same immutable source/canonical token
owner and exact path spans; no blanket source-range weakening, no rewriting/
reparsing path metadata or source annotation, no fake IDs/elements or dropping
metadata. A use instance retains its host map/transform/transform-node while
actual path commands/numbers keep canonical referenced offsets. All old ordinary
path constructors/literals and strict invalid-span/forged-topology assertions
remain valid. Bounded fallible storage/actual capacities, rollback/cleanup and
work polling apply to every added table/validation loop. Prove actual referenced
path/use and multiple transformed instances, sealed graph/frame behavior, host
annotation preserved, canonical metadata unchanged; foreign/stale/wrong-kind/
out-of-element metadata rejects and request rollback discards both correctly.

# Allowed paths
- rust/crates/vexel-scene/src/** (these owning append/validation/storage boundaries)
- rust/crates/vexel-scene/tests/shape_publication.rs (new only)
- rust/crates/vexel-scene/README.md
- artifacts/{w}/** (except coordinator frozen baseline/seed)
Coordinator freezes/registers shared manifests/interfaces outside this source
scope, integrates main, seeds peers, updates STATE/docs/ledgers.

# Protected paths
All previous tests/assertions/literals/othercrates/manifests/lock/config/source603/
157oracles/frozen suites/otherreports/orders/HEAD/index/state/docs/ledgers. No
unsafe/newdeps/network/install/GPL/Cshim/unsupportedOS/agents/reviewCLI/GUI/broadUI/
stage/commit/reset/delete/APIbilling/credits. Source/tests <800lines. Safe checked
fallible bounded ownership/work. Report a genuinely missing source contract,
continue independent requirement, never substitute reduced semantics.

# Verification
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py {w} --ready
Named Sol6.1Medium lifecycle/source-ownership design; Standard existing allowance
only, stopquota/rateerrors no retry/billing changes. Full exact verification,
APIcoverage with real downstream provider/decoder examples, result <=450words.
Root independent LunaHigh review/protectedaudit/freshmain mandatory BEFORE
acceptance or peer seeding. Full media/decoder/app/runtime/finalQA/demo pending.
''';sha=hashlib.sha256(s.encode()).hexdigest();s=s.replace('parallel_safe:',f'spec_sha256: {sha}\nparallel_safe:',1);(r/'work'/f'{w}.md').write_text(s)
(a/'confirmed-sources.json').write_text(json.dumps({'shape_admission':['src/svg/shape/shape_custom.c:53','src/svg/shape/shape_geometry.c:55','rust/crates/vexel-scene/src/shape_builder.rs:6'],'canonical_source':['WO-RUST-SVG-DECODE-01 preserved use-host path failure','WO-RUST-MEDIA-SOURCE-MAP-01 accepted separate host/canonical contract'],'prerequisite':'WO-RUST-SCENE-DECODE-INITIALIZATION-01 accepted, not yet complete','not_started':True},indent=2)+'\n')
sys.path.insert(0,str(r));from scripts.ledger import append_entry
append_entry(r/'ledger.jsonl',dict(ts=datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),wo_id=w,event='ready',revision=1,base_sha=head,head_sha=head,run_id=w+'-ready',session_id='coordinator-publication',model='gpt-6.1-sol',cli_version='0.162.1',duration_s=0,usage={'input_tokens':None,'output_tokens':None},status='ready',artifacts=[f'work/{w}.md',f'artifacts/{w}/confirmed-sources.json'],note='Confirmed coupled shared append gaps: original pregeometry real shape admission, canonical path metadata vs use-host annotation. Bounded combined order READY only; starts after current initialization source acceptance. All frozen assertions protected, no implementation/acceptance claim.'))
print(w,'READY, not started; prerequisite active')
