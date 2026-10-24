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
