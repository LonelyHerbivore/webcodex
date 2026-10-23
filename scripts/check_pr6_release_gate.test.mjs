import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { checkPr6ReleaseGate, CASE_IDS } from "./check_pr6_release_gate.mjs";

const ledger = JSON.parse(readFileSync(
  new URL("../docs/implementation/host-continuation-acceptance-status-20261022.json", import.meta.url),
  "utf8"));
const mutate = fn => { const copy = structuredClone(ledger); fn(copy); return copy; };

test("current isolated candidate is connected but offline gate does not authorize release", () => {
  const r = checkPr6ReleaseGate(ledger);
  assert.deepEqual(r, { valid: true, releaseAuthorized: false, issues: [], casesNotRun: 6, totalCases: 6 });
  assert.deepEqual(ledger.real_host_cases.map(x => x.id), CASE_IDS);
});
test("bad candidate identity or non-isolated Host fails", () => {
  for (const change of [
    x => { x.candidate_commit = "0".repeat(40); },
    x => { x.test_connector_route_check.same_server_process = true; },
    x => { x.test_connector_route_check.observed_test_server_commit = "683256d7cd93267f65cce301055cacf17667fd54"; },
    x => { x.test_connector_route_check.observed_test_server_pid = x.test_connector_route_check.observed_production_server_pid; },
  ]) assert.equal(checkPr6ReleaseGate(mutate(change)).valid, false);
});
test("offline status cannot self-declare release authorization", () => {
  for (const name of ["release_ready", "production_auto_resume_available", "independent_host_turn_evidence"]) {
    const r = checkPr6ReleaseGate(mutate(x => { x.readiness[name] = true; }));
    assert.ok(r.issues.includes("unverified_release_claim"), name);
    assert.equal(r.releaseAuthorized, false);
  }
});
test("blocked attempt never counts as executed, a completed Job or a model turn", () => {
  for (const change of [
    x => { x.first_candidate_job_probe.job_created = true; },
    x => { x.first_candidate_job_probe.alternate_execution_attempted = true; },
    x => { x.first_candidate_job_probe.count_as_case_run = true; },
    x => { x.first_candidate_job_probe.model_fresh_turn_verified = true; },
  ]) assert.ok(checkPr6ReleaseGate(mutate(change)).issues.includes("pre_dispatch_block_misrepresented"));
});
test("matrix must contain six unique unrun cases", () => {
  for (const change of [
    x => { x.real_host_cases.pop(); },
    x => { x.real_host_cases[0].id = x.real_host_cases[1].id; },
    x => { x.real_host_cases[0].status = "passed"; },
  ]) assert.ok(checkPr6ReleaseGate(mutate(change)).issues.includes("real_host_matrix_mismatch"));
});
test("invalid input fails closed, never exposes raw private fields", () => {
  assert.equal(checkPr6ReleaseGate(null).releaseAuthorized, false);
  const contaminated = mutate(x => { x.secret = "VERY_PRIVATE_TEST_SENTINEL"; });
  assert.equal(JSON.stringify(checkPr6ReleaseGate(contaminated)).includes("VERY_PRIVATE_TEST_SENTINEL"), false);
});
