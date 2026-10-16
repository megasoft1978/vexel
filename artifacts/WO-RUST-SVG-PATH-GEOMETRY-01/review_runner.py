from pathlib import Path
import subprocess,json,sys,time,os
root=Path(__file__).resolve().parents[2];name=sys.argv[1]
task=json.loads((root/'artifacts/WO-RUST-SVG-PATH-GEOMETRY-01/batch.json').read_text())[name];tree=Path(task['worktree']);a=root/'artifacts'/task['wo'];started=time.time()
env=os.environ.copy()
for key in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(key,None)
status={'name':name,'wo':task['wo'],'pid':os.getpid(),'model':'gpt-6-luna','effort':'medium','status':'running','started_epoch':started}
statuspath=a/'review-status.json';statuspath.write_text(json.dumps(status,indent=2)+'\n')
cmd=['codex','exec','--ignore-user-config','--profile','economy','--model','gpt-6-luna','-c','model_reasoning_effort="'+'medium'+'"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','read-only','--color','never','--json','--cd',str(tree),'--output-last-message',str(a/'review.md'),'-']
with (a/'review.prompt.txt').open('rb') as stdin,(a/'review.events.jsonl').open('wb') as stdout,(a/'review.stderr.log').open('wb') as stderr:
 result=subprocess.run(cmd,stdin=stdin,stdout=stdout,stderr=stderr,env=env)
status.update(status='completed' if result.returncode==0 else 'failed',exit_code=result.returncode,duration_seconds=time.time()-started);statuspath.write_text(json.dumps(status,indent=2)+'\n')
