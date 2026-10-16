from pathlib import Path
import json,subprocess,time,os,hashlib
root=Path(__file__).resolve().parents[2];a=root/'artifacts/WO-RUST-SVG-DEFINITIONS-01'
env=os.environ.copy()
for k in ['OPENAI_API_KEY','CODEX_API_KEY']:env.pop(k,None)
tree=root/'.cache/WO-RUST-SVG-DEFINITIONS-01/worker'
prior=json.loads((a/'accepted-integration-resume-status.json').read_text());assert prior['status']=='completed'
assert not subprocess.run(['ps','-p',str(prior['child_pid']),'-o','comm='],capture_output=True,text=True).stdout.strip()
assert json.loads((a/'accepted-input-terminal-audit.json').read_text())['verdict']=='PASS'
started=time.time();status={'pid':os.getpid(),'status':'running','model':'gpt-6-luna','effort':'high','started_epoch':started}
(a/'full-review-source-hashes.json').write_text(json.dumps({str(p.relative_to(tree)):hashlib.sha256(p.read_bytes()).hexdigest() for p in (tree/'rust/crates/vexel-svg-definitions').rglob('*') if p.is_file()},indent=2)+'\n')
(a/'full-review-status.json').write_text(json.dumps(status,indent=2)+'\n')
cmd=['codex','exec','--ignore-user-config','--profile','economy','--model','gpt-6-luna','-c','model_reasoning_effort="high"','-c','service_tier="default"','-c','model_provider="openai"','-c','web_search="disabled"','--sandbox','read-only','--color','never','--json','--cd',str(tree),'--output-last-message',str(a/'full-review.md'),'-']
with (a/'full-review.prompt.txt').open('rb') as stdin,(a/'full-review.events.jsonl').open('wb') as stdout,(a/'full-review.stderr.log').open('wb') as stderr:
    process=subprocess.Popen(cmd,stdin=stdin,stdout=stdout,stderr=stderr,env=env)
    status['child_pid']=process.pid;(a/'full-review-status.json').write_text(json.dumps(status,indent=2)+'\n')
    code=process.wait()
status.update(status='completed' if code==0 else 'failed',exit_code=code,duration_seconds=time.time()-started)
(a/'full-review-status.json').write_text(json.dumps(status,indent=2)+'\n')
