// Diagnostic-only, offline evidence consistency check.
// NEVER interpret this as attestation that ChatGPT scheduled an unattended turn.
// Records must be independently collected from authenticated Host and tool traces.
const MATRIX = Object.freeze([
  "agent.short.foreground", "agent.long.foreground", "agent.long.background",
  "job.short.foreground", "job.long.foreground", "job.long.background",
]);
export const HOST_CONTINUATION_CASES = MATRIX;
const isObject = value => !!value && typeof value === "object" && !Array.isArray(value);
const identity = value => typeof value === "string" && /^[a-zA-Z0-9_:-]{1,128}$/.test(value);
const timestamp = value => {
  if (typeof value !== "string"
      || !/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d{1,3})?Z$/.test(value)) return null;
  const ms = Date.parse(value);
  if (!Number.isFinite(ms)) return null;
  // Reject normalized invalid calendar dates (e.g. February 30) and 24:00:00.
  let canonicalInput = value.replace(/\.(\d{1,2})Z$/, (_, digits) => "." + digits.padEnd(3, "0") + "Z");
  if (!canonicalInput.includes(".")) canonicalInput = canonicalInput.slice(0, -1) + ".000Z";
  return new Date(ms).toISOString() === canonicalInput ? ms : null;
};

export function assessHostContinuationCase(sample) {
  const problems = [];
  const add = code => { if (!problems.includes(code)) problems.push(code); };
  if (!isObject(sample)) return { caseId: null, mechanicallyConsistent: false, problems: ["invalid_case"], hostVerified: false };
  const caseId = sample.caseId;
  if (!MATRIX.includes(caseId)) add("invalid_case_id");
  const kind = typeof caseId === "string" ? caseId.split(".")[0] : null;
  const original = sample.originalTurn;
  const trigger = sample.trigger;
  const dispatch = sample.hostDispatch;
  const fresh = sample.freshTurn;
  const consumed = sample.continuation;
  if (![original, trigger, dispatch, fresh, consumed].every(isObject)) add("missing_independent_event");
  if (!identity(sample.conversationKey) || !identity(sample.principalKey)) add("missing_scope");
  if (!identity(original?.conversationKey) || original?.conversationKey !== sample.conversationKey) add("original_conversation_mismatch");
  const visibility = isObject(sample.hostEnvironment) ? sample.hostEnvironment.visibilityAtTrigger : null;
  if (caseId && MATRIX.includes(caseId) && visibility !== caseId.split(".")[2]) add("visibility_mismatch");
  if (!identity(original?.id) || !identity(fresh?.id) || original?.id === fresh?.id) add("turn_identity_missing_or_reused");
  if (!identity(fresh?.conversationKey) || fresh?.conversationKey !== sample.conversationKey) add("conversation_mismatch");
  if (!identity(trigger?.principalKey) || trigger?.principalKey !== sample.principalKey) add("principal_mismatch");
  if (!identity(trigger?.eventId) || !identity(dispatch?.eventId) || trigger?.eventId !== dispatch?.eventId) add("event_mismatch");
  if (!identity(fresh?.eventId) || fresh?.eventId !== trigger?.eventId) add("fresh_turn_event_mismatch");
  if (!identity(consumed?.eventId) || consumed?.eventId !== trigger?.eventId) add("consumption_event_mismatch");
  if (!identity(consumed?.turnId) || consumed?.turnId !== fresh?.id) add("consumption_turn_mismatch");

  // An MCP App ACK is not model activity. A separate Host turn trace and a
  // separately captured model tool invocation are both required.
  if (fresh?.source !== "host_turn_trace" || consumed?.source !== "model_tool_trace") add("missing_independent_traces");
  if (fresh?.initiatedWithoutUserAction !== true || sample.userActionsAfterFinal !== 0) add("not_zero_click");
  if (dispatch?.outcome !== "dispatch_accepted") add("host_dispatch_not_accepted");
  if (sample.duplicateDispatchCount !== 0 || sample.duplicateWorkerStartCount !== 0
      || sample.duplicateFreshTurnCount !== 0) add("duplicate_side_effect_not_ruled_out");

  if (kind === "job") {
    if (consumed?.tool !== "observe_jobs" || !identity(trigger?.jobId)
        || trigger?.jobId !== consumed?.jobId) add("wrong_job_observation");
  } else if (kind === "agent") {
    if (consumed?.tool !== "consume_agent_wake" || !identity(trigger?.wakeId)
        || trigger?.wakeId !== consumed?.wakeId) add("wrong_agent_consumption");
  }

  const ordered = [
    original?.finalAt, trigger?.at, dispatch?.preparedAt, dispatch?.acceptedAt,
    fresh?.startedAt, consumed?.at,
  ].map(timestamp);
  if (ordered.some(x => x === null)) add("invalid_utc_timestamp");
  else {
    if (ordered.some((x, i) => i > 0 && x < ordered[i - 1])) add("event_time_order");
    // The published six-case Host matrix distinguishes ~30s from ~5min.
    // A foreground short probe must not be counted as a background long probe.
    const delay = (ordered[1] - ordered[0]) / 1000;
    const duration = typeof caseId === "string" ? caseId.split(".")[1] : null;
    if (duration === "short" && (delay < 15 || delay > 120)) add("wrong_delay_window");
    if (duration === "long" && (delay < 240 || delay > 900)) add("wrong_delay_window");
  }
  // A continuation in another session or with a fabricated tool event must
  // not pass even when its host ACK is present.
  return { caseId: MATRIX.includes(caseId) ? caseId : null,
    mechanicallyConsistent: problems.length === 0, problems, hostVerified: false };
}

export function assessHostContinuationMatrix(samples) {
  if (!Array.isArray(samples)) return { mechanicallyConsistent: false, hostVerified: false,
    missingCases: [...MATRIX], duplicateCases: [], results: [] };
  const seen = new Set();
  const duplicates = new Set();
  const results = samples.map(sample => {
    if (MATRIX.includes(sample?.caseId)) {
      if (seen.has(sample.caseId)) duplicates.add(sample.caseId);
      seen.add(sample.caseId);
    }
    return assessHostContinuationCase(sample);
  });
  const missingCases = MATRIX.filter(x => !seen.has(x));
  return {
    mechanicallyConsistent: missingCases.length === 0 && duplicates.size === 0
      && results.length === MATRIX.length && results.every(x => x.mechanicallyConsistent),
    hostVerified: false, // Requires independently authenticated real-Host audit.
    missingCases, duplicateCases: [...duplicates].sort(), results,
  };
}
