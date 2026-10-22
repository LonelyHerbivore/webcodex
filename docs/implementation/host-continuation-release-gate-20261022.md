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


## Observed production Host Job continuation (2026-10-22; **not** PR #6 acceptance)

- Host build actually used: `0.5.0-rc.2 / 683256d7`; no isolated PR #6 binary or MCP connection was active. Browser visibility was **not independently observed**, so this cannot be classified as `job.short.foreground` in the candidate's official six-case matrix.
- Began one harmless 45-second delayed Node Job (`wc_job_f2LEmtCq0p7XaAwY`), and registered one exact durable Wait (`wc_job_wait_W2hjfeYW2kFhcAjT`) with a stable idempotency key before the original assistant response finalized.
- `present_job_terminal_continuation` returned `waiting/not_ready`. After the original assistant final, the real ChatGPT Host produced a `ui://webcodex/job-terminal-continuation/v2` continuation with the same exact Job ID. This was followed by the assistant's automatic continued turn, whose actual model tool call `observe_jobs` independently returned `completed`, `exit_code=0`, with output marker `PR6_HOST_CONTINUATION_BASELINE_JOB_OK`. No Job was restarted.
- A keyed replay of the exact Wait returned `state=triggered`, `delivery_state=delivered`, `replayed=true`, `state_changed=false`, `automatic_resume_available=false`; the Window ActionAudit separately observed `present_job_terminal_continuation` then a later `observe_jobs` request for the same Job.
- This is a **positive real production Host app-triggered continuation and exact Job observation**, stronger than an ACK-only claim. The Host has not provided an independently attested `fresh_turn_started` model Turn ID or trustworthy visibility/zero-user-action audit. The PR #6 acceptance ledger's six `not_run` entries remain unchanged. This cannot establish background/closed-tab autonomous scheduling or justify production promotion.
- The isolated Windows Git Bash environment has no `cargo` or `rustc` on PATH and no PR #6 test Host binary in its checkout. The authorized chat connection is still attached to the original production Host. A separate candidate registration/build is required before testing PR #6's exact implementation.

## Exact PR #6 Windows binary build attempt (2026-10-22)

- Windows and WSL2 Ubuntu have no usable local Rust toolchain on PATH, and the PR #6 CI artifacts contained no deployable Windows Server binary.
- Created GitHub-only isolated branch `diagnostic/host-candidate-binary-20261022` from exact PR #6 head `c2d19dc9af63e966af5509a17ebb0b568cb1731d`. No modifications were made to PR #6, its Draft flag, its parent PR, or the production WebCodex environment.
- The fork's registered `Host continuation diagnostic acceptance` workflow (ID 383906171) was extended **only on that independent branch**, with a Windows build job guarded by `github.event_name == 'workflow_dispatch'`. Build checks out the immutable PR #6 SHA, compiles `--profile dogfood -p webcodex --bin webcodex-server`, checks `--version` against the exact source identity and uploads a 3-day test artifact. Other build steps continue to run their existing smoke checks.
- [Manual isolated binary build #39718838414](https://github.com/LonelyHerbivore/webcodex/actions/runs/39718838414) was accepted and started. Final build/artifact status must be read from GitHub; **at this documentation checkpoint the Windows build step was still running**, so it has not been called successful or deployed.
- Prepared a fail-closed D-drive-only smoke harness `scripts/isolated_pr6_windows_host_smoke.mjs`, syntax-checked. Only after obtaining an independently verified exact candidate binary should it create fresh isolated data, generate a one-use in-memory bootstrap token, bind to loopback, verify `/healthz`, confirm anonymous MCP is rejected, then stop on standard-input EOF. No real Runner enrollment, Host credential transfer, or browser simulation is authorized.
- An authorized ChatGPT MCP connection to the isolated candidate still must be configured separately before genuine fresh-model-turn cases; green binary/startup smoke is not real ChatGPT Host acceptance. The six candidate case statuses remain `not_run`.

## Current status

- GitHub CI and diagnostic CI: **PASS** at exact PR #6 SHA.
- Existing Node/Store recovery regression: **PASS in CI**, no claim of real WAN failure.
- Agent/Job Host fresh-turn, hidden/closed-page behavior: **UNVERIFIED for PR #6**.
- Host native unattended scheduler: **not established**.
- Release decision: **BLOCKED on independent real-Host acceptance**.
