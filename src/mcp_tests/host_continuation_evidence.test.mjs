import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { assessHostContinuationCase, assessHostContinuationMatrix, HOST_CONTINUATION_CASES } from "./host_continuation_evidence.mjs";

const fixture = caseId => {
  const kind = caseId.startsWith("job.") ? "job" : "agent";
  const tag = caseId.replaceAll(".", "_");
  const timePrefix = caseId.includes(".long.") ? "10:05:" : "10:00:";
  const second = caseId.includes(".long.") ? "00" : "30";
  return {
    caseId, conversationKey: `conversation_fixture_${tag}`, principalKey: `principal_fixture_${tag}`,
    originalTurn: { id: `turn_original_${tag}`, conversationKey: `conversation_fixture_${tag}`, finalAt: "2026-10-14T10:00:00Z" },
    hostEnvironment: { visibilityAtTrigger: caseId.split(".")[2] },
    trigger: { eventId: `event_fixture_${tag}`, principalKey: `principal_fixture_${tag}`,
      at: `2026-10-14T${timePrefix}${second}Z`, jobId: `job_fixture_${tag}`, wakeId: `wake_fixture_${tag}` },
    hostDispatch: { eventId: `event_fixture_${tag}`, outcome: "dispatch_accepted",
      preparedAt: `2026-10-14T${timePrefix}${caseId.includes(".long.") ? "01" : "31"}Z`,
      acceptedAt: `2026-10-14T${timePrefix}${caseId.includes(".long.") ? "02" : "32"}Z` },
    freshTurn: { id: `turn_fresh_${tag}`, eventId: `event_fixture_${tag}`,
      conversationKey: `conversation_fixture_${tag}`, source: "host_turn_trace",
      initiatedWithoutUserAction: true,
      startedAt: `2026-10-14T${timePrefix}${caseId.includes(".long.") ? "03" : "33"}Z` },
    continuation: { source: "model_tool_trace", eventId: `event_fixture_${tag}`,
      turnId: `turn_fresh_${tag}`, tool: kind === "job" ? "observe_jobs" : "consume_agent_wake",
      jobId: `job_fixture_${tag}`, wakeId: `wake_fixture_${tag}`,
      at: `2026-10-14T${timePrefix}${caseId.includes(".long.") ? "04" : "34"}Z` },
    userActionsAfterFinal: 0, duplicateDispatchCount: 0, duplicateWorkerStartCount: 0,
    duplicateFreshTurnCount: 0,
  };
};
const edit = (caseId, fn) => { const value = fixture(caseId); fn(value); return value; };

test("all six synthetic cases can be mechanically consistent but never host-verified", () => {
  const samples = HOST_CONTINUATION_CASES.map(fixture);
  const report = assessHostContinuationMatrix(samples);
  assert.equal(report.mechanicallyConsistent, true);
  assert.equal(report.hostVerified, false);
  assert.equal(report.results.length, 6);
  for (const result of report.results) {
    assert.equal(result.mechanicallyConsistent, true);
    assert.equal(result.hostVerified, false);
  }
});

test("Host ACK alone is not a new model turn", () => {
  const result = assessHostContinuationCase(edit("job.short.foreground", sample => {
    delete sample.freshTurn;
    delete sample.continuation;
  }));
  assert.equal(result.mechanicallyConsistent, false);
  assert.ok(result.problems.includes("missing_independent_event"));
});

test("a user-initiated turn or message is not zero-click wake", () => {
  for (const sample of [
    edit("job.long.background", s => { s.freshTurn.initiatedWithoutUserAction = false; }),
    edit("job.long.background", s => { s.userActionsAfterFinal = 1; }),
  ]) assert.ok(assessHostContinuationCase(sample).problems.includes("not_zero_click"));
});

