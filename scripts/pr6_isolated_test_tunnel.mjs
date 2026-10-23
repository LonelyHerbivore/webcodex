/**
 * Isolated PR6 Tunnel acceptance launcher (local test only).
 *
 * Reads the *Test* Desktop Tunnel identity strictly in memory. It never logs
 * or copies either credential and refuses to run while Desktop still owns
 * that identity. Does not alter production Server, Runner or Tunnel.
 *
 * Usage:
 *   node scripts/pr6_isolated_test_tunnel.mjs status
 *   node scripts/pr6_isolated_test_tunnel.mjs run
 *
 * In run mode keep stdin open. EOF / Ctrl+C stops tunnel, runner, server.
 */
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import {spawn} from 'node:child_process';

const mode=process.argv[2]??'status';
if(!['status','run'].includes(mode))throw Error('Choose status or run');
const root=path.resolve('.pr6-host-candidate-20261022');
const trio=path.join(root,'runtime-trio');
const sandbox=path.join(root,'isolated-host');
const appData=process.env.LOCALAPPDATA;
assert.ok(typeof appData==='string'&&path.isAbsolute(appData),'Windows LOCALAPPDATA not available');
assert.ok(root.toLowerCase().startsWith('d:'+path.sep.toLowerCase()),'Must run from the D-drive isolated checkout');

const desktopConfig=path.join(appData,'dev.webcodex.desktop','secrets','tunnel-config.json');
const desktop=JSON.parse(fs.readFileSync(desktopConfig,'utf8'));
assert.ok(Array.isArray(desktop.profiles),'Desktop Tunnel profiles unavailable');
const testProfiles=desktop.profiles.filter(x=>x.name==='Test');
assert.equal(testProfiles.length,1,'Expected exactly one Desktop Test Tunnel profile');
const test=testProfiles[0];
const prod=desktop.profiles.find(x=>x.name==='ChatGPT');
assert.ok(prod,'Expected a separate production ChatGPT Tunnel');
assert.equal(typeof test.credentials?.tunnel_id,'string');
assert.equal(typeof test.credentials?.api_key,'string');
assert.notEqual(test.credentials.tunnel_id,prod.credentials?.tunnel_id,'Refuse production tunnel identity');

const activeDigest=crypto.createHash('sha256')
  .update('https://api.openai.com/'+test.credentials.tunnel_id).digest('hex');
const marker=path.join(appData,'webcodex','state','tunnel-runs',activeDigest+'.active');
const active=fs.existsSync(marker);
const owner=active?(()=>{try{return JSON.parse(fs.readFileSync(marker,'utf8')).owner_pid}catch{return null}})():null;

const expected='c2d19dc9af63e966af5509a17ebb0b568cb1731d';
const manifest=fs.readFileSync(path.join(trio,'BUILD_ID.txt'),'utf8');
assert.ok(manifest.split(/\r?\n/).includes('source_commit='+expected),'Wrong candidate build');
for(const line of fs.readFileSync(path.join(trio,'SHA256SUMS.txt'),'utf8').trim().split(/\r?\n/)){
  const match=line.match(/^([a-f0-9]{64})\s+(\S+)$/i);
  assert.ok(match,'Bad candidate SHA256 line');
  const calculated=crypto.createHash('sha256').update(fs.readFileSync(path.join(trio,match[2]))).digest('hex');
  assert.equal(calculated,match[1].toLowerCase(),'Candidate binary SHA256 mismatch');
}
const envFile=path.join(sandbox,'webcodex.env');
const runnerConfig=path.join(sandbox,'connections','http_127.0.0.1_18086','pr6test','runner.toml');
assert.ok(fs.existsSync(envFile)&&fs.existsSync(runnerConfig),'Missing isolated Host/Runner enrollment');
const localEnv=fs.readFileSync(envFile,'utf8');
assert.ok(localEnv.split(/\r?\n/).includes('WEBCODEX_ADDR=127.0.0.1:18086'));
const token=localEnv.match(/^WEBCODEX_TOKEN=([^\r\n]+)$/m)?.[1];
assert.ok(token&&token.length>=48,'Local bootstrap token missing');

