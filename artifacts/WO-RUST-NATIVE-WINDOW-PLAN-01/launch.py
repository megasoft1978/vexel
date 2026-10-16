from pathlib import Path
import json,os,subprocess,shutil,hashlib
root=Path(__file__).resolve().parents[2];a=root/'artifacts/WO-RUST-NATIVE-WINDOW-PLAN-01';tree=root/'.cache/WO-RUST-NATIVE-WINDOW-PLAN-01/worker';assert not tree.exists();tree.parent.mkdir(parents=True,exist_ok=True)
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
(tree/'artifacts/WO-RUST-NATIVE-WINDOW-PLAN-01').mkdir(parents=True,exist_ok=True)
(a/'overlay-baseline.json').write_text(json.dumps({'root':str(root),'worktree':str(tree),'head':head,'files':hashes},indent=2)+'\n')
prompt='''Complete only frozen work/WO-RUST-NATIVE-WINDOW-PLAN-01.md in this current-source isolated tree. Read OWNER/AGENTS/order before source. Produce concrete implementation-ready full native-window lane, no generic second roadmap. Entire native window/geometry/input/IME/ticks/lifecycle/queue/teardown source/API map, pinned existing objc2 declaration links with mac13 availability, accepted MainThread/guard_native_call and UI symbols, exact split to mandatory file/menu/AX/text/presenter later gates, ready implementation order and safeguards. Your only writes: artifacts/WO-RUST-NATIVE-WINDOW-PLAN-01/result.md <=550words, parity-map.json, ready-window-order.md, verification.txt. No manifests/source/config/state/ledger/docs/orders/previousartifacts changes; no build/test/network/install/dependency/GUI/sourcefetch/agents/reset/stage/commit/delete. No new GPL source. Use existing subscription only Standard LunaHigh, stop quota/rate errors no retries/API billing. Only current accepted code was snapshotted; new root kernel contract work is not a peer dependency. Do scoped searches, read primary local binding declarations/source only as needed, keep tool/report output narrow. Successful exit creates candidate plan, root separately reviews/finalizes before implementation. Public wrapper interfaces must be Rust-owned value types, opaque main-thread native handles; callbacks catch unwind and native exceptions, weak routing invalidated before destruction. Do not silently lose input release/close events or use keyboard characters as substitute for IME. No completed native window/GUI/mac13runtime/performance claim.\n'''
(a/'worker.prompt.txt').write_text(prompt);(tree/'artifacts/WO-RUST-NATIVE-WINDOW-PLAN-01/worker.prompt.txt').write_text(prompt)
print(tree,len(files),flush=True)
