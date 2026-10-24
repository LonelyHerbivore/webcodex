// First-party Host-owned one-Worker watcher. Embedded via Rust include_str!
// in detached Runner Job argv; never execute script paths supplied by Orca.
import {spawn} from 'node:child_process';
const [runId,coordinator,dispatchId,taskId,watchKey]=process.argv.slice(1);
const token=(x,p)=>typeof x==='string'&&x.startsWith(p)&&x.length>p.length&&x.length<=128&&/^[A-Za-z0-9_-]+$/.test(x.slice(p.length));
if(!token(runId,'run_')||!token(coordinator,'term_')||!token(dispatchId,'ctx_')||
   !token(taskId,'task_')||!/^orca_host_wd_[0-9a-f]{32}$/.test(watchKey??''))process.exit(64);
const wait=ms=>new Promise(r=>setTimeout(r,ms));
async function read(args){
  let child;
  const bin=process.env.ORCA_BIN||(process.platform==='linux'?'orca-ide':'orca');
  const viaNode=/\.(?:mjs|cjs|js)$/i.test(bin);
  const exec=viaNode?process.execPath:bin;
  const argv=viaNode?[bin,...args,'--json']:[...args,'--json'];
  return await new Promise((resolve,reject)=>{
    let out='',err='',done=false;
    const fail=e=>{if(!done){done=true;reject(e);}};
    child=spawn(exec,argv,{shell:false,windowsHide:true,stdio:['ignore','pipe','pipe']});
    const timeout=setTimeout(()=>child.kill(),15000);
    child.stdout.on('data',chunk=>{out+=chunk.toString();if(out.length>524288)child.kill()});
    child.stderr.on('data',chunk=>{err+=chunk.toString();if(err.length>16384)child.kill()});
    child.on('error',fail);
    child.on('close',code=>{
      clearTimeout(timeout);
      if(done)return;
      if(code!==0||out.length>524288)return fail(new Error('orca_cli_failed'));
      let result;
      try{result=JSON.parse(out)}catch{return fail(new Error('orca_invalid_json'))}
      if(result?.ok!==true||!result.result)return fail(new Error('orca_unavailable'));
      done=true;resolve(result.result);
    });
  });
}
function verifyWorker(row){
  const dispatch=row?.dispatch;
  if(dispatch?.id!==dispatchId||dispatch.runId!==runId||dispatch.taskId!==taskId)
    throw new Error('worker_identity_mismatch');
  const sender=dispatch.assigneeHandle??row?.worker?.agentTerminalHandle??row?.projection?.agentTerminalHandle;
  if(typeof sender!=='string'||!/^[A-Za-z0-9_-]{1,128}$/.test(sender))
    throw new Error('worker_sender_unverifiable');
  return {sender,workerState:row?.worker?.state??null,dispatchStatus:dispatch.status??null};
}
function relevantMail(result,worker){
  // Orca inbox is a cross-pane read-only API. A Coordinator-pane --check
  // requires a stable Orca terminal and fails in a detached Runner Job.
  // The exact recipient filter is run:<runId>; never trust a global inbox.
  if(!Array.isArray(result?.messages)||!Number.isSafeInteger(result?.count)||
     result.count!==result.messages.length)
    throw new Error('inbox_identity_mismatch');
  // Orca inbox exposes only a bounded tail; at the cap we cannot prove that
  // earlier unread messages were not omitted. Stop instead of silent loss.
  if(result.count>=100)throw new Error('inbox_overflow_requires_reconciliation');
  const seen=new Map(),hits=[];
  for(const m of result.messages){
    if(m?.run_id!==runId||m?.to_handle!=='run:'+runId)
      throw new Error('inbox_identity_mismatch');
    if(![0,1,true,false].includes(m.read))
      throw new Error('inbox_read_state_unknown');
    if(m.read===1||m.read===true)continue;
    if(!['question','escalation','worker_done'].includes(m.type))continue;
    if(!token(m.id,'msg_'))continue;
    const identity=JSON.stringify([m.type,m.from_handle,m.payload,m.subject,m.body,m.created_at]);
    if(seen.has(m.id)){
      if(seen.get(m.id)!==identity)throw new Error('inbox_conflicting_message');
      continue;
    }
    seen.set(m.id,identity);
    let payload={};
    if(m.payload){try{payload=JSON.parse(m.payload)}catch{continue}}
    if(!payload||typeof payload!=='object'||Array.isArray(payload))continue;
    if(m.from_handle!=='dispatch:'+dispatchId&&m.from_handle!==worker.sender)continue;
    if(payload.dispatchId!==undefined&&payload.dispatchId!==dispatchId)continue;
    if(payload.taskId!==undefined&&payload.taskId!==taskId)continue;
    if(m.type==='worker_done'&&(
      payload.dispatchId!==dispatchId||payload.taskId!==taskId||
      !['succeeded','failed'].includes(payload.outcome)||
      payload.outcome!==worker.workerState||worker.dispatchStatus!=='completed'))continue;
    hits.push({messageId:m.id,type:m.type,dispatchId,taskId,
      outcome:m.type==='worker_done'?payload.outcome:null,
      fromHandle:m.from_handle,
      subject:String(m.subject??'').slice(0,256),
      bodyPreview:String(m.body??'').slice(0,1600)});
  }
  return hits;
}
async function inspect(){
  const bound=await read(['orchestration','run-show','--id',runId]);
  if(bound?.run?.id!==runId||bound?.run?.coordinator_handle!==coordinator)throw new Error('coordinator_mismatch');
  const row=await read(['orchestration','worker-show','--dispatch',dispatchId]);
  const worker=verifyWorker(row);
  const inbox=await read(['orchestration','inbox','--terminal','run:'+runId,'--limit','100']);
  const messages=relevantMail(inbox,worker);
  if(messages.length)return {status:'inbox_attention',messages};
  if(['failed','stopped','abandoned'].includes(row?.dispatch?.status)||
     ['failed','stopped','abandoned'].includes(worker.workerState))
    return {status:'worker_terminal_failure',messages:[]};
  // Never treat apparent success as worker_done without verified inbox message.
  return null;
}
let errorCount=0;
const deadline=Date.now()+900000;
while(Date.now()<deadline){
  try{
    const value=await inspect();
    errorCount=0;
    if(value){
      process.stdout.write(JSON.stringify({kind:'host_orca_inbox_runner_job_v1',
        runId,coordinatorHandle:coordinator,dispatchId,taskId,watchKey,
        ...value,hostWakeVerified:false})+'\n');
      process.exit(0);
    }
  }catch(err){
    if(['worker_identity_mismatch','worker_sender_unverifiable','inbox_identity_mismatch',
      'inbox_conflicting_message','inbox_read_state_unknown',
      'inbox_overflow_requires_reconciliation','coordinator_mismatch'].includes(err?.message)){
      process.stderr.write('host_orca_watcher_identity_failure\n');
      process.exit(72);
    }
    if(++errorCount>=3){
      process.stderr.write('host_orca_watcher_observation_failed\n');
      process.exit(74);
    }
  }
  await wait(3000);
}
process.stderr.write('host_orca_watcher_timeout\n');
process.exit(75);
