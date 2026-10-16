from pathlib import Path
import hashlib,json,subprocess,sys
from datetime import datetime,timezone
r=Path(__file__).resolve().parents[2];w='WO-RUST-CACHE-PUBLICATION-01';a=r/'artifacts'/w;a.mkdir(exist_ok=False);head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip()
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
# Logical cache generations and physical overlap admission
Start ONLY after source-owned initialization contract is independently accepted,
fresh-main checked, and previous shared writer/reviewer terminal. Coordinator
seeds exact current accepted Scene/tests and schedules one shared writer across
this and shape-publication order; never mutate while another is live. Retain
all original accepted assertions/literals/APIs and603/157source/oracles.

Confirmed scalar R2 blocker: current RasterJob.check_pool enforces combined
16MiB Dash+Grid across all physically live old+staged caches; check_grid likewise
counts both generations against65536cells/1000000refs. Original retained bound is
per cache. Reproducer builds exactly16777216-byte cache then replacement fails
before any allocation under caller256MiB physical ledger. Staging/rebuild must
retain old usable cache on failure, so release-old-before-build/Scratchbuffers/
copy-unmeteredVec is not valid. Original review missing-cap premise is clarified
in scalar coordinator-cache-cap-clarification.md; owned per-cache guard now exists.

Provide public safe opaque same-job/source logical-cache generation/replacement
transaction (or equivalent owning state). Existing public pool APIs/default caps
stay compatible. New explicit production construction/replacement keeps each
logical cache at16MiB actual retained bytes across shapes/segments/joins/grids,
65536retained cells and1000000refs; caller's ChargeLedger separately admits ALL
physically simultaneous old/new payload/actual native capacities/old-new buffer
growth/transaction metadata. No arbitrary doubled global byte limit or zero-byte
probe/posthoc accounting. Both role/source/job/serial identity and mixed/stale/
foreign/replayed/duplicate-commit checks remain strict. Buffer handles cannot
escape into wrong cache scope and bypass logicalcap; no raw Vec/lease escapes.
Transaction capacities/history allocation/fallible cleanup honest and bounded.

Lower builder must be able to construct segment/join/shape/grid buffers using
actual accepted mutable job APIs and commit them without copying. Cache read/
composition validates actual current cache and owns a borrow preventing release/
commit invalidation; no per-sample full cache revalidation or row allocation.
Keep source-only/copy metadata and original JobDashCache literals compatible;
new provenance can be private job state/opaque ticket or additive descriptor.
Success atomically publishes actual staged handles and retires old ownership;
error/cancel/failed admission/validation/abort keeps old handles/payload/reading
usable and releases new logical ownership. Drop/abandoned stage cannot yield a
successful apparently finalized cache/job with unresolved ownership; physical
history and retained registry capacity need not rewind. Never raise or remove
frozen defaultpool caps or weaken assertions to satisfy new transaction.

Prove real near-cap replacement success with old+new physical overlap under
explicitly sufficient ledger; insufficientphysicalbudget fails leavingold; logical
aggregate limit refuses independent ofphysicalbudget/stepcap; real cells/refs
old+new remain independently bounded per generation; replay/mixedscope/wrongjob/
source/stale/cancel/abort/repeated failure cleanup and actual composition reads.
Convert scalar's existing passing defect reproducer into required success proof
ONLY in downstream adoption after this gate, not by changing its originaldata.
Do not port the lower scalar engine here. Expose complete usable lower example.

# Allowed paths
- rust/crates/vexel-scene/src/** (cache allocation/publication ownership only)
- rust/crates/vexel-scene/tests/cache_publication.rs (new only)
- rust/crates/vexel-scene/README.md
- artifacts/{w}/** (except coordinator frozen baseline/seed)
Coordinator alone freezes metadata/manifests/locks/shared registration outside
this source scope, integrates main, seeds peers and updates docs/state/ledgers.

# Protected paths
All oldtests/assertions/literals/othercrates/manifests/config/lock/source603/157/
oldorders/evidence/HEAD/index/state/docs/ledgers. No unsafe/newdeps/network/GPL/
Cshim/install/unsupportedOS/agents/reviewCLI/GUI/broadUI/stage/commit/reset/delete/
APIbilling/credits. Source/tests<800, bounded checked fallible safe ownership/work.

# Verification
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py {w} --ready
Named Sol6.1Medium resource lifetime/accounting design; Standard subscriptiononly,
stopquota/rateerrors no retry/billingchange. Exactverification/APIcoverage and
result<=450words. Root independent LunaHigh review/protectedaudit/freshmain before
acceptance or seeding. Full renderer/native/app/finalQA/release/demo pending.
''';sha=hashlib.sha256(s.encode()).hexdigest();s=s.replace('parallel_safe:',f'spec_sha256: {sha}\nparallel_safe:',1);(r/'work'/f'{w}.md').write_text(s)
source=r/'artifacts/WO-RUST-SVG-SCALAR-SAMPLE-01/coordinator-cache-cap-clarification.md';(a/'confirmed-gap.md').write_bytes(source.read_bytes())
sys.path.insert(0,str(r));from scripts.ledger import append_entry
append_entry(r/'ledger.jsonl',dict(ts=datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),wo_id=w,event='ready',revision=1,base_sha=head,head_sha=head,run_id=w+'-ready',session_id='coordinator-cache-publication',model='gpt-6.1-sol',cli_version='0.162.1',duration_s=0,usage={'input_tokens':None,'output_tokens':None},status='ready',artifacts=[f'work/{w}.md',f'artifacts/{w}/confirmed-gap.md'],note='Actual shared near-cap stage restriction confirmed;16MiB already combined-job cap, per-cache policy differs from physical oldnew. Explicit transaction scope ready after initialization acceptance, not started. Original default caps/tests preserved; no source acceptance.'))
print(w,'READY; not started, initialization review pending')
