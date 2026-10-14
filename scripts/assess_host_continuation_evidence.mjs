#!/usr/bin/env node
// Offline-only diagnostic: reads redacted Host/test audit records from stdin.
// Never contacts a Host, consumes a Wake, or requests a model turn.
import { readFileSync } from "node:fs";
import { assessHostContinuationMatrix } from "../src/mcp_tests/host_continuation_evidence.mjs";

let records;
try {
  records = JSON.parse(readFileSync(0, "utf8"));
} catch {
  process.stderr.write("Invalid host continuation evidence JSON on stdin\n");
  process.exitCode = 2;
}
if (process.exitCode !== 2) {
  const result = assessHostContinuationMatrix(records);
  // This output intentionally contains no raw IDs, conversation contents,
  // authentication metadata or Host trace details.
  process.stdout.write(JSON.stringify(result, null, 2) + "\n");
  if (!result.mechanicallyConsistent) process.exitCode = 1;
}