console.log(JSON.stringify({
  profile:'Test',profile_enabled:test.enabled,profile_autostart:test.autostart,
  desktop_tunnel_active:active,desktop_owner_pid:owner,
  same_as_production_tunnel:false,
  candidate_commit:expected,
  candidate_loopback:'127.0.0.1:18086',
  credentials:'already available locally; kept in memory only',
  ready_to_start:!active&&!test.autostart&&!test.enabled,
  next:active?'Test owner stopped but run fence remains: diagnose uncertain last run and obtain explicit archival approval; do not delete the fence.':test.autostart||test.enabled?'Disable the Test profile and its autostart in Desktop before manual isolated use.':'Run this script with run and keep stdin open',
}));

if(mode==='status')process.exit(0);
assert.ok(!active,'Test tunnel still running/uncertain: fail closed; do not bypass its restart fence');
assert.ok(!test.autostart,'Test profile autostart still enabled; refuse racing Desktop relaunch');
assert.ok(!test.enabled,'Desktop Test identity still enabled; refuse Desktop/manual owner races');

const url='http://127.0.0.1:18086';
let alreadyUp=false;
try{const r=await fetch(url+'/healthz',{signal:AbortSignal.timeout(900)});alreadyUp=r.ok}catch{}
assert.equal(alreadyUp,false,'Port 18086 already hosts a Server; refusing to bind');

// Inherit network proxy policy only (FlClash may be required for OpenAI control plane).
const env={...process.env};
for(const k of Object.keys(env)){
  if(k.startsWith('CONTROL_PLANE_')||k.startsWith('WEBCODEX_'))delete env[k];
}
env.WEBCODEX_MCP_APP_RESUME_MODE='unknown';
env.NO_PROXY=[env.NO_PROXY, '127.0.0.1','localhost'].filter(Boolean).join(',');
env.TMP=path.join(sandbox,'temp');
env.TEMP=env.TMP;
env.TMPDIR=env.TMP;
if(!fs.existsSync(env.TMP))fs.mkdirSync(env.TMP,{recursive:true});

const launched=[];
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
function launch(program,args,customEnv=env,onOutput=null){
  const proc=spawn(path.join(trio,program+'.exe'),args,{
    cwd:sandbox,env:customEnv,windowsHide:true,stdio:['pipe','pipe','pipe']
  });
  const obj={program,proc,exit:new Promise(resolve=>proc.once('exit',(code,signal)=>resolve({code,signal})) )};
  // Never forward arbitrary child lines: they can contain secrets.
  proc.stderr.resume();
  if(onOutput)proc.stdout.on('data',onOutput);
  else proc.stdout.resume();
  launched.push(obj);
  return obj;
}
async function health(){
  for(let tries=0;tries<80;tries++){
    try{const r=await fetch(url+'/healthz',{signal:AbortSignal.timeout(900)});if(r.status===200)return;}catch{}
    await sleep(300);
  }
  throw Error('Isolated Host failed loopback health before Tunnel');
}
async function onlineRunner(){
  const headers={'authorization':'Bearer '+token,'content-type':'application/json',
    'accept':'application/json, text/event-stream'};
  const start=await fetch(url+'/mcp',{method:'POST',headers,
    body:JSON.stringify({jsonrpc:'2.0',id:1,method:'initialize',params:{}}),
    signal:AbortSignal.timeout(6000)});
  assert.equal(start.status,200,'Private MCP initialize failed');
  const sid=start.headers.get('mcp-session-id');
  for(let i=0;i<65;i++){
    const h={...headers,...(sid?{'mcp-session-id':sid}:{})};
    try{
      const response=await fetch(url+'/mcp',{method:'POST',headers:h,
        body:JSON.stringify({jsonrpc:'2.0',id:2,method:'tools/call',
          params:{name:'get_runtime_status',arguments:{compact:true}}}),
        signal:AbortSignal.timeout(3500)});
      if(response.status===200){
        const v=await response.json();
        const data=v.result?.structuredContent?.output??v.result?.structuredContent??{};
        const count=data.runners?.online_count??data.runners?.summary?.online??0;
        const projects=data.projects?.count??data.projects?.effective?.count??0;
        if(count===1&&projects===1)return;
      }
    }catch{}
    await sleep(350);
  }
  throw Error('Expected exactly one isolated Runner and one project');
}
let closing=false;
async function stopAll(){
  if(closing)return;
  closing=true;
  for(const p of launched.slice().reverse()){
    p.proc.stdin.end();
    const exit=await Promise.race([p.exit,sleep(12500).then(()=>({timeout:true}))]);
    if(exit.timeout){
      console.log(JSON.stringify({stopped:p.program,outcome:'unknown; preserved Tunnel run fence'}));
      // Do not kill an uncertain Tunnel, Runner or Host.
      break;
    }
    console.log(JSON.stringify({stopped:p.program,exit_code:exit.code}));
  }
}
process.on('SIGINT',()=>{void stopAll().then(()=>process.exit(0))});
process.on('SIGTERM',()=>{void stopAll().then(()=>process.exit(0))});
process.stdin.resume();
process.stdin.on('end',()=>{void stopAll().then(()=>process.exit(0))});

