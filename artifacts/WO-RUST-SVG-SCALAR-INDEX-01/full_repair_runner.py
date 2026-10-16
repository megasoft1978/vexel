from pathlib import Path
import os,json,subprocess,time
root=Path(__file__).resolve().parents[2];own=root/'artifacts'/'WO-RUST-SVG-SCALAR-INDEX-01';tree=root/'.cache'/'WO-RUST-SVG-SCALAR-INDEX-01'/'worker';out=tree/'artifacts'/'WO-RUST-SVG-SCALAR-INDEX-01';started=time.time();env=os.environ.copy()
for key in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(key,None)
env.update(CARGO_HOME=str(tree/'.cache/cargo'),TMPDIR=str(tree/'.cache/tmp'),CARGO_TARGET_DIR=str(tree/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
status={'pid':os.getpid(),'status':'running','model':'gpt-6.1-sol','effort':'medium','worktree':str(tree),'started_epoch':started};p=own/'full-repair-status.json';p.write_text(json.dumps(status,indent=2)+'\n')
cmd=['codex','exec','--ignore-user-config','--profile','complex','--model','gpt-6.1-sol','-c','model_reasoning_effort="medium"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--color','never','--json','--cd',str(tree),'--output-last-message',str(out/'full-repair-last-message.md'),'resume','01a13bc2-d03c-70e0-a0f4-16a422c8e04c','-']
with (own/'full-repair.prompt.txt').open('rb') as stdin,(own/'full-repair.events.jsonl').open('wb') as stdout,(own/'full-repair.stderr.log').open('wb') as stderr:
 child=subprocess.Popen(cmd,stdin=stdin,stdout=stdout,stderr=stderr,env=env);status['child_pid']=child.pid;p.write_text(json.dumps(status,indent=2)+'\n');code=child.wait()
status.update(status='completed' if code==0 else 'failed',exit_code=code,duration_seconds=time.time()-started);p.write_text(json.dumps(status,indent=2)+'\n')
