from pathlib import Path
import hashlib,subprocess,json,sys
from datetime import datetime,timezone
r=Path(__file__).resolve().parents[2];w='WO-RUST-SCENE-DECODE-INITIALIZATION-01';a=r/'artifacts'/w;a.mkdir(exist_ok=False);head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip()
text=f'''---
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
# Source-owned provisional initialization and original decoder priority
Start the single isolated shared writer ONLY after paint-channel contract source
is independently reviewed, freshly main-checked and accepted. Seed exact accepted
main Scene source/tests and preserve every prior assertion/literal/API. No live
shared writer or reviewer may overlap mutations. Coordinator manages all seeds.

Confirmed original contracts: src/svg/shape/traverse.c lines433-448 returns for
root display:none before transform/viewport parsing; decode.c lines198-216 walks
source and validates root_seen BEFORE actual animation parsing. Current
SceneBuilder::new requires parsed OwnedAnimation and a valid nonzero viewport
before traversal. A decoder cannot preserve priority or hidden-root behavior.
Provide explicit publicly usable bounded source-owned provisional initialization
and finalization. Existing constructor, consumers, views and tests remain source
compatible. Do not invent a 1x1/default viewport, parse an alternate source, expose
an apparently finished empty animation, or parse full source twice to hide state.
Represent unresolved initialization explicitly; production freeze/render/finished
SceneGraph must be impossible until the actual same-source animation and root
viewport/hidden disposition are finalized. Existing real shape/text/image/typed
owner/request/draft/child planner operations must work during traversal through
actual accepted SceneBuilder contracts. No alternate unmetered graph or clone-
and-rebuild. Use shared algorithms/storage rather than divergent scene models.

The original hidden display:none root may produce an empty document without
parsing malformed/missing viewport values; prove source-preserving explicit
empty-hidden representation and its safe completed view/frame/query disposition.
Keep normal nonzero viewport validation strict; never accept arbitrary forged
zero or unresolved state as a normal drawable scene. Preserve root_seen,
transform/style/attribute errors before skipped branches and original ordering.
Child/scene ownership, stale handles and source-map validation remain strict.
Actual late animation installation is same canonical source, validates generation
and ownership, checked actual-capacity/live-old-new/history accounting and bounds,
fallible/error/cancel cleanup. Pending state/handles cannot escape through a
successful freeze after partial failure. No double-charge or silently discarded
owned animation, fake empty backend, implicit finish or globalOOM/RSS claims.

Prove through public actual builder operations: traversal/geometry failure wins
before invalid animation; later actual animation parse failure is propagated;
hidden-root missing/malformed viewport avoids viewport failure but still obeys
later animation diagnostics; visible malformed viewport rejects; unresolved or
wrong-source/double finalization rejects; failure/cancel/budget overlap leaves
state honestly usable or explicitly abortable; normal path/scene/frame original
assertions remain unchanged. Decoder adapter implementation is downstream; use
explicit source fixtures, not an invented production lower engine.

# Allowed paths
- rust/crates/vexel-scene/src/** (only initialization/finalization/storage guards)
- rust/crates/vexel-scene/tests/decode_initialization.rs (new only)
- rust/crates/vexel-scene/README.md
- rust/crates/vexel-svg-animation/src/** (only minimal explicit owning provisional state API if required)
- rust/crates/vexel-svg-animation/tests/decode_initialization.rs (new only if required)
- artifacts/{w}/** (except coordinator frozen baseline/seed)
Coordinator owns work-order freeze, all shared manifests/locks/registration outside
these allowed source modules, main integration, peer seeding, STATE and ledgers.

# Protected paths
All existing tests/assertions/literals, other crates/engines, manifests/lock/
config, original603/157suites/fixtures/goldens/source, other reports/evidence/
orders, HEAD/index/state/docs/ledgers. No unsafe/newdeps/network/install/GPL/Cshim/
unsupportedOS/agents/GUI/broadUI/stage/commit/reset/delete/APIbilling/credits.
Source/tests <800lines; safe checked fallible bounded ownership and work controls.

# Verification
cargo +stable check -p vexel-scene -p vexel-svg-animation --locked --offline
cargo +stable test -p vexel-scene -p vexel-svg-animation --locked --offline
python3 scripts/wo_validate.py {w} --ready
Named Sol6.1Medium escalation: cross-file scene/view/ownership initialization
cannot be resolved in decoder-owned scope. Standard existing subscription only;
stop quota/rate errors, no restart loop/billing changes. Output exact logs and
verification, APIcoverage including downstream integration, result <=450words.
Candidate requires independent LunaHigh review/protected audit/fresh main checks
before acceptance or seeding. No full renderer/native/app/performance/finalQA claim.
'''
sha=hashlib.sha256(text.encode()).hexdigest();text=text.replace('parallel_safe:',f'spec_sha256: {sha}\nparallel_safe:',1);(r/'work'/f'{w}.md').write_text(text)
shutil_source=r/'artifacts/WO-RUST-RENDER-CONSUMER-RESUME-01/confirmed-decode-initialization-gap.json';(a/'confirmed-gap.json').write_bytes(shutil_source.read_bytes())
sys.path.insert(0,str(r));from scripts.ledger import append_entry
append_entry(r/'ledger.jsonl',dict(ts=datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),wo_id=w,event='ready',revision=1,base_sha=head,head_sha=head,run_id=w+'-ready',session_id='coordinator-initialization',model='gpt-6.1-sol',cli_version='0.162.1',duration_s=0,usage={'input_tokens':None,'output_tokens':None},status='ready',artifacts=[f'work/{w}.md',f'artifacts/{w}/confirmed-gap.json'],note='Confirmed original hidden-root viewport skip and post-traversal animation parse. Bounded explicit initialization contract ready; cannot start shared writer before paint-channel acceptance. No implementation/acceptance claim.'))
print(w,'READY; not started, prerequisite pending')
