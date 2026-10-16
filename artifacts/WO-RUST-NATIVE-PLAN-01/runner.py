from pathlib import Path
import subprocess,json,os,time
root=Path(__file__).resolve().parents[2];a=root/'artifacts/WO-RUST-NATIVE-PLAN-01';started=time.time();s={'pid':os.getpid(),'model':'gpt-6.1-sol','effort':'medium','status':'running','started_epoch':started};(a/'process-status.json').write_text(json.dumps(s,indent=2)+'\n');env=os.environ.copy()
for k in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(k,None)
cmd=['codex','exec','--ignore-user-config','--profile','complex','--model','gpt-6.1-sol','-c','model_reasoning_effort="medium"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','read-only','--color','never','--json','--cd',str(root),'--output-last-message',str(a/'result.md'),'-']
with (a/'plan.prompt.txt').open('rb') as i,(a/'worker.events.jsonl').open('wb') as o,(a/'worker.stderr.log').open('wb') as e:r=subprocess.run(cmd,stdin=i,stdout=o,stderr=e,env=env)
s.update(status='completed' if r.returncode==0 else 'failed',exit_code=r.returncode,duration_seconds=time.time()-started);(a/'process-status.json').write_text(json.dumps(s,indent=2)+'\n')
