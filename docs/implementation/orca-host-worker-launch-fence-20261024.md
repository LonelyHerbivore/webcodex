# Orca Host Worker launch fence — diagnostic implementation (2026-10-24)

This branch implements and CI-tests the **original Orca Worker launch stage** through the existing WebCodex Native Plugin Gateway. It is not a production release or proof of unattended ChatGPT wake.

## Current exact Host calls

1. `plugin_tool(action=describe, runner=<exact Runner>, plugin=orca, tool=orca_start_worker, project=<exact authorized Project>)` establishes a **Project-bound** opaque binding. The binding is observation identity, not authorization.
2. `start_orca_worker_with_host_intent` receives the Project, Run, Coordinator, task spec, agent, worktree, stable idempotency key and exact opaque binding. Host validates `job:run`, `project:write`, `plugin:invoke` and Runner access.
3. Host reserves a principal-owned SQLite intent with a SHA-256 digest of the canonical exact Project, Run, Coordinator, agent, worktree and spec. Raw task text and credentials are not stored in the intent.
4. Host validates that the retained binding corresponds to the exact Orca `orca_start_worker` tool on the same Runner and Project. The durable intent transitions **reserved → worker_start_dispatched** before external execution.
5. Host invokes the regular Plugin Gateway using the caller's original transport. That Gateway retains its scope checks, session permissions, Runner instance and provider/schema fencing, Project root checks and execution approval.
6. Only after a successful correlated Orca response with exact Run, Coordinator, Dispatch and Task identities does the Host transition to `worker_start_confirmed`. When anything is missing, uncertain or denied, retain `worker_start_dispatched` for independent reconciliation. The Host NEVER launches a replacement Worker in response to a repeat.

The previous `prepare_orca_host_worker_continuation` remains as an optional separate reservation tool. Its immutable digest now also includes Orca worktree and requires `project:write`. An existing reserved intent with an exact same request can be used by the new one-call launcher, but a progressed intent is never executed twice.

## Existing next-stage capability

`arm_and_present_job_terminal_continuation` already supports an **existing, independently authorized Runner Job**: create/replay its exact durable Wait and present an authorized still-waiting Host card in one call. It never creates an Orca watcher Job itself or certifies a later GPT turn.

The remaining missing link is a **trusted Host-owned watcher Job executable**. The Native Plugin currently returns an advisory absolute `node ...wait-orca-events-job.mjs` script path. Automatically executing an arbitrary path from Plugin output without an independent trusted Runner policy/executable origin check would create a command authority escalation. This branch does **not** do that. The watcher executable selection and Host-owned original Job ID persistence must be implemented before wiring the fully automatic Worker → Job → Wait chain.

## CI scope

`.github/workflows/host-continuation-diagnostic.yml` runs WebCodex/Store Rust `cargo check --locked --all-targets`, Store intent and Job terminal tests, Host original intent preparation, original Worker launch-fence and strict Plugin binding tests, plus MCP App regression. This is intentionally an independent `diagnostic/host-rust-intent-ci-20261024` branch.

The isolated tests validate schema, authorization refusals, binding source, durable replay blocking and result correlation. **They do not exercise an actual Orca Worker launch against a real Runner or ChatGPT wake.** Separate live and closed-tab Host acceptance still required.

Production Host, the original PR #6 branch, active Tunnel and the Orca test workspace must not be changed as a side effect of this diagnostic branch.
