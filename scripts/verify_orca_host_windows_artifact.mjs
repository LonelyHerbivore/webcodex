import {createHash} from 'node:crypto';
import {createReadStream} from 'node:fs';
import {open, lstat, readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';

const sha=/^[0-9a-f]{40}$/;
const sha256=/^[0-9a-f]{64}$/;
const printable=n=>Number.isSafeInteger(n)&&n>0;
export async function verifyWindowsHostArtifact(directory,expectedCommit){
  if(!sha.test(expectedCommit??''))throw new Error('expected_source_commit_invalid');
  const path=join(directory,'webcodex-server.exe');
  const manifestPath=join(directory,'manifest.json');
  const [exeStats,manifestStats]=await Promise.all([lstat(path),lstat(manifestPath)]);
  if(!exeStats.isFile()||exeStats.isSymbolicLink()||exeStats.size<4096||exeStats.size>1024*1024*1024)
    throw new Error('diagnostic_binary_file_invalid');
  if(!manifestStats.isFile()||manifestStats.isSymbolicLink()||manifestStats.size<80||manifestStats.size>8192)
    throw new Error('diagnostic_manifest_file_invalid');
  const obj=JSON.parse(await readFile(manifestPath,'utf8'));
  if(obj?.source_repository!=='LonelyHerbivore/webcodex'||
    obj?.source_commit!==expectedCommit||
    obj?.target!=='x86_64-pc-windows-msvc'||
    obj?.binary!=='webcodex-server.exe'||
    obj?.purpose!=='isolated_diagnostic_only_not_signed_or_production_ready'||
    !sha256.test(obj?.binary_sha256??'')||
    !printable(Number(obj?.workflow_run_id))||
    !printable(Number(obj?.workflow_run_attempt)))
    throw new Error('diagnostic_manifest_identity_invalid');
  const fd=await open(path,'r');
  try{
    const header=Buffer.alloc(256);
    const {bytesRead}=await fd.read(header,0,header.length,0);
    if(bytesRead<128||header.toString('ascii',0,2)!=='MZ')throw new Error('not_windows_pe_image');
    const offset=header.readUInt32LE(0x3c);
    if(offset<64||offset>exeStats.size-6)throw new Error('invalid_pe_header_offset');
    const pe=Buffer.alloc(6);
    const p=await fd.read(pe,0,6,offset);
    if(p.bytesRead!==6||pe.toString('ascii',0,4)!=='PE\0\0'||pe.readUInt16LE(4)!==0x8664)
      throw new Error('not_windows_x64_pe_image');
  }finally{await fd.close()}
  const h=createHash('sha256');
  for await (const chunk of createReadStream(path))h.update(chunk);
  if(h.digest('hex')!==obj.binary_sha256)throw new Error('diagnostic_binary_digest_mismatch');
  return Object.freeze({sourceCommit:obj.source_commit,sha256:obj.binary_sha256,
    workflowRunId:String(obj.workflow_run_id),size:exeStats.size,target:obj.target});
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
  try{
    const r=await verifyWindowsHostArtifact(process.argv[2],process.argv[3]);
    process.stdout.write(JSON.stringify(r)+'\n');
  }catch(e){process.stderr.write('Artifact rejected: '+e.message+'\n');process.exitCode=1}
}
