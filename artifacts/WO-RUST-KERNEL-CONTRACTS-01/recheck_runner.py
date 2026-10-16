from pathlib import Path
import json,subprocess,time,os,hashlib
root=Path(__file__).resolve().parents[2];a=root/'artifacts/WO-RUST-KERNEL-CONTRACTS-01'
env=os.environ.copy()
for k in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(k,None)
started=time.time();status={'pid':os.getpid(),'status':'running','model':'gpt-6-luna','effort':'high','started_epoch':started}
(a/'recheck-v1-source-hashes.json').write_text(json.dumps({str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in (root/'rust/crates/vexel-scene').rglob('*') if p.is_file()},indent=2)+'\n')
(a/'recheck-v1-status.json').write_text(json.dumps(status,indent=2)+'\n')
cmd=['codex','exec','--ignore-user-config','--profile','economy','--model','gpt-6-luna','-c','model_reasoning_effort="high"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','read-only','--color','never','--json','--cd',str(root),'resume','--output-last-message',str(a/'recheck-v1-result.md'),'01a13b76-c4dc-7f92-acd5-69fe98afa49e','-']
with (a/'recheck-v1.prompt.txt').open('rb') as stdin,(a/'recheck-v1.events.jsonl').open('wb') as stdout,(a/'recheck-v1.stderr.log').open('wb') as stderr:
    process=subprocess.Popen(cmd,stdin=stdin,stdout=stdout,stderr=stderr,env=env)
    status['child_pid']=process.pid;(a/'recheck-v1-status.json').write_text(json.dumps(status,indent=2)+'\n')
    code=process.wait()
status.update(status='completed' if code==0 else 'failed',exit_code=code,duration_seconds=time.time()-started)
(a/'recheck-v1-status.json').write_text(json.dumps(status,indent=2)+'\n')
