from pathlib import Path
import subprocess,json,os,time
root=Path(__file__).resolve().parents[2];a=root/'artifacts/WO-RUST-WAVE4-SETUP-01';started=time.time()
status={'pid':os.getpid(),'model':'gpt-6-luna','effort':'medium','status':'running','started_epoch':started}
(a/'review-status.json').write_text(json.dumps(status,indent=2)+'\n');env=os.environ.copy()
for k in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(k,None)
cmd=['codex','exec','--ignore-user-config','--profile','economy','--model','gpt-6-luna','-c','model_reasoning_effort="medium"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','read-only','--color','never','--json','--cd',str(root),'--output-last-message',str(a/'review.md'),'-']
with (a/'review.prompt.txt').open('rb') as i,(a/'review.events.jsonl').open('wb') as o,(a/'review.stderr.log').open('wb') as e:r=subprocess.run(cmd,stdin=i,stdout=o,stderr=e,env=env)
status.update(status='completed' if r.returncode==0 else 'failed',exit_code=r.returncode,duration_seconds=time.time()-started);(a/'review-status.json').write_text(json.dumps(status,indent=2)+'\n')
