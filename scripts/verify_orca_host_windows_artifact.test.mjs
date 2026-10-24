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