try{
  launch('webcodex-server',['--stop-on-stdin-eof','--env-file',envFile]);
  await health();
  launch('webcodex-runner',['--stop-on-stdin-eof','--config',runnerConfig]);
  await onlineRunner();
  // Only the tunnel child needs the OpenAI control-plane credentials.
  // Its profile ID suppresses the unmanaged CLI's clipboard-write handoff.
  const tunnelEnv={...env,CONTROL_PLANE_TUNNEL_ID:test.credentials.tunnel_id,
    CONTROL_PLANE_API_KEY:test.credentials.api_key,WEBCODEX_TUNNEL_PROFILE_ID:test.id};
  // Install the event listener before the child can emit a readiness line.
  let local=false,remote=false,lines='';
  const onTunnelOutput=buf=>{
    lines+=buf.toString('utf8');
    while(lines.includes('\n')){
      const end=lines.indexOf('\n');
      const line=lines.slice(0,end).trim();
      lines=lines.slice(end+1);
      if(!line.startsWith('{'))continue;
      try{
        const x=JSON.parse(line);
        if(x.local_mcp_ready===true)local=true;
        if(x.tunnel_ready===true)remote=true;
        // Report only whitelisted readiness booleans, never raw tunnel logs.
      }catch{}
    }
  };
  const tunnel=launch('webcodex',['server','tunnel','--provider','openai',
    '--env-file',envFile,'--json','--stop-on-stdin-eof'],tunnelEnv,onTunnelOutput);
  const safetyDeadline=Date.now()+60000;
  while(Date.now()<safetyDeadline&&!remote){
    if(tunnel.proc.exitCode!==null)throw Error('Test Tunnel exited before ready');
    await sleep(500);
  }
  assert.ok(remote,'Test Tunnel did not confirm a control-plane ready poll');
  console.log(JSON.stringify({phase:'ready',host:'PR6 isolated candidate',runner:'one test project',
    tunnel_profile:'Test',tunnel_ready:remote,local_mcp_ready:local,
    evidence:'Tunnel ready only; independent ChatGPT connector test still needed'}));
  // Keep the parent lifecycle owner alive until explicit EOF or Ctrl+C.
  await new Promise(resolve=>process.stdin.once('end',resolve));
}catch(error){
  console.error('Test-Tunnel startup failed safely; inspect only redacted diagnostics');
  await stopAll();
  process.exitCode=1;
}