from pathlib import Path
import subprocess,json,os,time
root=Path(__file__).resolve().parents[2];a=root/'artifacts/WO-RUST-RENDER-CONTRACT-PLAN-01';started=time.time()
status={'pid':os.getpid(),'model':'gpt-6.1-sol','effort':'medium','status':'running','started_epoch':started};p=a/'process-status.json';p.write_text(json.dumps(status,indent=2)+'\n')
env=os.environ.copy()
for k in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(k,None)
env.update(CARGO_HOME=str(root/'.cache/cargo'),TMPDIR=str(root/'.cache/tmp'),CARGO_TARGET_DIR=str(root/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
cmd=['codex','exec','--ignore-user-config','--profile','complex','--model','gpt-6.1-sol','-c','model_reasoning_effort="medium"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--color','never','--json','--cd',str(root),'--output-last-message',str(a/'result.md'),'-']
with (a/'worker.prompt.txt').open('rb') as i,(a/'worker.events.jsonl').open('wb') as o,(a/'worker.stderr.log').open('wb') as e:r=subprocess.run(cmd,stdin=i,stdout=o,stderr=e,env=env)
status.update(status='completed' if r.returncode==0 else 'failed',exit_code=r.returncode,duration_seconds=time.time()-started);p.write_text(json.dumps(status,indent=2)+'\n')