test("different conversation, principal, wake or turn ID never correlates", () => {
  const changes = [
    ["conversation_mismatch", s => { s.freshTurn.conversationKey = "conversation_other"; }],
    ["original_conversation_mismatch", s => { s.originalTurn.conversationKey = "conversation_other"; }],
    ["principal_mismatch", s => { s.trigger.principalKey = "principal_other"; }],
    ["event_mismatch", s => { s.hostDispatch.eventId = "event_other"; }],
    ["fresh_turn_event_mismatch", s => { s.freshTurn.eventId = "event_other"; }],
    ["consumption_event_mismatch", s => { s.continuation.eventId = "event_other"; }],
    ["consumption_turn_mismatch", s => { s.continuation.turnId = "turn_other"; }],
    ["turn_identity_missing_or_reused", s => { s.freshTurn.id = s.originalTurn.id; }],
  ];
  for (const [expected, change] of changes) {
    const result = assessHostContinuationCase(edit("agent.long.background", change));
    assert.ok(result.problems.includes(expected), expected);
    assert.equal(result.mechanicallyConsistent, false);
  }
});

test("must observe exact Job or consume exact Agent wake via an actual tool trace", () => {
  for (const [caseId, change, expected] of [
    ["job.short.foreground", s => { s.continuation.jobId = "wrong_job"; }, "wrong_job_observation"],
    ["agent.long.foreground", s => { s.continuation.wakeId = "wrong_wake"; }, "wrong_agent_consumption"],
    ["agent.long.background", s => { s.continuation.tool = "observe_jobs"; }, "wrong_agent_consumption"],
    ["job.long.background", s => { s.continuation.tool = "get_job_terminal_continuation_state"; }, "wrong_job_observation"],
  ]) assert.ok(assessHostContinuationCase(edit(caseId,change)).problems.includes(expected));
});

test("ambiguous delivery, duplicate starts and duplicate dispatches fail closed", () => {
  const changes = [
    ["host_dispatch_not_accepted", s => { s.hostDispatch.outcome = "delivery_unknown"; }],
    ["duplicate_side_effect_not_ruled_out", s => { s.duplicateWorkerStartCount = 1; }],
    ["duplicate_side_effect_not_ruled_out", s => { s.duplicateDispatchCount = 1; }],
    ["duplicate_side_effect_not_ruled_out", s => { s.duplicateFreshTurnCount = 1; }],
    ["duplicate_side_effect_not_ruled_out", s => { delete s.duplicateWorkerStartCount; }],
  ];
  for (const [expected, change] of changes) assert.ok(
    assessHostContinuationCase(edit("job.long.background",change)).problems.includes(expected));
});

test("invalid or out-of-order UTC timestamps cannot pass", () => {
  for (const [expected, change] of [
    ["invalid_utc_timestamp", s => { s.freshTurn.startedAt = "2026-10-14 10:00:33"; }],
    ["event_time_order", s => { s.freshTurn.startedAt = "2026-10-14T09:00:33Z"; }],
    ["invalid_utc_timestamp", s => { s.trigger.at = null; }],
  ]) assert.ok(assessHostContinuationCase(edit("agent.short.foreground",change)).problems.includes(expected));
});

test("claimed Host turn and model tool traces must be separately identified", () => {
  for (const change of [
    s => { s.freshTurn.source = "mcp_app_ui_message_ack"; },
    s => { s.continuation.source = "mcp_app_ui_message_ack"; },
  ]) assert.ok(assessHostContinuationCase(edit("job.long.foreground",change)).problems.includes("missing_independent_traces"));
});

test("matrix rejects missing, duplicate and malformed cases", () => {
  const cases = HOST_CONTINUATION_CASES.map(fixture);
  const missing = assessHostContinuationMatrix(cases.slice(0,5));
  assert.equal(missing.mechanicallyConsistent,false);
  assert.deepEqual(missing.missingCases, ["job.long.background"]);
  const duplicated = assessHostContinuationMatrix([...cases.slice(0,5), cases[0]]);
  assert.equal(duplicated.mechanicallyConsistent,false);
  assert.deepEqual(duplicated.duplicateCases,["agent.short.foreground"]);
  assert.equal(assessHostContinuationCase(null).hostVerified,false);
  assert.equal(assessHostContinuationMatrix(null).hostVerified,false);
});

