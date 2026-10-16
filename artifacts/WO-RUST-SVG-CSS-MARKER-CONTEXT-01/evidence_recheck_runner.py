from pathlib import Path
import json,subprocess,time,os,hashlib
root=Path(__file__).resolve().parents[2];a=root/'artifacts/WO-RUST-SVG-CSS-MARKER-CONTEXT-01'
env=os.environ.copy()
for k in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(k,None)
started=time.time();status={'pid':os.getpid(),'status':'running','model':'gpt-6-luna','effort':'low','started_epoch':started}
(a/'evidence-recheck-source-hashes.json').write_text(json.dumps({str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in (root/'rust/crates/vexel-svg-css').rglob('*') if p.is_file()},indent=2)+'\n')
(a/'evidence-recheck-status.json').write_text(json.dumps(status,indent=2)+'\n')
cmd=['codex','exec','--ignore-user-config','--profile','economy','--model','gpt-6-luna','-c','model_reasoning_effort="low"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','read-only','--color','never','--json','--cd',str(root),'--output-last-message',str(a/'evidence-recheck.md'),'resume','01a13c07-de2d-7370-9d00-de1b713fa122','-']
with (a/'evidence-recheck.prompt.txt').open('rb') as stdin,(a/'evidence-recheck.events.jsonl').open('wb') as stdout,(a/'evidence-recheck.stderr.log').open('wb') as stderr:
    process=subprocess.Popen(cmd,stdin=stdin,stdout=stdout,stderr=stderr,env=env)
    status['child_pid']=process.pid;(a/'evidence-recheck-status.json').write_text(json.dumps(status,indent=2)+'\n')
    code=process.wait()
status.update(status='completed' if code==0 else 'failed',exit_code=code,duration_seconds=time.time()-started)
(a/'evidence-recheck-status.json').write_text(json.dumps(status,indent=2)+'\n')
