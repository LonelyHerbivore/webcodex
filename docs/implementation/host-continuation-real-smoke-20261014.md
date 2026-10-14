# ChatGPT Host continuation live smoke — 2026-10-14

## Scope and environment

**Observed:** two chained WebCodex MCP App Job terminal continuations within a single ChatGPT conversation on a Windows Runner. This was tested against the deployed WebCodex runtime `0.5.0-rc.2` (commit `683256d7`), **not** a deployed build of the diagnostic Agent/Job visibility PRs. No Orca Worker was started or modified.

The first probe executed a read-only 90-second delay Job, used one durable Job wait and MCP App presentation, then received a `ui://webcodex/job-terminal-continuation/v2` notification after the original assistant response. The resulting assistant turn observed the completed first Job and itself initiated a **second distinct** read-only 90-second Job, registered a new exact durable Wait and presented its own MCP App. After that second assistant response ended, a second `ui://webcodex/job-terminal-continuation/v2` notification arrived. The subsequent assistant turn actually queried the second completed Job. No new ordinary user-authored message appears between these model turns in this conversation.

## Independent observations (without inventing Host IDs)

| Observation | Evidence source | Outcome |
| --- | --- | --- |
| Job finished | WebCodex `observe_jobs` in the resumed assistant turn | `status=completed`, `exit_code=0`, marker `HOST_WAKE_PROBE_JOB_TERMINAL_OK` |
| Durable terminal Wait | Exact idempotency-key replay of `wait_for_job_terminal` | `state=triggered`, `delivery_state=delivered`, terminal succeeded, `replayed=true`, `state_changed=false` |
| MCP App delivery | Window ActionAudit | `prepare_job_terminal_continuation` at 1791976836398 ms and `finish_job_terminal_continuation` at 1791976839961 ms, both succeeded |
| Subsequent model tool activity | Separate WebCodex `observe_jobs` audit and actual current model tool call | `observe_jobs` at 1791976861935 ms succeeded, correlated by the same Job id supplied by the App notification |
| Production auto-resume declaration | Exact wait replay | `automatic_resume_available=false`, both before binding and on replay |
| Second Job completion | `observe_jobs` in the next assistant turn | `status=completed`, `exit_code=0`, marker `HOST_WAKE_CHAIN_ROUND2_OK` |
| Second durable Wait | Exact idempotency-key replay | `state=triggered`, `delivery_state=delivered`, `terminal_outcome=succeeded`, `replayed=true`, `state_changed=false`, `automatic_resume_available=false` |
| Second MCP App terminal delivery | Window ActionAudit | `prepare_job_terminal_continuation` at 1791977124364 ms, `finish_job_terminal_continuation` at 1791977127909 ms; both completed successfully |
| Second turn-level tool activity | Separate WebCodex audit and actual `observe_jobs` call | `observe_jobs` at 1791977149649 ms succeeded after the second finish; model queried the exact second Job |

**Interpretation:** two consecutive real application-origin Job-completion messages were each followed by an assistant response that executed a tool query against the correct completed Job. In particular, the *first continuation turn created and armed the second Job*, establishing a bounded two-hop chain. These observations are stronger than `ui/message` ACK alone, but are not cryptographic/signed Host turn-start attestations.

## What this does not establish

- No independently authenticated `fresh_turn_started` Host turn ID or Host-signed scheduling receipt is accessible here. WebCodex's ActionAudit logs tool-handler activity; it explicitly does **not** attest Host model turns or the absence of unrelated user/UI actions.
- The test did not establish that the browser iframe was hidden, suspended, closed, or restored from destruction during completion.
- It did not test an actual Agent Wake/Orca Coordinator. It did test **two chained Job notification continuations**, but not indefinite multi-turn autonomy or Agent work.
- Two positive unspecified-visibility observations do not qualify the six-case real-Host acceptance matrix.
- `automatic_resume_available=false` is a conservative readiness flag, not a proof that this particular active-card `ui/message` cannot result in execution. Do not silently switch `WEBCODEX_MCP_APP_RESUME_MODE` to `unattended` or advertise production readiness.

