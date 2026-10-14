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
