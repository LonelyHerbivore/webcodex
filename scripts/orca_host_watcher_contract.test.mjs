import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const source=readFileSync(fileURLToPath(new URL('../src/tool_runtime/orca_host_watcher.mjs',import.meta.url)),'utf8');
const run='run_test1',coordinator='term_coordinator',dispatch='ctx_worker',task='task_worker';
const watch='orca_host_wd_'+'a'.repeat(32);
const fixture=String.raw`
const args=process.argv.slice(2);
const check=(key)=>args.includes(key);
const mode=process.env.TEST_ORCA_WATCH_MODE;
let data;
if(check('run-show')){
  data={run:{id:mode==='bad_coordinator'?'run_other':'run_test1',coordinator_handle:'term_coordinator'}};
}else if(check('worker-show')){
  const finished=mode==='done';
  data={dispatch:{id:'ctx_worker',runId:'run_test1',taskId:'task_worker',
    assigneeHandle:'term_worker',status:finished?'completed':'active'},
    worker:{state:finished?'succeeded':'running'}};
}else if(check('inbox')){
  const valid={id:mode==='done'?'msg_done':'msg_question',
    run_id:'run_test1',to_handle:'run:run_test1',
    from_handle:'dispatch:ctx_worker',
    type:mode==='done'?'worker_done':'question',
    read:0,subject:'Need input',body:'Question?',
    payload:mode==='done'?
      '{"dispatchId":"ctx_worker","taskId":"task_worker","outcome":"succeeded"}':
      '{"dispatchId":"ctx_worker","taskId":"task_worker"}'};
  if(mode==='bad_message')valid.id='not_valid';
  if(mode==='foreign_recipient')valid.to_handle='run:run_other';
  let messages=[valid];
  if(mode==='read_and_new'){
    messages=[{...valid,read:1}, {...valid,id:'msg_new_question',read:0}];
  }
  if(mode==='saturated'){
    messages=Array.from({length:100},(_,i)=>({...valid,id:'msg_inbox_'+i}));
  }
  data={messages,count:messages.length};
}else{
  process.exit(1);
}
console.log(JSON.stringify({ok:true,result:data}));
`;
function runFixture(mode){
 const dir=mkdtempSync(join(tmpdir(),'host-orca-watch-test-'));
 try{
  const bin=join(dir,'orca-fixture.mjs');writeFileSync(bin,fixture);
  return spawnSync(process.execPath,
   ['--input-type=module','--eval',source,'--',run,coordinator,dispatch,task,watch],{
     env:{...process.env,ORCA_BIN:bin,TEST_ORCA_WATCH_MODE:mode},
     encoding:'utf8',timeout:12000});
 }finally{rmSync(dir,{recursive:true,force:true})}
}
test('trusted embedded watcher reports exact verified coordinator question once',()=>{
 const r=runFixture('good');
 assert.equal(r.status,0,r.stderr);
 const events=r.stdout.trim().split('\n').filter(Boolean);
 assert.equal(events.length,1);
 const payload=JSON.parse(events[0]);
 assert.equal(payload.kind,'host_orca_inbox_runner_job_v1');
 assert.equal(payload.status,'inbox_attention');
 assert.equal(payload.messages[0].messageId,'msg_question');
 assert.equal(payload.hostWakeVerified,false);
});
test('watcher rejects mismatched coordinator identity instead of reporting a wake',()=>{
 const r=runFixture('bad_coordinator');
 assert.equal(r.status,72,r.stderr);
 assert.doesNotMatch(r.stdout,/inbox_attention/);
});
test('watcher requires only fixed positional identifiers',()=>{
 const r=spawnSync(process.execPath,['--input-type=module','--eval',source,'--',
 'run_test1','term_ok','ctx_ok','task_ok','bad_watch'],{encoding:'utf8',timeout:5000});
 assert.equal(r.status,64,r.stderr);
 assert.equal(r.stdout,'');
});

test('real Orca read-only run-show and run: recipient inbox avoid stable-pane requirement',()=>{
 const r=runFixture('good');
 assert.equal(r.status,0,r.stderr);
 assert.match(r.stdout,/"status":"inbox_attention"/);
});
test('read or ACKed questions are filtered out without losing new unread messages',()=>{
 const r=runFixture('read_and_new');
 assert.equal(r.status,0,r.stderr);
 const payload=JSON.parse(r.stdout.trim());
 assert.deepEqual(payload.messages.map(m=>m.messageId),['msg_new_question']);
});
test('finished Worker requires matching unread worker_done outcome',()=>{
 const r=runFixture('done');
 assert.equal(r.status,0,r.stderr);
 const payload=JSON.parse(r.stdout.trim());
 assert.equal(payload.messages[0].type,'worker_done');
 assert.equal(payload.messages[0].outcome,'succeeded');
});
test('an inbox recipient mismatch is rejected as identity failure',()=>{
 const r=runFixture('foreign_recipient');
 assert.equal(r.status,72,r.stderr);
 assert.equal(r.stdout,'');
});
test('saturated Orca inbox fails closed rather than silently dropping unread messages',()=>{
 const r=runFixture('saturated');
 assert.equal(r.status,72,r.stderr);
 assert.equal(r.stdout,'');
});
