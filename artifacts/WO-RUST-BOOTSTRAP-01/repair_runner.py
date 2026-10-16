from pathlib import Path
import subprocess,json,sys,time,os
root=Path(__file__).resolve().parents[2];name=sys.argv[1];session=sys.argv[2]
task=json.loads((root/'artifacts/WO-RUST-BOOTSTRAP-01/foundation-batch.json').read_text())[name];tree=Path(task['worktree']);a=tree/'artifacts'/task['wo'];started=time.time();env=os.environ.copy()
for key in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(key,None)
env.update(CARGO_HOME=str(tree/'.cache/cargo'),TMPDIR=str(tree/'.cache/tmp'),CARGO_TARGET_DIR=str(tree/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
status={'name':name,'session':session,'pid':os.getpid(),'status':'running','started_epoch':started};statuspath=root/'artifacts'/task['wo']/'repair-status.json';statuspath.write_text(json.dumps(status,indent=2)+'\n')
cmd=['codex','exec','--ignore-user-config','--profile','economy','--model','gpt-6-luna','-c','model_reasoning_effort="high"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--cd',str(tree),'--json','resume','--output-last-message',str(a/'repair-result.md'),session,'-']
with (a/'repair.prompt.txt').open('rb') as stdin,(a/'repair.events.jsonl').open('wb') as stdout,(a/'repair.stderr.log').open('wb') as stderr:
 result=subprocess.run(cmd,stdin=stdin,stdout=stdout,stderr=stderr,env=env)
status.update(status='completed' if result.returncode==0 else 'failed',exit_code=result.returncode,duration_seconds=time.time()-started);statuspath.write_text(json.dumps(status,indent=2)+'\n')
