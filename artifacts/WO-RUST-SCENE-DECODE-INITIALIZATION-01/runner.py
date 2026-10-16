from pathlib import Path
import os,json,subprocess,time
r=Path(__file__).resolve().parents[2];a=r/'artifacts'/'WO-RUST-SCENE-DECODE-INITIALIZATION-01';t=r/'.cache/WO-RUST-RENDER-INTEGRATION-CONTRACTS-01/worker';out=t/'artifacts'/'WO-RUST-SCENE-DECODE-INITIALIZATION-01';started=time.time();env=os.environ.copy()
for key in ('OPENAI_API_KEY','CODEX_API_KEY'):env.pop(key,None)
env.update(CARGO_HOME=str(t/'.cache/cargo'),TMPDIR=str(t/'.cache/tmp'),CARGO_TARGET_DIR=str(t/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
s={'pid':os.getpid(),'status':'running','model':'gpt-6.1-sol','effort':'medium','worktree':str(t),'started_epoch':started};p=a/'process-status.json';p.write_text(json.dumps(s,indent=2)+'\n')
cmd=['codex','exec','--ignore-user-config','--profile','complex','--model','gpt-6.1-sol','-c','model_reasoning_effort="medium"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--color','never','--json','--cd',str(t),'--output-last-message',str(out/'last-message.md'),'resume','01a13dcf-843b-7ed1-9b5d-0854d9f06b6a','-']
with (a/'worker.prompt.txt').open('rb') as i,(a/'worker.events.jsonl').open('wb') as o,(a/'worker.stderr.log').open('wb') as e:
 c=subprocess.Popen(cmd,stdin=i,stdout=o,stderr=e,env=env);s['child_pid']=c.pid;p.write_text(json.dumps(s,indent=2)+'\n');code=c.wait()
s.update(status='completed' if code==0 else 'failed',exit_code=code,duration_seconds=time.time()-started);p.write_text(json.dumps(s,indent=2)+'\n')