test("each case must match visibility and actual clock delay, not just a label", () => {
  const short = edit("job.short.foreground", s => { s.hostEnvironment.visibilityAtTrigger = "background"; });
  assert.ok(assessHostContinuationCase(short).problems.includes("visibility_mismatch"));
  const mislabeled = edit("agent.long.background", s => { s.trigger.at = "2026-10-14T10:00:30Z"; });
  assert.ok(assessHostContinuationCase(mislabeled).problems.includes("wrong_delay_window"));
  const tooQuick = edit("job.short.foreground", s => { s.trigger.at = "2026-10-14T10:00:05Z"; });
  assert.ok(assessHostContinuationCase(tooQuick).problems.includes("wrong_delay_window"));
});

test("nonexistent UTC calendar dates cannot be normalized into valid evidence", () => {
  for (const at of ["2026-02-30T10:00:00Z", "2026-10-14T24:00:00Z", "2026-13-14T10:00:00Z"]) {
    const result = assessHostContinuationCase(edit("job.short.foreground", sample => { sample.trigger.at = at; }));
    assert.ok(result.problems.includes("invalid_utc_timestamp"), at);
  }
});

test("malformed case labels cannot echo arbitrary private content", () => {
  const invalid = edit("job.short.foreground", sample => { sample.caseId = "secret conversation text"; });
  const result = assessHostContinuationCase(invalid);
  assert.equal(result.caseId, null);
  assert.deepEqual(result.problems.includes("invalid_case_id"),true);
  assert.equal(JSON.stringify(result).includes("secret conversation text"),false);
});

test("UTC timestamps with fractional seconds preserve exact chronology", () => {
  const sample = edit("job.short.foreground", s => {
    s.hostDispatch.acceptedAt = "2026-10-14T10:00:32.125Z";
    s.freshTurn.startedAt = "2026-10-14T10:00:33.1Z";
    s.continuation.at = "2026-10-14T10:00:34.05Z";
  });
  assert.equal(assessHostContinuationCase(sample).mechanicallyConsistent,true);
});

test("six-case matrix rejects reused conversations and event IDs", () => {
  const cases = HOST_CONTINUATION_CASES.map(fixture);
  const first = cases[0], second = cases[1];
  second.conversationKey = first.conversationKey;
  second.originalTurn.conversationKey = first.conversationKey;
  second.freshTurn.conversationKey = first.conversationKey;
  second.trigger.eventId = first.trigger.eventId;
  second.hostDispatch.eventId = first.trigger.eventId;
  second.freshTurn.eventId = first.trigger.eventId;
  second.continuation.eventId = first.trigger.eventId;
  const result = assessHostContinuationMatrix(cases);
  assert.equal(result.mechanicallyConsistent,false);
  assert.ok(result.duplicateEvidenceFields.includes("conversationKey"));
  assert.ok(result.duplicateEvidenceFields.includes("trigger.eventId"));
  assert.equal(result.hostVerified,false);
});

test("offline audit CLI returns sanitized outcomes and distinct exit codes", () => {
  const cli = fileURLToPath(new URL("../../scripts/assess_host_continuation_evidence.mjs", import.meta.url));
  const valid = HOST_CONTINUATION_CASES.map(fixture);
  const success = spawnSync(process.execPath,[cli],{input:JSON.stringify(valid),encoding:"utf8"});
  assert.equal(success.status,0,success.stderr);
  const parsed = JSON.parse(success.stdout);
  assert.equal(parsed.mechanicallyConsistent,true);
  assert.equal(parsed.hostVerified,false);
  assert.equal(success.stdout.includes("principal_fixture"),false);
  assert.equal(success.stdout.includes("conversation_fixture"),false);

  const incomplete = spawnSync(process.execPath,[cli],{input:JSON.stringify(valid.slice(0,1)),encoding:"utf8"});
  assert.equal(incomplete.status,1);
  assert.equal(JSON.parse(incomplete.stdout).mechanicallyConsistent,false);

  const malformed = spawnSync(process.execPath,[cli],{input:"not-json",encoding:"utf8"});
  assert.equal(malformed.status,2);
  assert.equal(malformed.stdout,"");
  assert.match(malformed.stderr,/Invalid host continuation evidence JSON/);
});
