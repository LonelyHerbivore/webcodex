# PR #6 — ChatGPT execution-tool safety block (2026-10-23)

## Scope and release decision

**Finding:** The user's real **WebCodex 测试** ChatGPT plugin reaches the exact PR #6 candidate Host (`c2d19dc9af63e966af5509a17ebb0b568cb1731d`). It can read a test Project, but the first legitimate asynchronous `run_job` request returned the ChatGPT-side message **“此工具调用被 OpenAI 的安全检查屏蔽。请仔细检查你发送的内容。”** before any usable WebCodex Job ID was issued.

**Impact:** The intended first genuine Host fresh-turn case `job.short.foreground` never started. The six PR #6 candidate Host acceptance scenarios remain **0/6 / not_run**. The release gate stays **BLOCKED**; this is **not a failure of the PR #6 auto-resume implementation**, which has not yet been exercised end to end.

## Environment verified via real connected apps

- The user explicitly authorized isolated testing and separately created a Secure MCP Tunnel and the ChatGPT plugin named **WebCodex 测试**.
- Test app `get_runtime_status`: WebCodex Server `0.5.0`, commit `c2d19dc9af63e966af5509a17ebb0b568cb1731d`, PID `7140`; online Runner `pr6-acceptance-20261023` at the same commit and one disposable Project.
- Production app independently reported `0.5.0-rc.2 / 683256d7cd93267f65cce301055cacf17667fd54`, PID `13236`, 27 Projects. No production process or identity was modified.
- Tunnel health emitted `tunnel_ready=true` and `local_mcp_ready=true`; the Test app successfully read `README.md` from the sandbox Project.
- `list_runners` through **Test** reported connected WebSocket Runner, `shell=true`, `jobs=true`, `async_jobs=true`, `async_shell_jobs=true`, `allow_raw_shell=true`, `max_timeout_secs=3600`, allowed root limited to the sandbox Project, and `job_concurrency=4` with zero jobs running/queued.
- `read_tool_manifest(run_job)` through **Test** reports registered `run_job` with `effect=execute`, `risk=job_run`, `approval=standard`, `authority.scopes=[job:run]`, and native gateway `call_runtime_tool`; schema supports `command`, `project`, `session_id`, `shell`, `timeout_secs`, `purpose` and `assertion_name`.

## One failed attempt; no retries

The actual test request was a disposable `node -e` timer with a 65-second delayed single marker, `timeout_secs=100`, purpose `test`, target `agent:pr6-acceptance-20261023:acceptance-workspace`, session `wc_sess_XEIdBiT7ZIYVK8jj`. Its only intended effect was to print a harmless marker, allowing the Host continuation wait to be tested. The call was dispatched via the candidate app's specified `call_runtime_tool` gateway to `run_job`, and the platform blocked the tool call.

No Job ID was returned. `list_runners` subsequently reported zero active/queued Jobs. No `wait_for_job_terminal` registration or Host continuation card occurred. **No alternate execution entry point was used to work around the safety refusal.** Calling another shell/run tool to accomplish the denied effect is out of scope.

The Test Host's `read_tool_trace` query for the exact Project and `run_job` returned zero retained ActionAudit calls and `capture_mode=off`; `read_current_window_activity` displayed successful read-only calls but no `run_job` event. Since trace capture is off, the absence of a trace alone does not reveal the internal reason. The explicit upstream refusal message is the primary evidence of a safety block before a usable Server receipt.

## What is and is not established

1. **Verified:** Both ChatGPT plugin and candidate MCP Server are reachable, and the Runner can advertise execution capabilities, with a sandbox-only allowed root.
2. **Verified:** ChatGPT can separately decline an execution tool invocation without an approval prompt. Official documentation states that some especially risky MCP actions may be blocked and are not unlocked by permissive app settings.
3. **Unverified:** The internal rule that triggered this specific decision. The request payload, tool risk classification, account/workspace entitlement, tool invocation context, and platform safety decision are plausible dimensions; no private policy reason was exposed.
4. **Unverified:** The candidate Host's automated new model turn for any of the six scenarios. No functional failure can be assigned to PR #6 from this blocked precondition.

## Recommended support escalation / official references

Request review of this exact false-positive (if applicable) without trying to disable safety controls. Include: the localized refusal text, October 23 2026, the allowed test-only job purpose and no-sensitive-data marker, app name, tool `run_job` (`execute/job_run/job:run`), exact candidate SHA, isolated Runner policy/capabilities and the fact that there is **no WebCodex Job ID**. Supply ChatGPT conversation URL or client-side request ID privately to OpenAI Support if available; never publish Tunnel keys, credentials, agent conversation contents or production logs.

Official guidance:
- https://help.openai.com/en/articles/12584461-connecting-custom-mcp-servers-in-chatgpt
- https://help.openai.com/en/articles/20001497-troubleshooting-plugins-and-apps-in-chatgpt

No package upgrade, permission escalation, Host fake ACK, duplicate Job creation, Tunnel restart, production change, or Draft PR promotion is proposed on this evidence.
