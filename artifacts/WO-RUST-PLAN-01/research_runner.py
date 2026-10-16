from pathlib import Path
import subprocess,json,sys,time,os
root=Path(__file__).resolve().parents[2];a=root/'artifacts/WO-RUST-PLAN-01';name=sys.argv[1];effort=sys.argv[2];out=a/'research'/name;started=time.time()
status={'name':name,'model':'gpt-6-luna','effort':effort,'status':'running','pid':os.getpid(),'started_epoch':started}
out.with_suffix('.status.json').write_text(json.dumps(status,indent=2)+'\n')
env=os.environ.copy()
for key in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(key,None)
cmd=['codex','exec','--profile','economy','--model','gpt-6-luna','-c','model_reasoning_effort="'+effort+'"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="live"','--sandbox','read-only','--color','never','--json','--cd',str(root),'--output-last-message',str(out.with_suffix('.md')),'-']
with (a/'research'/f'{name}.prompt.txt').open('rb') as prompt,out.with_suffix('.events.jsonl').open('wb') as events,out.with_suffix('.stderr.log').open('wb') as errors:
 result=subprocess.run(cmd,stdin=prompt,stdout=events,stderr=errors,env=env)
status.update(status='completed' if result.returncode==0 else 'failed',exit_code=result.returncode,duration_seconds=time.time()-started);out.with_suffix('.status.json').write_text(json.dumps(status,indent=2)+'\n')
