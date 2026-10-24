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
if(check('run-current')){
  data={run:{id:mode==='bad_coordinator'?'run_other':'run_test1',coordinator_handle:'term_coordinator'}};
}else if(check('worker-show')){
  data={dispatch:{id:'ctx_worker',runId:'run_test1',taskId:'task_worker',
    assigneeHandle:'term_worker',status:'active'},worker:{state:'running'}};
}else if(check('check')){
  const valid={id:'msg_question',run_id:'run_test1',to_handle:'run:run_test1',
    from_handle:'dispatch:ctx_worker',type:'question',subject:'Need input',body:'Question?',
    payload:'{"dispatchId":"ctx_worker","taskId":"task_worker"}'};
  if(mode==='bad_message')valid.id='not_valid';
  data={runId:'run_test1',messages:[valid]};
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
