# Host-owned Orca Worker → Watcher Job → Wait — CI diagnostic

This isolated GitHub branch adds two Rust Host tools after the confirmed Worker-start stage.

## Watcher admission

start_orca_host_watcher_job accepts only the original durable idempotency key. It reads the exact caller-owned Worker/Run/Coordinator/Dispatch/Task identity and checks Runner/project ownership, job:run, job:detach, project:write, and DetachedProcessJobs support before changing any state. The first irreversible transition is worker_start_confirmed → watch_start_dispatched, saved before Runner execution. The original Runner Job ID is saved on watch_start_dispatched → watch_job_confirmed.

The watcher source is first-party Host code in src/tool_runtime/orca_host_watcher.mjs, compiled into the Rust module. The Host starts it with structured direct argv in a keyed detached Runner Job. No arbitrary script path returned by a Plugin is executed. It reads only run-current, worker-show and check --peek, checks the exact run and worker/terminal/message identities, then writes one bounded attention result. It never ACKs, answers, retries or releases Orca work.

## Wait admission

register_orca_host_wait_and_present accepts only the same durable intent key. It verifies the saved watcher Job/Project binding, and records wait_registration_dispatched BEFORE calling existing Host wait_for_job_terminal with the original deterministic wait key. On confirmation it records wait_registered and registered. It presents the original authorized Host card through the existing Host presentation gate. A repeated registered request only returns the original wait ID.

All uncertain external-effect phases reject automatic re-execution and require independent reconciliation. Node/Orca availability and Runner detachable execution capability are required. One-Worker first-cycle only; further explicitly ACKed question rearm uses separate policy-controlled semantics.

## Acceptance status

The GitHub CI workflow compiles Rust Host/Store, runs durable Store, Worker, Job, Wait and binding tests, and executes Node watcher fixtures. This is diagnostic code: no production Host or Tunnel deployment and no claim of an unattended ChatGPT model turn. Final acceptance must use a separately authorized real Orca Runner and Host continuation.


Compatibility update: The Native Plugin adds an opt-in orca_start_worker_host_bound tool with projectBound=true while retaining the legacy orca_start_worker unchanged. Host exact binding accepts only the bound alias. No old manual Plugin client is required to supply a new Project field.

## New single-call Host orchestration

start_orca_worker_with_continuation requires the new opt-in Native Plugin alias
orca_start_worker_host_bound with projectBound=true. It first verifies the
exact Runner/Project and detached-process capability, plus job:run,
job:detach, project:write and plugin:invoke. Only then does it call the
existing Host crash-fenced Worker start, bundled watcher admission, and
original Host Wait registration functions sequentially. After each step it
requires the newly confirmed stage, rather than accepting a replayed or
uncertain result as a new effect. Partial errors retain the proven
Dispatch/Task/Job/Wait references for manual reconciliation.

The old unbound orca_start_worker is retained for legacy callers and
is rejected by the new Host composite binding check. F-drive Native Plugin
alpha.17 was tested against actual WebCodex Runner metadata: the bound
alias requires a Project on describe, while legacy caller semantics remain.

This single tool provides only an orchestrated authorized registration in
the invoking model turn. The Host must separately validate an ACTUAL
subsequent ChatGPT model turn to claim successful unattended wake.

## Isolated Windows build gate

The diagnostic GitHub workflow adds a Windows Server 2022 x64 compilation job for
webcodex-server.exe. It verifies the native Rust target, GitHub checkout SHA,
Windows PE machine type, binary --version, and first-party watcher Node fixture.
An artifact is uploaded for five days with an exact SHA-256 manifest and a
diagnostic-only warning. This is NOT a signed release or a production update;
never replace the active WebCodex Server/Tunnel using this build without
separate operator authorization and isolated end-to-end acceptance.


## Windows diagnostic Host/CLI/Runner trio

