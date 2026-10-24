import {createHash} from 'node:crypto';
import {createReadStream} from 'node:fs';
import {open, lstat, readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';

const sha=/^[0-9a-f]{40}$/;
const sha256=/^[0-9a-f]{64}$/;
const expectedTrio=['webcodex-server.exe','webcodex.exe','webcodex-runner.exe'];
const printable=n=>Number.isSafeInteger(n)&&n>0;

async function verifiedBinary(path,expectedDigest){
  const st=await lstat(path);
  if(!st.isFile()||st.isSymbolicLink()||st.size<4096||st.size>1024*1024*1024)
    throw new Error('diagnostic_binary_file_invalid');
  const fd=await open(path,'r');
  try{
    const header=Buffer.alloc(256);
    const {bytesRead}=await fd.read(header,0,header.length,0);
    if(bytesRead<128||header.toString('ascii',0,2)!=='MZ')throw new Error('not_windows_pe_image');
    const offset=header.readUInt32LE(0x3c);
    if(offset<64||offset>st.size-6)throw new Error('invalid_pe_header_offset');
    const pe=Buffer.alloc(6);
    const result=await fd.read(pe,0,6,offset);
    if(result.bytesRead!==6||pe.toString('ascii',0,4)!=='PE\0\0'||pe.readUInt16LE(4)!==0x8664)
      throw new Error('not_windows_x64_pe_image');
  }finally{await fd.close()}
  const h=createHash('sha256');
  for await(const chunk of createReadStream(path))h.update(chunk);
  if(h.digest('hex')!==expectedDigest)throw new Error('diagnostic_binary_digest_mismatch');
  return Object.freeze({size:st.size,sha256:expectedDigest});
}

export async function verifyWindowsHostArtifact(directory,expectedCommit,{requireTrio=false}={}){
  if(!sha.test(expectedCommit??''))throw new Error('expected_source_commit_invalid');
  const manifestPath=join(directory,'manifest.json');
  const st=await lstat(manifestPath);
  if(!st.isFile()||st.isSymbolicLink()||st.size<80||st.size>8192)
    throw new Error('diagnostic_manifest_file_invalid');
  const obj=JSON.parse(await readFile(manifestPath,'utf8'));
  if(obj?.source_repository!=='LonelyHerbivore/webcodex'||
     obj?.source_commit!==expectedCommit||
     obj?.target!=='x86_64-pc-windows-msvc'||
     obj?.purpose!=='isolated_diagnostic_only_not_signed_or_production_ready'||
     !printable(Number(obj?.workflow_run_id))||
     !printable(Number(obj?.workflow_run_attempt)))
    throw new Error('diagnostic_manifest_identity_invalid');
  let digests;
  if(obj?.binaries!==undefined){
    if(!obj.binaries||typeof obj.binaries!=='object'||Array.isArray(obj.binaries)||
       Object.keys(obj.binaries).length!==3||
       !expectedTrio.every(name=>Object.hasOwn(obj.binaries,name)&&sha256.test(obj.binaries[name])))
      throw new Error('diagnostic_trio_manifest_invalid');
    digests=obj.binaries;
  }else{
    if(obj?.binary!=='webcodex-server.exe'||!sha256.test(obj?.binary_sha256??''))
      throw new Error('diagnostic_manifest_identity_invalid');
    digests={'webcodex-server.exe':obj.binary_sha256};
  }
  if(requireTrio&&Object.keys(digests).length!==3)
    throw new Error('diagnostic_trio_required');
  const verified={};
  for(const name of Object.keys(digests))
    verified[name]=await verifiedBinary(join(directory,name),digests[name]);
  return Object.freeze({sourceCommit:obj.source_commit,
    sha256:verified['webcodex-server.exe'].sha256,
    workflowRunId:String(obj.workflow_run_id),
    size:verified['webcodex-server.exe'].size,target:obj.target,
    binaries:Object.freeze(verified)});
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
  try{
    const result=await verifyWindowsHostArtifact(process.argv[2],process.argv[3],
      {requireTrio:process.argv[4]==='--require-trio'});
    process.stdout.write(JSON.stringify(result)+'\n');
  }catch(error){
    process.stderr.write('Artifact rejected: '+error.message+'\n');
    process.exitCode=1;
  }
}
