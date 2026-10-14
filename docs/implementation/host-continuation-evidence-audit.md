# Host continuation: six-case evidence audit (diagnostic only)

This is an **offline consistency check**, not a production Host wake adapter or a method of starting model turns.

The only successful outcome currently returned by the checker is `mechanicallyConsistent: true` with `hostVerified: false`. That means a provided collection of records has the required independent fields and consistency constraints. **No provided JSON can prove its own authenticity.** Real-Host acceptance requires a human/auditor to obtain authorized Host turn telemetry and tool-call logs from independently trusted records.

## Why this exists

`ui/message` can be acknowledged by the ChatGPT Host without a verified subsequent model execution. A 2026-10-14 diagnostic Job measured 2,608 seconds from terminal event to Host delivery settlement, while no new ChatGPT turn was established by that observation. The MCP iframe timer can be suspended when hidden.

The six cases in the canonical `mcp-app-auto-resume-restoration.md` matrix are:

| Case ID | Kind | Trigger after original final | Visibility at trigger |
| --- | --- | --- | --- |
| `agent.short.foreground` | Agent | ~30 seconds | foreground |
| `agent.long.foreground` | Agent | ~5 minutes | foreground |
| `agent.long.background` | Agent | ~5 minutes | background |
| `job.short.foreground` | Job | ~30 seconds | foreground |
| `job.long.foreground` | Job | ~5 minutes | foreground |
| `job.long.background` | Job | ~5 minutes | background |

The checker accepts 15–120 seconds as a short trigger window and 240–900 seconds as a long one, subject to UTC chronology. These are **diagnostic tolerances**, not Host scheduling SLAs. It does not measure a real browser's visibility itself; the `hostEnvironment.visibilityAtTrigger` must be independently recorded, not guessed or copied from the case ID.

## Running the checker

Keep the evidence JSON **locally**, outside Git and outside C: (for example on D:). For a test-only record, in Git Bash:

```bash
node scripts/assess_host_continuation_evidence.mjs < /d/host-wake-evidence.local.json
```

The input is a JSON array with **exactly six** separate case records. The checker never writes a file, calls ChatGPT, dispatches a Worker or exposes raw identifiers in its output. It returns exit code 0 for structurally complete and consistent records, 1 for incomplete/inconsistent records, 2 for invalid JSON. **Exit 0 still reports `hostVerified:false`**.

The following is an illustrative **synthetic** one-case Job record. It is not evidence of a real Host pass:

```json
{
  "caseId": "job.short.foreground",
  "conversationKey": "conversation_fixture",
  "principalKey": "principal_fixture",
  "originalTurn": {
    "id": "turn_original",
    "conversationKey": "conversation_fixture",
    "finalAt": "2026-10-14T10:00:00Z"
  },
  "hostEnvironment": {"visibilityAtTrigger": "foreground"},
  "trigger": {
    "eventId": "event_fixture",
    "principalKey": "principal_fixture",
    "jobId": "job_fixture",
    "at": "2026-10-14T10:00:30Z"
  },
  "hostDispatch": {
    "eventId": "event_fixture",
    "outcome": "dispatch_accepted",
    "preparedAt": "2026-10-14T10:00:31Z",
    "acceptedAt": "2026-10-14T10:00:32Z"
  },
  "freshTurn": {
    "id": "turn_fresh",
    "eventId": "event_fixture",
    "conversationKey": "conversation_fixture",
    "source": "host_turn_trace",
    "initiatedWithoutUserAction": true,
    "startedAt": "2026-10-14T10:00:33Z"
  },
  "continuation": {
    "turnId": "turn_fresh",
    "eventId": "event_fixture",
    "source": "model_tool_trace",
    "tool": "observe_jobs",
    "jobId": "job_fixture",
    "at": "2026-10-14T10:00:34Z"
  },
  "userActionsAfterFinal": 0,
  "duplicateDispatchCount": 0,
  "duplicateWorkerStartCount": 0,
  "duplicateFreshTurnCount": 0
}
```

For Agent cases, use `wakeId` under both `trigger` and `continuation`, and `continuation.tool = "consume_agent_wake"`.

## Source trust and failure handling

- Obtain original final-turn time and **new-turn identity and start time** from independently authenticated ChatGPT Host execution traces. `ui/message` result `{}` is **not** a model-turn trace.
- Obtain the `observe_jobs` / `consume_agent_wake` call and its actual Host turn ID from a distinct trusted tool-call trace. Merely displaying a model response is insufficient.
- Correlate identical principal, conversation, exact event and new turn. Confirm no new user action after the original final, no duplicate Host dispatch, no duplicate fresh turn and no duplicate Worker start.
- If the Host cannot expose these traces, mark the real case **unverified**. Do not invent turn IDs, copy the source labels into synthesized events, reinterpret ACK as execution or lower the required number of cases.
- Do not upload real conversation text, API keys, authorization headers, token material, raw claim/binding fences, or sensitive Host trace payloads. Use bounded pseudonymous IDs and local records; only publish aggregate problem codes.

This checker is deliberately independent of both Orca Coordinator state and the MCP App transport: it cannot restart Workers, manufacture fresh-turn proof, or bypass ChatGPT permissions. It remains a diagnostic prerequisite rather than a substitute for the missing production Host scheduling interface.
