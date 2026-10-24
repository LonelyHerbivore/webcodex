# Orca P0: Test Host managed OAuth authorization handoff

**Scope:** isolated candidate only, no changes to production Host/Runner/F: Plugin
or previously connected Tunnel. Source of truth for the last built candidate is
the verified Windows Host/CLI/Runner GitHub diagnostic artifact. This procedure
does not authorize release or assume GPT continuation is working.

## Confirmed current blocker

The existing ChatGPT WebCodex 测试 connection uses an OpenAI Tunnel with
No authentication. That tunnel injects its existing technical bearer for
server MCP requests; its identity is bootstrap, not a named managed-user
credential. The Host composite deliberately rejects such calls before starting
any Worker/Job/Wait. Changing the Runner owner, increasing tool scopes,
refreshing the ChatGPT tool manifest, or setting authority trusted_agent does
not change this identity.

We ran only the read-only public-origin preflight against the isolated Test
server env file, using scripts/pr6_managed_oauth_preflight.mjs.
Results: dedicated_public_https_origin=false, oauth2_enabled=false,
issuer_matches_public_origin=false. No endpoint or credential was published.
Do not try to bypass the blocked test PAT issuance using another interface.

## Exact user/operator action required

1. Keep the current Test Tunnel and production WebCodex connection unchanged.
   Operator chooses a separate approved HTTPS public origin for an
   isolated test server. It must route to that server only, protect the
   underlying bootstrap secret, and follow the WebCodex OAuth deployment
   guide. The current 127.0.0.1:18086 loopback listener is not a public origin.
2. In ChatGPT connector creation UI, create a separate test connection using
   the OAuth authorization flow, not the existing Tunnel/No authentication
   connector. Obtain ChatGPT's exact OAuth redirect URI from that flow.
   Only the URI (not code, token, client secret or PAT) may be shared back.
   The user authorizes the managed account pr6test.
3. A human operator uses officially supported first-party commands on
   the separate Test server: log in as the existing managed user through
   webcodex login. Then create a new profile using:

       webcodex connect <TEST_HTTPS_SERVER> --auth managed-oauth --user pr6test --oauth-redirect-uri <EXACT_CHATGPT_CALLBACK> --project <DISPOSABLE_TEST_WORKSPACE> --profile <UNIQUE_TEST_PROFILE>

   The command needs operator-controlled approved auth, never a tool-created
   credential in this test. Do not reuse an existing Runner identity or
   collide with an existing project registry.
4. The user completes OAuth login/consent in ChatGPT. Refresh the new
   connection so Host tool descriptors and MCP App resources are current.
   Do not paste credentials in chat.
5. First verify a read-only authenticated identity, matching Test Runner
   ownership and the scopes job:run, project:write, plugin:invoke, job:detach.
   Then use a fresh idempotency key to run one read-only Worker in the isolated
   test workspace. Independently inspect exact new Dispatch, Runner Job,
   Host Wait, terminal event, App carrier and a new GPT model turn. Never
   re-dispatch a Worker when a previous effect is uncertain.

ChatGPT's callback and a controlled public HTTPS test origin are prerequisites.
Do not present an OpenAI Tunnel-only No-auth connection as managed OAuth.

## Acceptance gate

PR #6 six-case independent Host gate remains blocked until independent
evidence includes real new-turn execution and isolation, restart, stale
Window and duplicate-delivery scenarios. Green Rust/Node CI or ui/message
acceptance alone does not establish an autonomous GPT wake.
