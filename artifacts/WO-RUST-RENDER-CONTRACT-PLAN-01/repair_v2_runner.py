from pathlib import Path
import subprocess,json,os,time
root=Path(__file__).resolve().parents[2];a=root/'artifacts/WO-RUST-RENDER-CONTRACT-PLAN-01';started=time.time()
status={'pid':os.getpid(),'model':'gpt-6-luna','effort':'low','status':'running','started_epoch':started};p=a/'repair-v2-status.json';p.write_text(json.dumps(status,indent=2)+'\n')
env=os.environ.copy()
for k in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(k,None)
env.update(CARGO_HOME=str(root/'.cache/cargo'),TMPDIR=str(root/'.cache/tmp'),CARGO_TARGET_DIR=str(root/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
cmd=['codex','exec','--ignore-user-config','--profile','economy','--model','gpt-6-luna','-c','model_reasoning_effort="low"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--color','never','--json','--cd',str(root),'--output-last-message',str(a/'repair-v2-result.md'),'-']
with (a/'repair-v2.prompt.txt').open('rb') as i,(a/'repair-v2.events.jsonl').open('wb') as o,(a/'repair-v2.stderr.log').open('wb') as e:r=subprocess.run(cmd,stdin=i,stdout=o,stderr=e,env=env)
status.update(status='completed' if r.returncode==0 else 'failed',exit_code=r.returncode,duration_seconds=time.time()-started);p.write_text(json.dumps(status,indent=2)+'\n')
