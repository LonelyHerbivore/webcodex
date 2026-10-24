import fs from 'node:fs';
import {pathToFileURL} from 'node:url';

function envSetting(source, name) {
  const found=source.split(/\r?\n/).filter(line=>line.startsWith(name+'='));
  if(found.length!==1)return null;
  return found[0].slice(name.length+1).trim().replace(/^"(.*)"$/,'$1');
}
export function inspectManagedOAuthTestConfig(source) {
  if(typeof source!=='string'||source.length>65536)throw new Error('invalid_test_env_input');
  const publicUrl=envSetting(source,'WEBCODEX_PUBLIC_URL');
  const issuer=envSetting(source,'WEBCODEX_OAUTH2_ISSUER');
  const enabled=envSetting(source,'WEBCODEX_OAUTH2_ENABLED')==='true';
  const validOrigin=value=>{
    if(!value)return false;
    try {
      const url=new URL(value);
      return url.protocol==='https:' && !!url.hostname &&
        url.username==='' && url.password==='' &&
        url.pathname==='/' && url.search==='' && url.hash==='' &&
        !['localhost','127.0.0.1','::1','[::1]'].includes(url.hostname);
    } catch {return false}
  };
  const httpsOrigin=validOrigin(publicUrl);
  const matchingIssuer=httpsOrigin && validOrigin(issuer) &&
    new URL(publicUrl).origin===new URL(issuer).origin;
  const conditions={
    dedicated_public_https_origin:httpsOrigin,
    oauth2_enabled:enabled,
    issuer_matches_public_origin:matchingIssuer,
    user_login_consent_required:true,
    exact_chatgpt_redirect_uri_required:true,
    tunnel_no_auth_not_managed_identity:true,
  };
  const missing=[];
  if(!httpsOrigin)missing.push('public_https_test_origin');
  if(!enabled)missing.push('oauth2_server_disabled');
  if(!matchingIssuer)missing.push('issuer_not_matching_public_origin');
  return Object.freeze({
    managed_oauth_preflight_ready:missing.length===0,
    conditions:Object.freeze(conditions),
    missing:Object.freeze(missing),
    existing_tunnel_must_remain_unchanged:true,
    next_step:missing.length===0
      ? 'User must authorize a separate managed-user OAuth ChatGPT connection using its exact redirect URI and existing Test owner. Never share a PAT or bootstrap token.'
      : 'Operator must provision a separate authorized HTTPS Test origin and matching OAuth issuer, then user logs in and explicitly authorizes ChatGPT via managed-user OAuth. Do not expose the current loopback Test Host or loosen bootstrap auth.',
  });
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
  try{
    if(process.argv.length!==3)throw new Error('usage: node pr6_managed_oauth_preflight.mjs <isolated-host-webcodex.env>');
    const stat=fs.lstatSync(process.argv[2]);
    if(!stat.isFile()||stat.isSymbolicLink()||stat.size>65536)
      throw new Error('invalid_test_env_file');
    const result=inspectManagedOAuthTestConfig(fs.readFileSync(process.argv[2],'utf8'));
    process.stdout.write(JSON.stringify(result)+'\n');
  } catch(error){
    process.stderr.write('Preflight failed: '+error.message+'\n');
    process.exitCode=1;
  }
}
