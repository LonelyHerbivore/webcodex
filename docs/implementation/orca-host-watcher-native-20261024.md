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
