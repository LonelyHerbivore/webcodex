# Host continuation release gate — 2026-10-22

**Classification: diagnostic only / NOT release-ready.** This is an operator handoff and evidence inventory, not a ChatGPT Host turn attestation. Do not merge PR #6 or deploy the Host candidate based on GitHub CI alone.

## Immutable code and CI evidence

- Superseded PR: [#5](https://github.com/LonelyHerbivore/webcodex/pull/5), **closed, unmerged**.
- Active replacement: [#6](https://github.com/LonelyHerbivore/webcodex/pull/6), **Draft**, stacked on [#4](https://github.com/LonelyHerbivore/webcodex/pull/4). Exact head: `c2d19dc9af63e966af5509a17ebb0b568cb1731d`.
- [CI PR run #39713539023](https://github.com/LonelyHerbivore/webcodex/actions/runs/39713539023): `completed/success`, same exact head. Server Rust, Runner Rust, MCP conformance and relevant Node/contract checks passed; platform jobs marked `skipped` are not claims of Windows/macOS runtime acceptance.
- [Host continuation diagnostic acceptance #39713484909](https://github.com/LonelyHerbivore/webcodex/actions/runs/39713484909): `completed/success`, same head; **Rust Host/Store compile check** and **MCP App bridge and delivery regression** each `success`.
- Production WebCodex remains `0.5.0-rc.2 / 683256d7`; this is not the PR #6 candidate. Isolated checkout: `D:\webcodex-orca-plugin\.host-network-recovery-20261014`; local commit graph is different due to GitHub file-API commits, but source equivalence was verified previously for the five PR files.
- All CI runs above are automated coverage, **not** authenticated ChatGPT fresh-turn evidence.

## Current authoritative blockers

1. Source `src/agent_wake.rs`: `EndpointContinuationCarrier::McpApp` has no `push_adapter()`; its View must pull/read state and invoke `ui/message`. `EndpointContinuationCarrier::Push` is an adapter abstraction, not evidence that this ChatGPT Host has a registered authorized scheduler.
2. This session's read-only `list_agent_identities` reports `production_auto_resume_available=false` for the existing owned Agent; that is a conservative readiness projection, not permission to flip `WEBCODEX_MCP_APP_RESUME_MODE` to `unattended`.
3. `ui/message` acceptance, Runner Job success, Shell mail printing and `delivered` do **not** mean a later model turn actually started. Current Host `read_current_window_activity` also explicitly cannot attest model turn state.
4. No verified PR #6 isolated Server + registered test MCP App Host connection is attached to the original ChatGPT conversation here. Consequently, the six real Host cases below remain **unexecuted** on this candidate. Do not send forged Window/session IDs, browser credentials or simulated user interactions.
5. Page closed/browser killed is a separate additional endurance case not covered by the published six cases. A destroyed iframe cannot itself schedule a new turn; pending durable state can be recovered only through an authorized Host surface.

## Production-independent real Host acceptance matrix

Create six **new, disposable test conversations** and independently authorized Agent or Job stimulus. In each case mount the exact PR #6 candidate App, end the presenting assistant turn normally, and then do **not** send user input, force a turn through browser automation, or rely on Shell output.

| Case | Stimulus after original final | Browser state at trigger | Fresh-turn proof |
| --- | --- | --- | --- |
| agent.short.foreground | 30 seconds | visible | Host turn trace + same Wake `consume_agent_wake` |
| agent.long.foreground | 5 minutes | visible | Host turn trace + same Wake `consume_agent_wake` |
| agent.long.background | 5 minutes | hidden | Host turn trace + same Wake `consume_agent_wake` |
| job.short.foreground | 30 seconds | visible | Host turn trace + exact Job `observe_jobs` |
| job.long.foreground | 5 minutes | visible | Host turn trace + exact Job `observe_jobs` |
| job.long.background | 5 minutes | hidden | Host turn trace + exact Job `observe_jobs` |

For each case, retain only redacted independent UTC evidence: original final, triggering event, prepare, dispatch accepted/unknown, subsequent **different** model Turn ID, actual consuming/observing model tool trace, user action count, duplicate dispatch/start/turn counts, principal and conversation correlation. Never export raw secret keys, binding fences, claims, message content, or private ChatGPT/session credentials. Use the existing offline `scripts/assess_host_continuation_evidence.mjs` only for **mechanical consistency**; its `hostVerified=false` is intentionally non-overridable by synthetic data. An independent authenticated Host-turn audit is required for actual release approval.

## Execution order and go/no-go rules

1. Preserve PR #6 as Draft and retain the exact SHA above. Do not change the passing source while assembling acceptance evidence.
2. Obtain an independently isolated test Host connection that registers the built PR #6 `present_agent_continuation` and `present_job_terminal_continuation` tools in the original authorized ChatGPT Host; isolate Store/database, users, ports, credentials and Runner Projects. Preserve the live Host binary and settings. The temporary `WEBCODEX_MCP_APP_RESUME_MODE` remains `unknown`.
3. Verify tool/resource descriptors, safe authentication scopes and resource readback before running a model wake case. Build and record identity; mismatched build invalidates the attempt.
4. Run the six cases; after each, verify the exact Wake/Wait state and duplicate-effect counters. Any `delivery_unknown` is fail-closed: inspect authoritative state, **never** replay Host message/Worker just to get a green case.
5. Independently audit the actual ChatGPT Host turn and model tool trace, not just WebCodex's own response receipt. Missing independent turn evidence means **not passed**.
6. Optional supplementary cases: closed tab/reopen, browser freeze, physical WAN interruption/restoration, server restart, and page replacement. Distinguish *recovery on reopen* from *turn started while tab was closed*.
7. Only after genuine Host attestation and safety review should the isolated deployment be considered for promotion. Never merge stacked Draft PR #6 and #4 directly into production without reviewing parent ordering and release scope.

## Current status

- GitHub CI and diagnostic CI: **PASS** at exact PR #6 SHA.
- Existing Node/Store recovery regression: **PASS in CI**, no claim of real WAN failure.
- Agent/Job Host fresh-turn, hidden/closed-page behavior: **UNVERIFIED for PR #6**.
- Host native unattended scheduler: **not established**.
- Release decision: **BLOCKED on independent real-Host acceptance**.
