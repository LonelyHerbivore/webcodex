import test from 'node:test';
import assert from 'node:assert/strict';
import {inspectManagedOAuthTestConfig as preflight} from './pr6_managed_oauth_preflight.mjs';

test('Tunnel/No authentication defaults cannot imply managed-user OAuth consent',()=>{
  const out=preflight('WEBCODEX_ADDR=127.0.0.1:18086\nWEBCODEX_TOKEN=VERY_SECRET\n');
  assert.equal(out.managed_oauth_preflight_ready,false);
  assert.equal(out.conditions.tunnel_no_auth_not_managed_identity,true);
  assert.deepEqual(out.missing,[
    'public_https_test_origin','oauth2_server_disabled','issuer_not_matching_public_origin'
  ]);
  assert.doesNotMatch(JSON.stringify(out),/VERY_SECRET/);
});
test('eligible separate public HTTPS issuer still requires browser OAuth user approval',()=>{
  const out=preflight('WEBCODEX_PUBLIC_URL=https://test.example.org\nWEBCODEX_OAUTH2_ISSUER=https://test.example.org\nWEBCODEX_OAUTH2_ENABLED=true');
  assert.equal(out.managed_oauth_preflight_ready,true);
  assert.equal(out.conditions.user_login_consent_required,true);
  assert.equal(out.conditions.exact_chatgpt_redirect_uri_required,true);
  assert.equal(out.existing_tunnel_must_remain_unchanged,true);
});
test('rejects local origins, wrong issuer, user-info and ambiguous duplicate env keys',()=>{
  for(const env of [
    'WEBCODEX_PUBLIC_URL=http://localhost:18086\nWEBCODEX_OAUTH2_ENABLED=true\nWEBCODEX_OAUTH2_ISSUER=http://localhost:18086',
    'WEBCODEX_PUBLIC_URL=https://example.org\nWEBCODEX_OAUTH2_ENABLED=true\nWEBCODEX_OAUTH2_ISSUER=https://another.example.org',
    'WEBCODEX_PUBLIC_URL=https://name:password@example.org\nWEBCODEX_OAUTH2_ENABLED=true\nWEBCODEX_OAUTH2_ISSUER=https://example.org',
    'WEBCODEX_PUBLIC_URL=https://example.org\nWEBCODEX_PUBLIC_URL=https://another.example.org\nWEBCODEX_OAUTH2_ENABLED=true\nWEBCODEX_OAUTH2_ISSUER=https://example.org'
  ])assert.equal(preflight(env).managed_oauth_preflight_ready,false);
});
test('preflight is read-only and rejects oversized env snapshots',()=>{
  assert.throws(()=>preflight('a'.repeat(65537)),/invalid_test_env_input/);
});
