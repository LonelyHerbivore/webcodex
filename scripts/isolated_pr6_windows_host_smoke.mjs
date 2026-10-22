// Diagnostic only: exact PR #6 binary on an isolated loopback-only Server.
// No shared credentials, no real Runner enrollment, no persistent Shell.
import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';

const root=path.resolve('.pr6-host-candidate-20261022');
const binary=path.join(root,'webcodex-server.exe');
const expectedCommit='c2d19dc9af63e966af5509a17ebb0b568cb1731d';
assert.ok(fs.existsSync(binary),'Candidate must be downloaded into the D: sandbox');
const identity=fs.readFileSync(path.join(root,'BUILD_ID.txt'),'utf8');
const line=identity.split(/\r?\n/).find(s=>s.startsWith('source_commit='));
assert.equal(line,'source_commit='+expectedCommit,'Exact PR6 candidate source');
const expectedHash=fs.readFileSync(path.join(root,'SHA256SUMS.txt'),'utf8').split(/\s+/)[0].toLowerCase();
const actualHash=crypto.createHash('sha256').update(fs.readFileSync(binary)).digest('hex');
assert.equal(actualHash,expectedHash,'GitHub artifact hash');
const version=execFileSync(binary,['--version'],{encoding:'utf8',timeout:15000,windowsHide:true}).trim();
assert.ok(version.includes('commit '+expectedCommit+', dirty=false'),'Unexpected candidate identity: '+version);
const help=execFileSync(binary,['--help'],{encoding:'utf8',timeout:15000,windowsHide:true});
assert.ok(help.includes('--stop-on-stdin-eof'),'Expected bounded Server lifecycle');

const allocatePort=()=>new Promise((resolve,reject)=>{
  const srv=net.createServer();
  srv.once('error',reject);
  srv.listen(0,'127.0.0.1',()=>{
    const port=srv.address().port;
    srv.close(error=>error?reject(error):resolve(port));
  });
});
const port=await allocatePort();
const dataDir=path.join(root,'runtime-data-mcp');
assert.ok(!fs.existsSync(dataDir),'Refuse to reuse existing data directory');
fs.mkdirSync(dataDir,{recursive:true});
const env={...process.env};
for(const key of Object.keys(env)){
  if(key.startsWith('WEBCODEX_')||/^(OPENAI_|GITHUB_|GH_|ANTHROPIC_|CODEX_)/.test(key))delete env[key];
}
const scratch=path.join(root,'temp-mcp');
fs.mkdirSync(scratch,{recursive:true});
Object.assign(env,{
  WEBCODEX_ADDR:'127.0.0.1:'+port,
  WEBCODEX_DATA:dataDir,
  WEBCODEX_TOKEN:crypto.randomBytes(32).toString('hex'),
  WEBCODEX_MCP_APP_RESUME_MODE:'unknown',
  WEBCODEX_ALLOW_ANONYMOUS:'false',
  WEBCODEX_SHARED_KEY_ENABLED:'false',
  WEBCODEX_PUBLIC_URL:'',
  TEMP:scratch,TMP:scratch,TMPDIR:scratch,
});
const child=spawn(binary,['--stop-on-stdin-eof'],{
  cwd:root,env,windowsHide:true,stdio:['pipe','pipe','pipe']
});
let stderr='',stdout='';
child.stdout.on('data',x=>{stdout=(stdout+x.toString()).slice(-6000)});
child.stderr.on('data',x=>{stderr=(stderr+x.toString()).slice(-6000)});
const exited=new Promise(resolve=>child.once('exit',(code,signal)=>resolve({code,signal})));
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
let healthStatus=null,unauthorizedStatus=null,authorizedMcpStatus=null,foundPresentationTools=[],stop=null;
try{
  const deadline=Date.now()+30000;
  while(Date.now()<deadline){
    if(child.exitCode!==null)throw new Error('Candidate exited before healthz');
    try{
      const res=await fetch('http://127.0.0.1:'+port+'/healthz',{signal:AbortSignal.timeout(1000)});
      if(res.ok){healthStatus=res.status;break;}
    }catch{}
    await sleep(300);
  }
  assert.equal(healthStatus,200,'Loopback /healthz not ready');
  const response=await fetch('http://127.0.0.1:'+port+'/mcp',{
    method:'POST',
    headers:{'content-type':'application/json'},
    body:JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/list',params:{}}),
    signal:AbortSignal.timeout(5000),
  });
  unauthorizedStatus=response.status;
  assert.ok([401,403].includes(unauthorizedStatus),'Unexpected anonymous MCP status '+unauthorizedStatus);
  // The test token is in memory and never logged or persisted.
  const headers={
    'authorization':'Bearer '+env.WEBCODEX_TOKEN,
    'content-type':'application/json',
    'accept':'application/json, text/event-stream',
    'mcp-protocol-version':'2025-11-25',
  };
  const initial=await fetch('http://127.0.0.1:'+port+'/mcp',{
    method:'POST',headers,
    body:JSON.stringify({jsonrpc:'2.0',id:2,method:'initialize',params:{}}),
    signal:AbortSignal.timeout(5000),
  });
  assert.equal(initial.status,200,'Authorized MCP initialize failed');
  const initialized=await initial.json();
  assert.equal(initialized.result?.serverInfo?.name,'webcodex','Wrong MCP Server identity');
  const session=initial.headers.get('mcp-session-id');
  const listed=await fetch('http://127.0.0.1:'+port+'/mcp',{
    method:'POST',
    headers:{...headers,...(session?{'mcp-session-id':session}:{})},
    body:JSON.stringify({jsonrpc:'2.0',id:3,method:'tools/list',params:{
      _meta:{'io.modelcontextprotocol/protocolVersion':'2025-11-25'}
    }}),
    signal:AbortSignal.timeout(5000),
  });
  authorizedMcpStatus=listed.status;
  assert.equal(authorizedMcpStatus,200,'Authorized MCP tools/list failed');
  const listBody=await listed.json();
  assert.ok(Array.isArray(listBody.result?.tools),'MCP tool descriptors unavailable');
  const wanted=['present_agent_continuation','present_job_terminal_continuation'];
  foundPresentationTools=wanted.filter(name=>listBody.result.tools.some(tool=>tool.name===name));
  assert.deepEqual(foundPresentationTools,wanted,'Required continuation presentation descriptors absent');
}finally{
  child.stdin.end();
  stop=await Promise.race([exited,sleep(10000).then(()=>({timeout:true}))]);
  if(stop?.timeout){child.kill();stop=await exited;}
}
const summary={
  source_commit:expectedCommit,
  binary_sha256:actualHash,
  version,
  listen_scope:'127.0.0.1',
  health_status:healthStatus,
  anonymous_mcp_status:unauthorizedStatus,
  authorized_mcp_status:authorizedMcpStatus,
  presentation_tools:foundPresentationTools,
  server_exit:stop,
  host_turn_id_attested:false,
  pr6_six_case_acceptance:'not_run',
  tested_at:new Date().toISOString(),
};
fs.writeFileSync(path.join(root,'smoke-evidence-mcp.json'),JSON.stringify(summary,null,2));
console.log(JSON.stringify(summary,null,2));
assert.equal(stop?.code,0,'Server must stop cleanly on stdin EOF');
