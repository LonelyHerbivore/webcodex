#!/usr/bin/env node
// PR #6 diagnostic-only release guard. Offline data is never Host attestation.
import { readFileSync } from "node:fs";

export const CASE_IDS = Object.freeze([
  "agent.short.foreground", "agent.long.foreground", "agent.long.background",
  "job.short.foreground", "job.long.foreground", "job.long.background",
]);

const object = v => v !== null && typeof v === "object" && !Array.isArray(v);
const SHA = /^[0-9a-f]{40}$/;

export function checkPr6ReleaseGate(ledger) {
  const issues = [];
  const add = code => { if (!issues.includes(code)) issues.push(code); };
  if (!object(ledger)) return { valid: false, releaseAuthorized: false, issues: ["invalid_ledger"], casesNotRun: 0 };
  if (ledger.schema !== "webcodex.host-continuation-acceptance.v1" || ledger.pull_request !== 6) add("wrong_schema_or_pr");
  if (!SHA.test(ledger.candidate_commit || "")
      || ledger.candidate_commit !== ledger.isolated_runtime_trio?.exact_source_commit) add("candidate_identity_mismatch");
  const route = ledger.test_connector_route_check;
  if (route?.routed_to_pr6_candidate !== true
      || route?.same_server_process !== false || route?.same_runner_instance !== false
      || route?.observed_test_server_commit !== ledger.candidate_commit
      || route?.observed_production_server_pid === route?.observed_test_server_pid
      || !Number.isInteger(route?.observed_production_server_pid)
      || !Number.isInteger(route?.observed_test_server_pid)) add("host_isolation_unverified");
  if (ledger.isolated_runtime_trio?.candidate_chatgpt_connection_established !== true
      || ledger.isolated_runtime_trio?.one_registered_runner_seen_online !== true
      || ledger.isolated_runtime_trio?.registered_test_projects !== 1) add("candidate_runtime_not_verified");

  // Offline records may be verified for *consistency*, never promoted to a
  // Host-proven release. Real independent turn evidence is a different
  // authority and must never be synthesized from local flags or test fixtures.
  const readiness = ledger.readiness;
  if (readiness?.release_ready !== false
      || readiness?.production_auto_resume_available !== false
      || readiness?.independent_host_turn_evidence !== false) add("unverified_release_claim");

  const cases = ledger.real_host_cases;
  if (!Array.isArray(cases) || cases.length !== CASE_IDS.length
      || new Set(cases.map(v => v?.id)).size !== CASE_IDS.length
      || CASE_IDS.some(id => !cases.some(v => v?.id === id && v.status === "not_run"))) {
    add("real_host_matrix_mismatch");
  }
  const probe = ledger.first_candidate_job_probe;
  if (probe?.case_id !== "job.short.foreground"
      || probe?.tool_gateway_blocked_by_platform_safety !== true
      || probe?.job_created !== false || probe?.model_fresh_turn_verified !== false
      || probe?.count_as_case_run !== false || probe?.alternate_execution_attempted !== false) {
    add("pre_dispatch_block_misrepresented");
  }
  const count = Array.isArray(cases)
    ? cases.filter(v => CASE_IDS.includes(v?.id) && v?.status === "not_run").length : 0;
  return { valid: issues.length === 0, releaseAuthorized: false, issues,
    casesNotRun: count, totalCases: CASE_IDS.length };
}

if (process.argv[1] && import.meta.url === new URL("file://" + process.argv[1].replaceAll("\\", "/")).href) {
  const file = process.argv[2] || "docs/implementation/host-continuation-acceptance-status-20261022.json";
  let result;
  try { result = checkPr6ReleaseGate(JSON.parse(readFileSync(file, "utf8"))); }
  catch { process.stderr.write("PR6 release ledger cannot be read or parsed\n"); process.exit(2); }
  // Never emit environment paths, tokens, conversations, or raw ledger content.
  process.stdout.write(JSON.stringify(result, null, 2) + "\n");
  if (!result.valid) process.exitCode = 1;
}
