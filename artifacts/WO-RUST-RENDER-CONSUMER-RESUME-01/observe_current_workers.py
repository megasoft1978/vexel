from pathlib import Path
import json,subprocess
from datetime import datetime,timezone
r=Path(__file__).resolve().parents[2];a=r/'artifacts/WO-RUST-RENDER-CONSUMER-RESUME-01';p=a/'current-goal-checkpoint.json';d=json.loads(p.read_text());now=datetime.now(timezone.utc).isoformat();rows=[]
for old in d['worker_status']:
 sf=r/old['status_file'];s=json.loads(sf.read_text());pid=s.get('child_pid');obs=subprocess.run(['ps','-p',str(pid),'-o','pid=,etime=,comm='],capture_output=True,text=True).stdout.strip() if pid else ''
 s.update(wo_id=old['wo_id'],status_file=old['status_file'],process_observation=obs or 'not live',confirmed_live_at_utc=now if obs else None,source_candidate_accepted=False)
 prefix='worker' if sf.name=='process-status.json' else sf.name.removesuffix('-status.json')
 eventfile=sf.parent/(prefix+'.events.jsonl');last_message=None;error=None;commands=0
 if eventfile.exists():
  for line in eventfile.read_text().splitlines():
   try:e=json.loads(line)
   except json.JSONDecodeError:continue
   i=e.get('item',{});
   if i.get('type')=='agent_message':last_message=i.get('text','')
   if i.get('type')=='command_execution' and e.get('type')=='item.completed':commands+=1
   if e.get('type') in ('error','turn.failed'):error=e
   if e.get('type')=='turn.completed':s['reported_turn_usage']=e.get('usage')
 s['completed_command_count']=commands;s['last_agent_message']=last_message;s['last_error']=error;rows.append(s)
 print(json.dumps({'wo_id':s['wo_id'],'status':s['status'],'live':bool(obs),'commands':commands,'last_message':last_message[-650:] if last_message else None,'error':error}))
d.update(worker_status=rows,total_confirmed_source_writers=sum(x['process_observation']!='not live' for x in rows),recorded_utc=now);p.write_text(json.dumps(d,indent=2)+'\n')