## Follow-up gates

1. **Completed:** bounded second Job continuation verified; no third Job started, and neither Job was rerun.
2. Preserve exact Wait/Job idempotency and replay checks on any subsequent test; never auto-restart a Worker from an uncertain outcome.
3. Independently test actual Agent Wake/Coordinator continuation, without changing production Orca resources.
4. Run a background-visibility test only with independently observable browser state; absent such telemetry the result must remain `visibility=unknown`.
5. Acquire authenticated Host model-turn and tool-call provenance before marking any six-case matrix entry `hostVerified`.

This file intentionally omits the live session's IDs, credential material, private transcript and raw source command path. No CLI action should be triggered from this diagnostic text.


## Real Agent Wake continuation (independent live test, 2026-10-14)

A controlled pre-existing diagnostic Agent identity (no Orca Worker) was reused on
WebCodex Runtime `0.5.0-rc.2`, not a build of this PR. A fresh generation-2 ChatGPT
Endpoint and separate one-message Conversation were created; the message was
addressed only to the diagnostic Agent. The Agent Continuation MCP App was
presented while Wake state was `pending`, the Binding was not yet established,
and `production_auto_resume_available=false`.

After the previous assistant turn ended, the real Host supplied an
`ui://webcodex/agent-continuation/v20` continuation containing the exact
Agent, Endpoint, generation, Wake identity, and a server-issued consume token.
**This initiated an actual new assistant turn, without an intervening ordinary
user-authored message in this conversation.** The resumed turn then performed
the following independent MCP operations:

1. `bootstrap_agent_conversation` for the exact Agent/Endpoint/generation,
   Conversation, and Wake. The server confirmed the Endpoint was
   `wake_capable=true`, `host_binding.adapter_kind=mcp_app`, Wake
   `state=delivered`, and precisely one queued Delivery.
2. `list_agent_inbox` and `read_conversation`. Both identified the same
   single diagnostic Message and Delivery; no business Message was supplied
   in the Host continuation envelope.
3. `consume_agent_wake` using **the exact Host-supplied token** returned
   `state=consumed`, `already_consumed=false`, `state_changed=true`.
4. Separately `consume_agent_deliveries` returned the same sole Delivery as
   consumed, without retry or duplicate.
5. Final `list_agent_identities` and transcript read showed zero active
   Endpoints, zero queued Deliveries and zero unresolved Wakes after cleanup.
   The test Endpoint was explicitly detached; the durable diagnostic Agent
   and Conversation were left intact.

No Orca Coordinator, Worker, shell Job, source file or production deployment
was created/modified by the Agent Wake itself. This validates one actual
active-card Agent Wake → resumed model/tool calls → exact consume/cleanup
chain. It does **not** attest true background/closed-tab delivery, or an
independently signed `fresh_turn_started` Host execution ID; the existing
six-case real-Host matrix remains unverified.

### Important reply ordering boundary

An optional diagnostic reply to the source Conversation was attempted **after**
`consume_agent_wake` with the otherwise valid `wake_reply_id` and
`reply_operation_index=0`. WebCodex correctly rejected it with
`error_kind=wake_already_consumed`, without changing durable state.
Do not retry or reactivate a consumed Wake to post a reply.

**For an Agent that needs to report inside a Conversation**, the safe order is:

- Bootstrap and independently read Inbox + Conversation.
- Process the exact work, within independently re-authorized scope.
- While the Wake is accepted and **not yet consumed**, post each desired
  Agent-authored reply with the supplied `wake_reply_id` and a stable
  per-send `reply_operation_index`. On a response loss, replay only that same
  exact operation index/body; never invent a new reply identity.
- Consume the exact Wake once, then separately consume only processed Delivery
  IDs. Finally release/detach the test Endpoint if the scenario is complete.

A report in this original ChatGPT continuation turn is distinct from a
durable Conversation Message; the latter was **not** posted in this smoke
test. Ensure the Orca Coordinator follows this sequencing when implementing
real Agent-to-Agent reporting and never treats a rejected post as a completed
reply.

