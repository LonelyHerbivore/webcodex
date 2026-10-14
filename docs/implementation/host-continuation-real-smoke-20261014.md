# ChatGPT Host continuation live smoke — 2026-10-14

## Scope and environment

**Observed:** a single ChatGPT conversation using WebCodex MCP App Job terminal continuation on a Windows Runner. This was tested against the deployed WebCodex runtime `0.5.0-rc.2` (commit `683256d7`), **not** a deployed build of the diagnostic Agent/Job visibility PRs. No Orca Worker was started or modified.

The probe executed a read-only, single, 90-second delay Job that printed one marker to stdout, then used one durable Job wait and the MCP App presentation. The original assistant turn ended before the Job completion notification appeared. The subsequent incoming `ui://webcodex/job-terminal-continuation/v2` message supplied an exact terminal notification and initiated the next assistant response without a newly typed user instruction observed in this conversation.

## Independent observations (without inventing Host IDs)

| Observation | Evidence source | Outcome |
| --- | --- | --- |
| Job finished | WebCodex `observe_jobs` in the resumed assistant turn | `status=completed`, `exit_code=0`, marker `HOST_WAKE_PROBE_JOB_TERMINAL_OK` |
| Durable terminal Wait | Exact idempotency-key replay of `wait_for_job_terminal` | `state=triggered`, `delivery_state=delivered`, terminal succeeded, `replayed=true`, `state_changed=false` |
| MCP App delivery | Window ActionAudit | `prepare_job_terminal_continuation` at 1791976836398 ms and `finish_job_terminal_continuation` at 1791976839961 ms, both succeeded |
| Subsequent model tool activity | Separate WebCodex `observe_jobs` audit and actual current model tool call | `observe_jobs` at 1791976861935 ms succeeded, correlated by the same Job id supplied by the App notification |
| Production auto-resume declaration | Exact wait replay | `automatic_resume_available=false`, both before binding and on replay |

**Interpretation:** one real application-origin follow-up message was processed by a new assistant response, which actually queried the completed Job, not just a Host ACK or a mocked test. The observed process and tool chain is materially stronger than `ui/message` ACK alone.

## What this does not establish

- No independently authenticated `fresh_turn_started` Host turn ID or Host-signed scheduling receipt is accessible here. WebCodex's ActionAudit logs tool-handler activity; it explicitly does **not** attest Host model turns or the absence of unrelated user/UI actions.
- The test did not establish that the browser iframe was hidden, suspended, closed, or restored from destruction during completion.
- It did not test an actual Agent Wake/Orca Coordinator or multiple autonomous round trips.
- A single positive foreground or unspecified-visibility observation does not qualify the six-case real-Host acceptance matrix.
- `automatic_resume_available=false` is a conservative readiness flag, not a proof that this particular active-card `ui/message` cannot result in execution. Do not silently switch `WEBCODEX_MCP_APP_RESUME_MODE` to `unattended` or advertise production readiness.

## Follow-up gates

1. Separately validate a second chained zero-user-message Job completion, if initiated, and stop the probe after that second continuation; do not create an infinite wake loop.
2. Inspect exact Wait and Job with idempotent reads and verify no duplicate Job execution or Host dispatch.
3. Run a background-visibility test only with an independently observable browser state and the user's awareness; absent such telemetry the result must remain `visibility=unknown`.
4. Acquire authenticated Host model-turn and tool-call provenance before marking any six-case matrix entry `hostVerified`.

This file intentionally omits the live session's IDs, credential material, private transcript and raw source command path. No CLI action should be triggered from this diagnostic text.
