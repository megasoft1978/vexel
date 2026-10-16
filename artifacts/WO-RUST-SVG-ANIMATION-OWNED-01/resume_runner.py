from pathlib import Path
import subprocess,json,sys,time,os
root=Path(__file__).resolve().parents[2];name,mode,label,session=sys.argv[1:5];model=sys.argv[5] if len(sys.argv)>5 else 'gpt-6-luna';effort=sys.argv[6] if len(sys.argv)>6 else 'high'
task=json.loads((root/'artifacts/WO-RUST-SVG-ANIMATION-OWNED-01/batch.json').read_text())[name];tree=Path(task['worktree']);a=(tree if mode=='repair' else root)/'artifacts'/task['wo'];started=time.time();env=os.environ.copy()
for key in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(key,None)
env.update(CARGO_HOME=str(tree/'.cache/cargo'),TMPDIR=str(tree/'.cache/tmp'),CARGO_TARGET_DIR=str(tree/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
status={'name':name,'wo':task['wo'],'session':session,'pid':os.getpid(),'mode':mode,'model':model,'effort':effort,'status':'running','started_epoch':started};statuspath=root/'artifacts'/task['wo']/(label+'-status.json');statuspath.write_text(json.dumps(status,indent=2)+'\n')
cmd=['codex','exec','--ignore-user-config','--profile','economy','--model',model,'-c','model_reasoning_effort="'+effort+'"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write' if mode=='repair' else 'read-only','--cd',str(tree),'--json','resume','--output-last-message',str(a/(label+'-result.md')),session,'-']
with (a/(label+'.prompt.txt')).open('rb') as stdin,(a/(label+'.events.jsonl')).open('wb') as stdout,(a/(label+'.stderr.log')).open('wb') as stderr:result=subprocess.run(cmd,stdin=stdin,stdout=stdout,stderr=stderr,env=env)
status.update(status='completed' if result.returncode==0 else 'failed',exit_code=result.returncode,duration_seconds=time.time()-started);statuspath.write_text(json.dumps(status,indent=2)+'\n')
