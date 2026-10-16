from pathlib import Path
import os,json,subprocess,time
root=Path(__file__).resolve().parents[2];a=root/'artifacts'/'WO-RUST-SVG-MEDIA-01';tree=root/'.cache'/'WO-RUST-SVG-MEDIA-01'/'worker';out=tree/'artifacts'/'WO-RUST-SVG-MEDIA-01';started=time.time();env=os.environ.copy()
for key in ('OPENAI_API_KEY','CODEX_API_KEY'):env.pop(key,None)
env.update(CARGO_HOME=str(tree/'.cache/cargo'),TMPDIR=str(tree/'.cache/tmp'),CARGO_TARGET_DIR=str(tree/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
s={'pid':os.getpid(),'status':'running','model':'gpt-6.1-sol','effort':'medium','worktree':str(tree),'started_epoch':started};p=a/'accepted-integration-resume-status.json';p.write_text(json.dumps(s,indent=2)+'\n')
cmd=['codex','exec','--ignore-user-config','--profile','complex','--model','gpt-6.1-sol','-c','model_reasoning_effort="medium"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','workspace-write','--color','never','--json','--cd',str(tree),'--output-last-message',str(out/'accepted-integration-resume-last-message.md'),'resume','01a13bc2-c530-75f2-bbf6-65b821addd14','-']
with (a/'accepted-integration-resume.prompt.txt').open('rb') as i,(a/'accepted-integration-resume.events.jsonl').open('wb') as o,(a/'accepted-integration-resume.stderr.log').open('wb') as e:
 c=subprocess.Popen(cmd,stdin=i,stdout=o,stderr=e,env=env);s['child_pid']=c.pid;p.write_text(json.dumps(s,indent=2)+'\n');code=c.wait()
s.update(status='completed' if code==0 else 'failed',exit_code=code,duration_seconds=time.time()-started);p.write_text(json.dumps(s,indent=2)+'\n')
