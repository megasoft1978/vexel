from pathlib import Path
import os,json,subprocess,time
root=Path(__file__).resolve().parents[2];a=root/'artifacts/WO-RUST-NATIVE-WINDOW-PLAN-01';tree=root/'.cache/WO-RUST-NATIVE-WINDOW-PLAN-01/worker';out=tree/'artifacts/WO-RUST-NATIVE-WINDOW-PLAN-01';started=time.time();env=os.environ.copy()
for k in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(k,None)
env.update(CARGO_HOME=str(tree/'.cache/cargo'),TMPDIR=str(tree/'.cache/tmp'),CARGO_TARGET_DIR=str(tree/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
status={'pid':os.getpid(),'status':'running','model':'gpt-6-luna','effort':'low','worktree':str(tree),'started_epoch':started};p=a/'repair-v1-status.json';p.write_text(json.dumps(status,indent=2)+'\n')
cmd=['codex','exec','--ignore-user-config','--profile','economy','--model','gpt-6-luna','-c','model_reasoning_effort="low"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--color','never','--json','--cd',str(tree),'resume','--output-last-message',str(a/'repair-v1-last-message.md'),'01a13b5d-2e2b-7e73-86f8-68ce0dfb29ad','-']
with (a/'repair-v1.prompt.txt').open('rb') as stdin,(a/'repair-v1.events.jsonl').open('wb') as stdout,(a/'repair-v1.stderr.log').open('wb') as stderr:
 child=subprocess.Popen(cmd,stdin=stdin,stdout=stdout,stderr=stderr,env=env);status['child_pid']=child.pid;p.write_text(json.dumps(status,indent=2)+'\n');code=child.wait()
status.update(status='completed' if code==0 else 'failed',exit_code=code,duration_seconds=time.time()-started);p.write_text(json.dumps(status,indent=2)+'\n')
