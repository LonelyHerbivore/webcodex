# Host continuation release gate — 2026-10-22

**Priority updated 2026-10-23:** [normal-network GPT automatic continuation comes first](host-continuation-active-wake-priorities-20261023.md). P0 = real Agent completion -> fresh GPT turn -> exact Wake consumed -> next task. Network fluctuations deferred to P2. This changes the implementation order, not the historical full-release audit.

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
- The isolated Windows Git Bash and WSL environments have no local `cargo`/`rustc` on PATH. A PR #6 Windows candidate has since been compiled via GitHub CI and tested on loopback; the authorized ChatGPT MCP connection, however, still targets the original production Host and does not register this isolated candidate.

## Exact PR #6 Windows binary build attempt (2026-10-22)

- Windows and WSL2 Ubuntu have no usable local Rust toolchain on PATH, and the PR #6 CI artifacts contained no deployable Windows Server binary.
- Created GitHub-only isolated branch `diagnostic/host-candidate-binary-20261022` from exact PR #6 head `c2d19dc9af63e966af5509a17ebb0b568cb1731d`. No modifications were made to PR #6, its Draft flag, its parent PR, or the production WebCodex environment.
- The fork's registered `Host continuation diagnostic acceptance` workflow (ID 383906171) was extended **only on that independent branch**, with a Windows build job guarded by `github.event_name == 'workflow_dispatch'`. Build checks out the immutable PR #6 SHA, compiles `--profile dogfood -p webcodex --bin webcodex-server`, checks `--version` against the exact source identity and uploads a 3-day test artifact. Other build steps continue to run their existing smoke checks.
- [Manual isolated binary build #39718838414](https://github.com/LonelyHerbivore/webcodex/actions/runs/39718838414) completed **success** (Windows Host binary verification, upload, Rust Host/Store contract and MCP App regression). The Windows artifact ID is `12400203704`, with exactly pinned source commit `c2d19dc9af63e966af5509a17ebb0b568cb1731d`. This GitHub run's **workflow commit** is the isolated diagnostic branch `58f07f9`; the produced binary identifies the fixed PR #6 source, not the diagnostic commit.
- Downloaded `webcodex-server.exe` to `.pr6-host-candidate-20261022/` under this D: checkout; verified the artifact SHA256 `a2d9eda2370816738dee8cfd1d649eb8ec8b6148c5c2d7e1a0e694957385ff84` and its `0.5.0 (commit c2d19dc9..., dirty=false, built_at=1792033983)` identity. No production binary was changed.
- Executed `scripts/isolated_pr6_windows_host_smoke.mjs` twice on **real Windows** with new isolated data, one-use in-memory bootstrap tokens and loopback-only sockets. The final run recorded `/healthz` HTTP 200, anonymous MCP HTTP 401, authorized `initialize`/`tools/list` HTTP 200, both registered `present_agent_continuation` / `present_job_terminal_continuation` descriptors, and clean Server `exit_code=0` on stdin EOF. Test evidence is `.pr6-host-candidate-20261022/smoke-evidence-mcp.json`; no Agent/Worker was created and no authenticated ChatGPT turn was fabricated.
- An authorized ChatGPT MCP connection to the isolated candidate still must be configured separately before genuine fresh-model-turn cases; green binary/startup smoke is not real ChatGPT Host acceptance. The six candidate case statuses remain `not_run`.


## Exact PR #6 local Runner join and Tunnel handoff (2026-10-23)

- [Manual exact-source runtime build #39909520529](https://github.com/LonelyHerbivore/webcodex/actions/runs/39909520529) completed successfully, producing `pr6-windows-runtime-c2d19dc9` (Artifact `12481869213`). The Windows workflow checked out immutable PR #6 HEAD `c2d19dc9af63e966af5509a17ebb0b568cb1731d` and compiled `webcodex.exe`, `webcodex-server.exe`, and `webcodex-runner.exe`. All three SHA256 manifest entries and their exact Commit/version/build-time lines were verified on the laptop after downloading to `.pr6-host-candidate-20261022/runtime-trio/`.
- Created a **local-only** configuration under `.pr6-host-candidate-20261022/isolated-host/`, protected with a restricted Windows D-drive ACL. Address: `127.0.0.1:18086`; data, temporary files, enrollment connections, and the randomly generated bootstrap credential reside only in that directory. Credentials are never written to documentation, tool output, Git or ChatGPT. `WEBCODEX_MCP_APP_RESUME_MODE=unknown`, anonymous access off, shared-key off, QUIC off.
- Reused the disposable `.pr6-host-candidate-20261022/acceptance-workspace/` as the **only** Project/allowed root for a separately paired Runner. The CLI securely consumed one one-time pairing code using `--code-stdin`, wrote the Runner config under the explicit D-drive `--dir` and passed `webcodex-runner --check-config`. The CLI output path needed to be resolved against its true working directory; only the generated in-scope configuration was then reused. No duplicate pairing or Worker launch occurred.
- In one complete, real Windows process test, the isolated Server started and passed authentication. The separate Runner connected via WebSocket; MCP `get_runtime_status` reported `runners.online_count=1`, one Project, zero Workers started. `--stop-on-stdin-eof` stopped the test Runner and Server cleanly, both exit code 0. Production Server `0.5.0-rc.2 / 683256d7`, original Runner and Orca were untouched.
- **External blocker:** a distinct OpenAI Secure MCP Tunnel identity and restricted control-plane key have not been configured in the local execution environment or Windows user environment. Registering a custom ChatGPT MCP app requires the user's explicit Tunnel selection/authorization. Do not reuse the production Tunnel identity or elevate credentials merely for a test. No isolated candidate is currently connected to ChatGPT; all six real Host turn cases remain `not_run`, release blocked.


## ChatGPT 「WebCodex 测试」 connector routing inspection (2026-10-23)

- The user created and explicitly selected a separate ChatGPT connector named **WebCodex 测试**. Its authorized read-only `get_runtime_status` succeeded. A simultaneous read through **WebCodex 幻16** returned exactly the **same production Server process PID 13236**, version `0.5.0-rc.2`, Git commit `683256d7cd93267f65cce301055cacf17667fd54`, Runner instance `20c220ec-cee0-4b81-80d4-8de8e4fc51d5`, and 27 registered projects. Thus two app entries currently route to the **same production Server**, despite distinct UI names.
- The isolated candidate binary and local config still exist at D-drive loopback `127.0.0.1:18086`; there was no listener on port 18086 when inspected. The isolated env file, current process env and Windows user env did not contain a test `CONTROL_PLANE_TUNNEL_ID` or `CONTROL_PLANE_API_KEY` (presence checks only; no secret values accessed).
- **Next authorization gate:** provision an independent OpenAI Secure MCP Tunnel targeting the isolated PR #6 Server, then bind the **WebCodex 测试** ChatGPT app to that exact tunnel via the user's developer-mode UI. Merely naming another connector does not move its upstream route. Preserve production Tunnel/Server. Re-run both app `get_runtime_status` and demand different Host commit/PID and exactly one test Runner/project before claiming PR #6 app routing. No automatic model turns or candidate Host acceptance have been run through this test connector yet.


## Confirmed Desktop Test-Tunnel ownership and prepared safe handoff (2026-10-23)

- User clarified **WebCodex 测试** was created with a **new** Tunnel. Read-only Desktop configuration inspection confirmed two distinct profiles, `ChatGPT` and `Test`, each with its own Tunnel ID and API Key, both enabled and configured to autostart. Only profile names, booleans, revisions and credential **presence** were reported; no secret values left the machine.
- Matching each profile's Tunnel ID against the native RunFence filename via SHA256 of `https://api.openai.com/{tunnel_id}` established that **ChatGPT** is currently owned by `webcodex.exe` PID **4844**, **Test** by PID **12648**. Both processes use Desktop's *same production* `C:\Users\asus\AppData\Local\dev.webcodex.desktop\runtime\local\webcodex.env`; hence even the new Test identity currently forwards to the production Host, and cannot constitute PR #6 acceptance.
- Both active RunFence markers exist and both owning processes were verified as `webcodex.exe`; production WebCodex Server PID is **13236**. Never remove an `.active`/`.lock` marker or launch the same Tunnel identity twice. The **Test** profile must first be gracefully stopped in Desktop and its autostart disabled; leave **ChatGPT** enabled and running.
- Added `scripts/pr6_isolated_test_tunnel.mjs` with `status` and `run` modes; syntax and safe status preflight passed. It selects the exact named **Test** identity from the Desktop profile store, checks it differs from production and its RunFence is absent and autostart off, verifies the fixed PR #6 binaries' three checksums, and launches the isolated Server/Runner/Tunnel only on `127.0.0.1:18086`. Tunnel ID and API Key remain **memory-only in the Tunnel child**; no copies to D or logs, no credential in command-line arguments. The launcher prevents unmanaged clipboard writes and requires an observed control-plane ready event. Closing its stdin gracefully stops the owned processes; uncertain stop state remains fenced.
- At the recorded preflight, **Test was still active at PID 12648 and autostart was enabled**. The new launcher correctly returns `ready_to_start=false`; therefore no new Tunnel process, credential changes, real ChatGPT turn or real Host acceptance occurred. Only the Test Tunnel's Desktop owner should be stopped through normal UI; do not kill PID 12648, and do not touch production PID 4844.


## Test Tunnel stop verified; restart fence requires recovery approval (2026-10-23)

- Following the user's “Test 已停止”, Desktop `Test` is `enabled=false, autostart=false`; its previous owner PID `12648` is absent. Production `ChatGPT` is still `enabled=true, autostart=true`, and its owner remains alive. The `Test` lock file can be acquired non-blockingly and released, proving no current lock holder at observation time.
- The *Test* `.active` fence still exists. According to `crates/webcodex-openai-tunnel/src/run_fence.rs`, clean completed delivery clears the fence; persistence after a dead owner means `tunnel_restart_uncertain`. Its absence of an owner is **not** proof of completed notification/response delivery. No PID was killed and no `.active`/`.lock` was removed or renamed.
- Corrected `scripts/pr6_isolated_test_tunnel.mjs`: manual ownership requires `enabled=false` and `autostart=false` in Desktop, and a fully cleared/explicitly recovered exact-run fence. The safe preflight still blocks (`ready_to_start=false`).
- Windows environment-recovery apply in `crates/webcodex-environment/src/tunnel_recovery.rs` explicitly rejects non-Unix apply; the native `run_fence::archive` supports exact-run operator archival only after holding the exclusive lock, proving the recorded owner absent and **accepting prior-effect uncertainty**. Do not casually delete the marker or fake completed delivery. Request explicit user approval before exact-run archival, preserving a private `.recovered/{run_id}` record, with zero replay of earlier requests.
- PR #6 candidate 0/6 independent ChatGPT Host cases remains unchanged; no new Tunnel started.


## Operator-authorized Test fence recovery and real PR #6 ChatGPT routing (2026-10-23)

- User explicitly authorized archival of the exact **Test** Tunnel's uncertain prior run after reviewing the no-replay risk. Rechecked Desktop `Test enabled=false, autostart=false` and `ChatGPT enabled=true`; previous Test owner PID `12648` was absent (Windows process lookup), while production owner PID `4844` and its fence persisted. Acquired the Test identity lock nonblocking, reread the exact unchanged Version 2 marker/Run ID, and atomically **renamed** its original `.active` marker into the immutable `.active.recovered/{exact-run-id}` archive directory. No old operation was resent; no source, data, credentials, or production marker were altered.
- Started existing `scripts/pr6_isolated_test_tunnel.mjs run` as one **interactive, Runner-owned Job** `wc_job_yh1vVWJ6ZrIUwFxf` (not a hidden unmanaged service). The launcher validated all three exact-source binary SHA256 hashes, left Desktop Test disabled, and started local Host at `127.0.0.1:18086`, one isolated Runner and the **existing distinct Test Tunnel identity**. Its safe machine output reported `tunnel_ready=true` and `local_mcp_ready=true`. The process stays alive only while its owning Job remains open; use its Job input/stop lifecycle for shutdown, not PID kills.
- **Direct live ChatGPT test app proof**: the user-selected **WebCodex 测试** plugin `get_runtime_status` now reports **`0.5.0`, SHA `c2d19dc9af63e966af5509a17ebb0b568cb1731d`, Server PID `7140`, Runner `pr6-acceptance-20261023` at the same source, and exactly one test project**. The separate production app continues to return **`0.5.0-rc.2`, SHA `683256d7...`, Server PID `13236`, and 27 projects**. A direct `read_files` via **WebCodex 测试** successfully read the isolated acceptance workspace README. These are independently observed through both ChatGPT connectors, beyond local Tunnel-ready logs.
- First attempted `job.short.foreground` execution through the test app's `run_job` gateway was **blocked by an OpenAI safety check before dispatch**; there is **no Job ID, no started test worker, and no Host fresh model turn** from this attempt. Respect that block; do not route the same effect through alternate execution tools to bypass it. All six PR #6 candidate real-Host cases stay `not_run`. Release remains blocked pending permitted end-to-end fresh-turn evidence.

## Execution admission diagnosis — 2026-10-23

- The Test app's first `run_job` call was rejected by an explicit ChatGPT safety check before a usable Server Job was created. Read-only Host/RPC probes and `list_runners` independently confirm test Runner `shell/jobs/async_jobs/async_shell_jobs=true`, `allow_raw_shell=true`, one allowed sandbox root, max timeout 3600 seconds, zero Job concurrency usage. `run_job` registry declares `execute/job_run`, `approval=standard`, `job:run`. Thus no local Runner admission defect has been identified.
- Candidate Server retained ActionAudit contains no matching `run_job` call. Server trace capture was off, so the precise ChatGPT safety rule and backend-routing disposition are not independently observable. Official ChatGPT guidance says certain MCP actions can be safety-blocked without an approval prompt, and permissive app settings cannot override this.
- Full sanitized support-ready evidence: [PR6 ChatGPT execution-tool safety block](pr6-chatgpt-run-job-safety-block-20261023.md). Respect the refusal; no alternate tool execution to circumvent it. Six candidate Host fresh-turn cases remain **0/6** and PR #6 stays Draft.


## Offline PR #6 release ledger guard (FastCtx development, 2026-10-23)

- Added \`scripts/check_pr6_release_gate.mjs\` and six \`node:test\` regression cases in \`scripts/check_pr6_release_gate.test.mjs\`. These operate on diagnostic JSON only; no Tunnel, Host, Runner, Job, Wake, or private credential is accessed.
- Verifies candidate SHA consistency, distinct isolated/production Host identity, offline ledger schema, six unique **not_run** case records, and that a ChatGPT safety-blocked call did not create a Job or falsely count as model continuation. Rejects forged \`release_ready\`, \`independent_host_turn_evidence\`, or \`production_auto_resume_available\` flags.
- Normal invocation: \`node scripts/check_pr6_release_gate.mjs\`. A valid blocked-status ledger exits **0** with \`releaseAuthorized=false\`; mismatched gate exits **1**; unreadable or malformed input exits **2**. The script intentionally has **no authority to approve any release**.
- Ran through WebCodex's registered **FastCtx** (production-side MCP gateway), targeting only this isolated diagnostic checkout. Existing Agent Wake/Job Terminal/evidence \`node --test\` suites passed **241/241**; new guard tests passed **6/6**. The independent six real Host turn cases remain **0/6**, PR #6 Draft unchanged. This is regression infrastructure progress, not a workaround for the blocked \`run_job\` invocation.

## Background auto-resume limitation proven by App contract (FastCtx audit)

- **Job terminal MCP App:** \`src/mcp_job_terminal_continuation_app.html\` checks \`document.hidden\` before \`prepare_job_terminal_continuation\`, and, if the tab becomes hidden during prepare, finishes \`delivery_unknown\` without sending \`ui/message\`. The existing \`hidden tab keeps completed Job pending; foreground re-entry dispatches exactly once\` and \`switching to hidden during prepare\` unit tests encode that behavior.
- **Agent Wake MCP App:** \`src/mcp_agent_continuation_app.html\` likewise blocks hidden-tab \`ui/message\`, with regression coverage for background deferral and hidden-during-prepare uncertainty. Both are deliberate duplicate-prevention contracts, not accidental timer failures.
- Therefore **the App-only visible-tab carrier cannot deliver a fresh ChatGPT model turn while the tab remains hidden** in either published \`*.long.background\` acceptance case. Green offline App tests prove suppression, not background autonomy. Background acceptance requires a separately available, explicitly authorized **Host-native unattended scheduler/carrier** with its own independently attested model turn and exactly-once intent. Never simulate success with a shell watcher, forged user action or looping \`ui/message\`.
- This is an additional **architectural release blocker**, distinct from the earlier ChatGPT safety refusal for \`run_job\`. No unattended Host-native carrier has been verified here. The PR #6 real matrix remains \`not_run\`; do not claim background success merely because the user returns to the foreground later.

## Current status

- GitHub CI and diagnostic CI: **PASS** at exact PR #6 SHA.
- Existing Node/Store recovery regression: **PASS in CI**, no claim of real WAN failure.
- Agent/Job Host fresh-turn, hidden/closed-page behavior: **UNVERIFIED for PR #6**.
- Host native unattended scheduler: **not established**.
- Release decision: **BLOCKED on independent real-Host acceptance**.
