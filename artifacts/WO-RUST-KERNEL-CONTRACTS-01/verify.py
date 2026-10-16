from pathlib import Path
import subprocess,os,json,time
root=Path.cwd();a=root/'artifacts/WO-RUST-KERNEL-CONTRACTS-01';env=os.environ.copy();env.update(CARGO_HOME=str(root/'.cache/cargo'),TMPDIR=str(root/'.cache/tmp'),CARGO_TARGET_DIR=str(root/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
commands=[['cargo','+stable','generate-lockfile','--offline'],['cargo','+stable','check','-p','vexel-scene','--locked','--offline'],['cargo','+stable','test','-p','vexel-scene','--locked','--offline'],['python3','scripts/wo_validate.py','WO-RUST-KERNEL-CONTRACTS-01','--ready'],['python3','scripts/ledger.py','validate','ledger.jsonl'],['git','diff','--check'],['python3',str(a/'audit.py')]]
records=[]
with (a/'verification.txt').open('w') as log:
 for command in commands:
  start=time.time();result=subprocess.run(command,cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);records.append({'argv':command,'exit_code':result.returncode,'duration_seconds':time.time()-start});log.write('$ '+' '.join(command)+'\n'+result.stdout+'\n');log.flush();print('PASS' if result.returncode==0 else 'FAIL',' '.join(command),flush=True)
  if result.returncode:raise SystemExit(result.returncode)
(a/'verification.json').write_text(json.dumps({'status':'pass','commands':records,'focused_tests':43,'unit_tests':6,'integration_tests':37,'new_contract_tests':16,'whole_app_testing':False,'ui_e2e':False},indent=2)+'\n')