The independent GitHub CI Windows job now compiles the exact source revision of
webcodex-server.exe, webcodex.exe, and webcodex-runner.exe together. A diagnostic
manifest contains all three binary SHA-256 hashes, source SHA, exact Windows x64
target and run attempt. The validator rejects an absent/substituted binary,
unknown additional file entry, wrong PE architecture, changed revision and
the earlier single-server manifest when trio verification is explicitly required.
Source/release/Runner executable identity is verified, not inferred from filenames.

The artifact is a short-lived **unsigned** diagnostic bundle. Do not replace
the production Host, Runner or Tunnel, and do not start an old Test Tunnel
candidate as though it exercises this branch. Isolated tests require a matched
candidate build and an explicit safe Test lifecycle.

## Precise P0 carrier evidence

The Host Wait admission response and one-call Worker->Watcher->Wait response
now report automatic_resume_available using the **actual JobTerminalContinuationController
for the original principal/Wait**. Unknown, user-confirmed-only and unbound
carriers are false; a presented card, successful Job, durable Wait and completed
CI remain insufficient evidence of a new ChatGPT model turn. The combined
tool also reports no_proven_unattended_carrier when capability cannot be
established. host_wake_verified remains false until an independently observed
future turn.


## Verified Orca CLI read-only stable-pane compatibility

A live read-only inspection in dsh-image-user-question confirmed that
orchestration run-current --from term_* and orchestration check
--terminal term_* --peek return stable_pane_required from ordinary Git Bash.
A detached Runner Job cannot assume it owns a stable Orca terminal.

The first-party watcher now uses orchestration run-show --id run_* and
orchestration inbox --terminal run:run_* --limit 100 instead, both observed
working from a non-Orca-shell process. It independently validates the
Coordinator binding and each message's Run/recipient/Worker provenance.
The inbox API returns both read and unread messages: the watcher considers
only unread read=0 rows and never ACKs. It rejects unknown read state and
a saturated 100-row window rather than silently omitting older messages.
Live read-only worker-show succeeded for two already-terminal Workers; no
new Worker or message was created. The final unattended ChatGPT continuation
still needs a real connected Host carrier and an actual new model turn.

## Live isolated Test Tunnel owner admission (2026-10-24)

A distinct diagnostic Host/Runner/Test Tunnel from commit 2d72924 was launched
on loopback 127.0.0.1:18086 with the native Orca Plugin alpha.17 copied into
test-only storage and the single Runner project mapped to dsh-image-user-question.
The independently connected ChatGPT Test app confirmed the **exact source
commit**, one online Runner/project, and plugin status Ready with 40 tools.
An exact Project-bound orca_start_worker_host_bound describe returned a valid
binding. Production Host, Runner, Tunnel and F: Plugin were untouched.

One authorized-looking Host composite call was attempted with a new unique
idempotency key, current Orca Run and a **read-only** Codex worker specification.
The composite returned worker_start_dispatched_or_existing, while
read_orca_host_intent returned orca_host_intent_unauthorized. Independent
read-only Orca task-list/worker-list showed **only the prior two completed
Tasks/Dispatches**: no new worker was observed. No duplicate dispatch was
attempted. The tested Connector's trusted_agent runtime policy did not imply
an acceptable managed Host Intent principal: the inner store correctly
denied the caller. This exposed an error-reporting gap in the composite.

The revised composite explicitly preflights the managed owner kind before
any Project, Plugin or Worker action, and returns
orca_host_composite_preflight_denied with worker_started=false and
host_wake_verified=false. It NEVER weakens the original Host authorization.
A Test app with supported authenticated managed owner identity is required
before the real Worker/Job/Wait and independent new GPT-turn gate can proceed.

The interactive Test lifecycle was closed by sending an explicit EOF to
the single owned Job. WebCodex CLI, Runner and Server all exited code 0;
the Test Tunnel lock became absent and profile remains disabled. The
existing dsh-image-user-question Git status retains its pre-existing
screenshots.json deletion; it was not altered by this test.
