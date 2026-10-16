from pathlib import Path
import subprocess,json,sys,time,os
root=Path(__file__).resolve().parents[2];name=sys.argv[1]
batch=json.loads((root/'artifacts/WO-RUST-WAVE2-SETUP-01/batch.json').read_text());task=batch[name];tree=Path(task['worktree']);wo=task['wo'];a=tree/'artifacts'/wo;started=time.time()
status={'name':name,'wo':wo,'pid':os.getpid(),'model':'gpt-6-luna','effort':task['effort'],'status':'running','started_epoch':started}
(root/'artifacts'/wo/'process-status.json').write_text(json.dumps(status,indent=2)+'\n')
env=os.environ.copy()
for key in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(key,None)
env.update(CARGO_HOME=str(tree/'.cache/cargo'),TMPDIR=str(tree/'.cache/tmp'),CARGO_TARGET_DIR=str(tree/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
cmd=['codex','exec','--ignore-user-config','--profile','economy','--model','gpt-6-luna','-c','model_reasoning_effort="'+task['effort']+'"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--color','never','--json','--cd',str(tree),'--output-last-message',str(a/'result.md'),'-']
with (a/'worker.prompt.txt').open('rb') as stdin,(a/'worker.events.jsonl').open('wb') as stdout,(a/'worker.stderr.log').open('wb') as stderr:
 result=subprocess.run(cmd,stdin=stdin,stdout=stdout,stderr=stderr,env=env)
status.update(status='completed' if result.returncode==0 else 'failed',exit_code=result.returncode,duration_seconds=time.time()-started)
(root/'artifacts'/wo/'process-status.json').write_text(json.dumps(status,indent=2)+'\n')
