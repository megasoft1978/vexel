from pathlib import Path
import json,os,subprocess,shutil,hashlib
root=Path(__file__).resolve().parents[2];a=root/'artifacts/WO-RUST-NATIVE-WINDOW-01';tree=root/'.cache/WO-RUST-NATIVE-WINDOW-01/worker';assert not tree.exists();tree.parent.mkdir(parents=True,exist_ok=True)
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip();subprocess.run(['git','worktree','add','--detach',str(tree),head],cwd=root,check=True,capture_output=True)
files=set()
for name in ['app','src','tests','docs','work','rust','scripts','bench','packaging','resources','.cargo']:
 start=root/name
 if not start.exists():continue
 for p,d,f in os.walk(start):
  d[:]=[x for x in d if x not in ['.git','.cache','build','target','__pycache__','node_modules']]
  for fn in f:
   if fn!='.DS_Store':files.add(str((Path(p)/fn).relative_to(root)))
for p in root.iterdir():
 if p.is_file() and p.name!='.DS_Store':files.add(p.name)
for name in ['WO-RUST-NATIVE-PLAN-01','WO-RUST-NATIVE-BINDINGS-AUDIT-01','WO-RUST-NATIVE-BINDINGS-SETUP-01']:
 for p in (root/'artifacts'/name).glob('*'):
  if p.is_file() and p.suffix in ['.md','.json']:files.add(str(p.relative_to(root)))
hashes={}
for rel in sorted(files):
 dest=tree/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(root/rel,dest);hashes[rel]=hashlib.sha256((root/rel).read_bytes()).hexdigest()
for name in ['.cache/cargo','.cache/tmp','build/rust']:(tree/name).mkdir(parents=True,exist_ok=True)
shutil.copytree(root/'.cache/cargo/registry',tree/'.cache/cargo/registry')
(tree/'artifacts/WO-RUST-NATIVE-WINDOW-01').mkdir(parents=True,exist_ok=True)
(a/'overlay-baseline.json').write_text(json.dumps({'root':str(root),'worktree':str(tree),'head':head,'files':hashes},indent=2)+'\n')
prompt="""Implement ONLY work/WO-RUST-NATIVE-WINDOW-01.md in isolated current-source tree. Read OWNER/AGENTS/order before code. Full accepted real nativewindow/lifecycle/geometry/events/IME/queue/ticks/appearance/teardown scope, all namedsource methods mapped; no fake backend or smaller replacement. Embedded readiness artifact's candidate header is historical: Native WINDOW PLAN R2 is accepted; this frozen READY implementation order authorizes source work. You own new native_window/** and tests/native_window*.rs plus exact lib.rs module export bridge and ownartifacts. Protect current manifest/config/locks/main_thread/native_boundary/original tests/peers/legacy/docs/ledgers/index. Actual selected pins/guard boundary accepted, no extra feature/package/install/sourcefetch/network/GPL/Cshim/unsupportedOS/recursive agents/stage/commit/reset/delete. Small audited unsafe platform calls only, guard outbound/inbound native callbacks, no fabricated main-thread marker, no nativeptr public. Owned InputReset/pending/IME state and weak native registry must be complete; no model/RefCell borrow across reentrant native calls. Provide internal renderer surface access seam but no fake presenter. No GUI launch/automation/broadUIE2E; scoped pure unit/compile-affinity and cargo +stable locked offline check/test permitted and required. Source/tests <800; exact original7 boundary checks preserved. Missing true selected-feature/API: report exact blocker before substituting semantics. Produce own result.md <=400words, parity-map.json, verification.txt. Standard LunaHigh existing subscription only, no purchasedcredits/APIbilling; stop quota/rate errors. Successful exit onlycandidate, root separately audits/reviews/integrates. Keep searches scoped and outputs narrow.
"""
(a/'worker.prompt.txt').write_text(prompt);(tree/'artifacts/WO-RUST-NATIVE-WINDOW-01/worker.prompt.txt').write_text(prompt)
print(tree,len(files),flush=True)
