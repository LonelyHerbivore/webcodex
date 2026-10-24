import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {verifyWindowsHostArtifact} from './verify_orca_host_windows_artifact.mjs';

const sha='a'.repeat(40);
async function fixture(){
 const root=await mkdtemp(join(tmpdir(),'wc-host-pe-'));
 const exe=join(root,'webcodex-server.exe');
 const manifest=join(root,'manifest.json');
 const b=Buffer.alloc(8192);
 b.write('MZ',0,'ascii');
 b.writeUInt32LE(128,0x3c);
 b.write('PE\0\0',128,'ascii');
 b.writeUInt16LE(0x8664,132);
 await writeFile(exe,b);
 const data={source_repository:'LonelyHerbivore/webcodex',
  source_commit:sha,workflow_run_id:'123456',workflow_run_attempt:'1',
  target:'x86_64-pc-windows-msvc',binary:'webcodex-server.exe',
  binary_sha256:createHash('sha256').update(b).digest('hex'),
  purpose:'isolated_diagnostic_only_not_signed_or_production_ready'};
 await writeFile(manifest,JSON.stringify(data));
 return {root,exe,manifest,data};
}
test('accepts exact matched Windows x64 PE, Git SHA and digest',async()=>{
 const f=await fixture();
 try{
  const v=await verifyWindowsHostArtifact(f.root,sha);
  assert.equal(v.sourceCommit,sha);
  assert.equal(v.sha256,f.data.binary_sha256);
  assert.equal(v.size,8192);
 }finally{await rm(f.root,{recursive:true,force:true})}
});
test('rejects byte substitution with otherwise valid manifest',async()=>{
 const f=await fixture();
 try{
  const b=await readFile(f.exe);
  b[300]=1;await writeFile(f.exe,b);
  await assert.rejects(verifyWindowsHostArtifact(f.root,sha),/digest_mismatch/);
 }finally{await rm(f.root,{recursive:true,force:true})}
});
test('rejects wrong source commit and wrong target machine',async()=>{
 const f=await fixture();
 try{
  await assert.rejects(verifyWindowsHostArtifact(f.root,'b'.repeat(40)),/identity_invalid/);
  const b=await readFile(f.exe);
  b.writeUInt16LE(0xaa64,132);
  await writeFile(f.exe,b);
  await assert.rejects(verifyWindowsHostArtifact(f.root,sha),/not_windows_x64/);
 }finally{await rm(f.root,{recursive:true,force:true})}
});

async function trioFixture(){
 const f=await fixture();
 const binaries={};
 for(const name of ['webcodex-server.exe','webcodex.exe','webcodex-runner.exe']){
  const path=join(f.root,name);
  if(name!=='webcodex-server.exe'){
   const b=await readFile(f.exe);
   b[250]=name==='webcodex.exe'?37:83;
   await writeFile(path,b);
  }
  const b=await readFile(path);
  binaries[name]=createHash('sha256').update(b).digest('hex');
 }
 delete f.data.binary;
 delete f.data.binary_sha256;
 f.data.binaries=binaries;
 await writeFile(f.manifest,JSON.stringify(f.data));
 return f;
}
test('requires the exact Host/CLI/Runner trio and verifies all three PE digests',async()=>{
 const f=await trioFixture();
 try{
  const result=await verifyWindowsHostArtifact(f.root,sha,{requireTrio:true});
  assert.deepEqual(Object.keys(result.binaries).sort(),
   ['webcodex-server.exe','webcodex.exe','webcodex-runner.exe'].sort());
  assert.equal(result.binaries['webcodex-runner.exe'].size,8192);
 }finally{await rm(f.root,{recursive:true,force:true})}
});
test('rejects a single substituted Runner while Host binary remains valid',async()=>{
 const f=await trioFixture();
 try{
  const runner=join(f.root,'webcodex-runner.exe');
  const b=await readFile(runner);
  b[310]=14;await writeFile(runner,b);
  await assert.rejects(verifyWindowsHostArtifact(f.root,sha,{requireTrio:true}),/digest_mismatch/);
 }finally{await rm(f.root,{recursive:true,force:true})}
});
test('does not accept legacy server-only manifest at the trio gate',async()=>{
 const f=await fixture();
 try{
  await assert.rejects(verifyWindowsHostArtifact(f.root,sha,{requireTrio:true}),/trio_required/);
 }finally{await rm(f.root,{recursive:true,force:true})}
});
test('rejects manifest with unknown or missing executable entries',async()=>{
 const f=await trioFixture();
 try{
  f.data.binaries['not-trusted.exe']=f.data.binaries['webcodex.exe'];
  await writeFile(f.manifest,JSON.stringify(f.data));
  await assert.rejects(verifyWindowsHostArtifact(f.root,sha,{requireTrio:true}),/trio_manifest_invalid/);
 }finally{await rm(f.root,{recursive:true,force:true})}
});
