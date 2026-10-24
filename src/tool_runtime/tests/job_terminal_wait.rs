//! E3 generic Job terminal attention contract and registration-race coverage.

use super::super::*;
use super::support::*;
use crate::job_terminal_attention::{principal_for_auth, JobTerminalContinuationController};
use crate::runner_protocol::{RunnerCapabilities, RunnerJobUpdateRequest, RunnerRequest};
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use webcodex_store::{JobTerminalDeliveryState, JobTerminalWaitState};

async fn attention_runtime() -> (TempDir, ToolRuntime, Arc<crate::Database>) {
    let temp = tempfile::tempdir().unwrap();
    let db =
        Arc::new(crate::Database::open(&temp.path().join("job-terminal-attention.db")).unwrap());
    let controller = JobTerminalContinuationController::new(db.clone());
    let registry = Arc::new(
        crate::job_receipts::production_registry_with_terminal_attention(
            db.clone(),
            controller.clone(),
        )
        .await,
    );
    let runtime = ToolRuntime::new(registry, Arc::new(RuntimeInfo::default()))
        .with_job_terminal_attention(db.clone(), controller);
    (temp, runtime, db)
}

async fn attention_runtime_with_controller() -> (
    TempDir,
    ToolRuntime,
    Arc<crate::Database>,
    JobTerminalContinuationController,
) {
    let temp = tempfile::tempdir().unwrap();
    let db = Arc::new(
        crate::Database::open(&temp.path().join("job-terminal-attention-controller.db")).unwrap(),
    );
    let controller = JobTerminalContinuationController::new(db.clone());
    let registry = Arc::new(
        crate::job_receipts::production_registry_with_terminal_attention(
            db.clone(),
            controller.clone(),
        )
        .await,
    );
    let runtime = ToolRuntime::new(registry, Arc::new(RuntimeInfo::default()))
        .with_job_terminal_attention(db.clone(), controller.clone());
    (temp, runtime, db, controller)
}

fn app_binding(byte: u8) -> String {
    format!(
        "wc_host_binding_{}",
        webcodex_core::compact::encode([byte; 16])
    )
}

async fn start_owned_job(
    runtime: &ToolRuntime,
    client_id: &str,
    project_id: &str,
    auth: &crate::auth::AuthContext,
) -> (String, RunnerRequest) {
    let caps = RunnerCapabilities {
        async_jobs: true,
        async_shell_jobs: true,
        ..Default::default()
    };
    register_agent_projects_for_auth(
        runtime,
        client_id,
        auth,
        caps,
        vec![registered_project(
            project_id,
            &format!("/tmp/{project_id}"),
        )],
    )
    .await;
    let started = runtime
        .dispatch_with_auth(
            ToolCall::RunJob {
                project: format!("agent:{client_id}:{project_id}"),
                command: format!("echo {client_id}"),
                session_id: None,
                timeout_secs: Some(60),
                cwd: None,
                purpose: None,
                shell: None,
            },
            Some(auth),
        )
        .await;
    assert!(started.success, "{:?}", started.error);
    let job_id = started.output["job_id"].as_str().unwrap().to_string();
    let request = wait_for_runner_request_for_client(runtime, client_id).await;
    assert_eq!(request.job_id.as_deref(), Some(job_id.as_str()));
    (job_id, request)
}

async fn complete_job(
    runtime: &ToolRuntime,
    client_id: &str,
    request: &RunnerRequest,
    update_seq: Option<u64>,
) {
    runtime
        .runner_registry
        .update_job(RunnerJobUpdateRequest {
            client_id: client_id.to_string(),
            runner_instance_id: format!("inst-{client_id}"),
            update_seq,
            job_id: request.job_id.clone().expect("Job request id"),
            request_id: Some(request.request_id.clone()),
            status: "completed".to_string(),
            stdout_chunk: Some("done\n".to_string()),
            stderr_chunk: None,
            log_snapshot: None,
            exit_code: Some(0),
            duration_ms: Some(25),
            error: None,
            command_execution_state: None,
            validation_progress: None,
            test_count_evidence: None,
            format_mutation: None,
            activity: None,
            finished: true,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn active_job_arms_then_terminalizes_durably_without_observe_jobs_polling() {
    let (_temp, runtime, db) = attention_runtime().await;
    let hash = "a".repeat(64);
    let auth = shared_key_auth_context(&hash);
    let (job_id, request) = start_owned_job(&runtime, "e3-active", "project-a", &auth).await;

    let armed = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id: job_id.clone(),
                idempotency_key: "e3-active-arm".to_string(),
            },
            Some(&auth),
        )
        .await;
    assert!(armed.success, "{:?}", armed.error);
    assert_eq!(armed.output["job_id"], job_id);
    assert_eq!(armed.output["state"], "waiting");
    assert_eq!(armed.output["delivery_state"], "not_ready");
    assert_eq!(armed.output["automatic_resume_available"], false);
    assert_eq!(armed.output["fallback_tool"], "observe_jobs");
    for forbidden in ["stdout", "stderr", "command", "cwd", "environment"] {
        assert!(armed.output.get(forbidden).is_none(), "leaked {forbidden}");
    }

    // Canonical Runner terminal truth is sufficient: the test intentionally does
    // not call observe_jobs between the Job handoff and the durable terminal fact.
    complete_job(&runtime, "e3-active", &request, Some(1)).await;

    let principal = principal_for_auth(Some(&auth));
    let wait_id = armed.output["wait_id"].as_str().unwrap();
    let stored = db
        .read_job_terminal_wait(&principal, wait_id, chrono::Utc::now().timestamp())
        .unwrap();
    assert_eq!(stored.state, JobTerminalWaitState::Triggered);
    assert_eq!(stored.delivery_state, JobTerminalDeliveryState::Pending);
    assert_eq!(stored.terminal_status.as_deref(), Some("completed"));
    assert_eq!(stored.terminal_outcome.as_deref(), Some("succeeded"));
}

#[tokio::test]
async fn already_terminal_registration_is_immediate_and_keyed_replay_is_stable() {
    let (_temp, runtime, _db) = attention_runtime().await;
    let auth = shared_key_auth_context(&"b".repeat(64));
    let (job_id, request) = start_owned_job(&runtime, "e3-terminal", "project-b", &auth).await;
    complete_job(&runtime, "e3-terminal", &request, Some(1)).await;

    let first = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id: job_id.clone(),
                idempotency_key: "e3-after-terminal".to_string(),
            },
            Some(&auth),
        )
        .await;
    assert!(first.success, "{:?}", first.error);
    assert_eq!(first.output["state"], "triggered");
    assert_eq!(first.output["delivery_state"], "pending");
    assert_eq!(first.output["terminal_status"], "completed");
    assert_eq!(first.output["replayed"], false);

    let replay = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id,
                idempotency_key: "e3-after-terminal".to_string(),
            },
            Some(&auth),
        )
        .await;
    assert!(replay.success, "{:?}", replay.error);
    assert_eq!(replay.output["wait_id"], first.output["wait_id"]);
    assert_eq!(replay.output["replayed"], true);
    assert_eq!(replay.output["automatic_resume_available"], false);
}

#[tokio::test]
async fn terminal_delivery_latency_is_read_only_and_survives_exact_replay() {
    let (_temp, runtime, db) = attention_runtime().await;
    let auth = shared_key_auth_context(&"e".repeat(64));
    let (job_id, request) =
        start_owned_job(&runtime, "e3-delayed-delivery", "project-latency", &auth).await;
    complete_job(&runtime, "e3-delayed-delivery", &request, Some(1)).await;
    let first = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id: job_id.clone(),
                idempotency_key: "e3-delayed-settlement".to_string(),
            },
            Some(&auth),
        )
        .await;
    assert!(first.success, "{:?}", first.error);
    assert_eq!(first.output["delivery_state"], "pending");
    assert!(first.output["terminal_observed_at"].is_number());
    assert!(first.output["delivery_settled_at"].is_null());
    assert!(first.output["terminal_to_delivery_settlement_seconds"].is_null());

    let wait_id = first.output["wait_id"].as_str().unwrap();
    let principal = principal_for_auth(Some(&auth));
    let observed_at = first.output["terminal_observed_at"].as_i64().unwrap();
    let settlement_at = observed_at + 2_608;
    let prepared = db
        .prepare_job_terminal_delivery(&principal, wait_id, settlement_at)
        .unwrap()
        .expect("pending wait can be prepared once");
    let finished = db
        .finish_job_terminal_delivery(
            &principal,
            wait_id,
            &prepared.attempt_id,
            true,
            settlement_at,
        )
        .unwrap();
    assert_eq!(finished.delivery_state, JobTerminalDeliveryState::Delivered);

    let replay = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id,
                idempotency_key: "e3-delayed-settlement".to_string(),
            },
            Some(&auth),
        )
        .await;
    assert!(replay.success, "{:?}", replay.error);
    assert_eq!(replay.output["replayed"], true);
    assert_eq!(replay.output["state_changed"], false);
    assert_eq!(replay.output["wait_id"], wait_id);
    assert_eq!(replay.output["delivery_state"], "delivered");
    assert_eq!(replay.output["terminal_observed_at"], observed_at);
    assert_eq!(replay.output["delivery_settled_at"], settlement_at);
    assert_eq!(
        replay.output["terminal_to_delivery_settlement_seconds"],
        2_608
    );
    assert!(replay.output.get("new_model_turn_confirmed").is_none());
}

#[tokio::test]
async fn unauthorized_registration_is_existence_hiding_and_session_window_are_not_authority() {
    let (_temp, runtime, _db) = attention_runtime().await;
    let auth_a = shared_key_auth_context(&"c".repeat(64));
    let auth_b = shared_key_auth_context(&"d".repeat(64));
    let (job_id, _request) = start_owned_job(&runtime, "e3-owner", "project-c", &auth_a).await;

    let denied = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id: job_id.clone(),
                idempotency_key: "foreign-arm".to_string(),
            },
            Some(&auth_b),
        )
        .await;
    let missing = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id: "missing-e3-job".to_string(),
                idempotency_key: "missing-arm".to_string(),
            },
            Some(&auth_b),
        )
        .await;
    assert!(!denied.success);
    assert!(!missing.success);
    assert_eq!(denied.output["error_kind"], "unknown_job");
    assert_eq!(missing.output["error_kind"], "unknown_job");

    // Session/window/observation fields do not exist in the E3 registration
    // contract and therefore cannot become visibility or execution authority.
    let spec = crate::tool_runtime::registered_tool_specs()
        .into_iter()
        .find(|spec| spec.name == "wait_for_job_terminal")
        .unwrap();
    assert_eq!(spec.input_schema["additionalProperties"], false);
    for field in ["job_id", "idempotency_key"] {
        assert_eq!(spec.input_schema["properties"][field]["minLength"], 1);
        assert_eq!(spec.input_schema["properties"][field]["maxLength"], 128);
    }
    for absent in ["session_id", "client_window", "after_observation_token"] {
        assert!(spec.input_schema["properties"].get(absent).is_none());
    }
    let old_token_error = ToolCall::from_tool_name(
        "wait_for_job_terminal",
        json!({
            "job_id": job_id.clone(),
            "idempotency_key": "old-token-must-not-participate",
            "after_observation_token": "obsolete-process-local-cursor"
        }),
    )
    .expect_err("obsolete observation cursors must fail closed instead of being ignored");
    assert!(
        old_token_error.contains("unknown field `after_observation_token`"),
        "{old_token_error}"
    );
}

#[tokio::test]
async fn keyed_replay_reports_current_exact_carrier_capability_only_for_its_wait() {
    let (_temp, runtime, _db, controller) = attention_runtime_with_controller().await;
    let runtime = runtime.with_model_workflow_policy(
        crate::model_workflow::ModelWorkflowPolicy::from_values(None, Some("unattended")).unwrap(),
    );
    let auth = shared_key_auth_context(&"7".repeat(64));
    let (job_id, _request) =
        start_owned_job(&runtime, "e3-carrier", "project-carrier", &auth).await;
    let armed = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id: job_id.clone(),
                idempotency_key: "exact-carrier-replay".to_string(),
            },
            Some(&auth),
        )
        .await;
    assert!(armed.success, "{:?}", armed.error);
    assert_eq!(armed.output["automatic_resume_available"], false);
    let wait_id = armed.output["wait_id"].as_str().unwrap().to_string();
    let principal = principal_for_auth(Some(&auth));
    let binding_id = app_binding(21);
    controller
        .bind_mcp_app(
            &principal,
            &wait_id,
            binding_id.clone(),
            Some("window-replay"),
            chrono::Utc::now().timestamp(),
        )
        .unwrap();

    let replay = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id: job_id.clone(),
                idempotency_key: "exact-carrier-replay".to_string(),
            },
            Some(&auth),
        )
        .await;
    assert!(replay.success, "{:?}", replay.error);
    assert_eq!(replay.output["wait_id"], wait_id);
    assert_eq!(replay.output["replayed"], true);
    assert_eq!(replay.output["automatic_resume_available"], true);

    let (other_job_id, _other_request) =
        start_owned_job(&runtime, "e3-carrier-other", "project-carrier-other", &auth).await;
    let other = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id: other_job_id,
                idempotency_key: "unrelated-carrier-wait".to_string(),
            },
            Some(&auth),
        )
        .await;
    assert!(other.success, "{:?}", other.error);
    assert_eq!(other.output["automatic_resume_available"], false);

    controller
        .unbind_mcp_app(
            &principal,
            &wait_id,
            &binding_id,
            Some("window-replay"),
            chrono::Utc::now().timestamp(),
        )
        .unwrap();
    let after_unbind = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id,
                idempotency_key: "exact-carrier-replay".to_string(),
            },
            Some(&auth),
        )
        .await;
    assert!(after_unbind.success, "{:?}", after_unbind.error);
    assert_eq!(after_unbind.output["automatic_resume_available"], false);
}

#[tokio::test]
async fn presentation_reauthorizes_exact_wait_and_exposes_no_ambient_authority_selectors() {
    let (_temp, runtime, _db) = attention_runtime().await;
    let owner = shared_key_auth_context(&"8".repeat(64));
    let foreign = shared_key_auth_context(&"9".repeat(64));
    let (job_id, _request) =
        start_owned_job(&runtime, "e3-present", "project-present", &owner).await;
    let armed = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id: job_id.clone(),
                idempotency_key: "presentation-wait".to_string(),
            },
            Some(&owner),
        )
        .await;
    let wait_id = armed.output["wait_id"].as_str().unwrap().to_string();

    let presented = runtime
        .present_job_terminal_continuation(Some(&owner), wait_id.clone())
        .await;
    assert!(presented.success, "{:?}", presented.error);
    assert_eq!(
        presented.output["job_terminal_continuation"]["wait_id"],
        wait_id
    );
    assert_eq!(
        presented.output["job_terminal_continuation"]["job_id"],
        job_id
    );
    assert_eq!(
        presented.output["job_terminal_continuation"]["automatic_resume_available"],
        false
    );

    let denied = runtime
        .present_job_terminal_continuation(Some(&foreign), wait_id.clone())
        .await;
    assert!(!denied.success);
    assert_eq!(denied.output["error_kind"], "job_terminal_wait_not_found");

    assert!(crate::tool_runtime::registered_tool_specs()
        .iter()
        .any(|spec| spec.name == "present_job_terminal_continuation"));
    let schema =
        webcodex_tool_contracts::input_schema_for_tool("present_job_terminal_continuation");
    assert_eq!(schema["required"], json!(["wait_id"]));
    for forbidden in [
        "job_id",
        "session_id",
        "client_window",
        "peer_id",
        "principal_digest",
        "project",
    ] {
        assert!(schema["properties"].get(forbidden).is_none());
    }
}

#[tokio::test]
async fn app_binding_uses_hashed_host_sideband_only_and_never_returns_raw_identity_or_fence() {
    let (_temp, runtime, db, _controller) = attention_runtime_with_controller().await;
    let runtime = runtime.with_model_workflow_policy(
        crate::model_workflow::ModelWorkflowPolicy::from_values(None, Some("unattended")).unwrap(),
    );
    let auth = shared_key_auth_context(&"6".repeat(64));
    let (job_id, _request) =
        start_owned_job(&runtime, "e3-sideband", "project-sideband", &auth).await;
    let armed = runtime
        .dispatch_with_auth(
            ToolCall::WaitForJobTerminal {
                job_id,
                idempotency_key: "host-sideband-wait".to_string(),
            },
            Some(&auth),
        )
        .await;
    assert!(armed.success, "{:?}", armed.error);
    let wait_id = armed.output["wait_id"].as_str().unwrap().to_string();
    let raw_host_session = "PRIVATE_CHATGPT_HOST_SESSION_12345";
    let params = json!({
        "name": "bind_job_terminal_continuation",
        "arguments": {},
        "_meta": {"openai/session": raw_host_session}
    });
    let window = crate::client_window::stateless_mcp_window(&params);
    let canonical = window.identity.as_ref().expect("valid Host sideband");
    assert_ne!(canonical.key(), raw_host_session);
    assert!(!canonical.key().contains(raw_host_session));

    let binding_id = app_binding(22);
    let bound = runtime
        .job_terminal_continuation_bind_for_window(
            Some(&auth),
            Some(canonical),
            wait_id.clone(),
            binding_id.clone(),
        )
        .await;
    assert!(bound.success, "{:?}", bound.error);
    let serialized = serde_json::to_string(&bound).unwrap();
    assert!(!serialized.contains(raw_host_session));
    assert!(!serialized.contains(&binding_id));
    assert_eq!(bound.output["host_binding"]["bound"], true);
    assert_eq!(
        bound.output["job_terminal_continuation"]["automatic_resume_available"],
        true
    );

    let durable = db
        .read_job_terminal_wait(
            &principal_for_auth(Some(&auth)),
            &wait_id,
            chrono::Utc::now().timestamp(),
        )
        .unwrap();
    let durable_json = serde_json::to_string(&durable).unwrap();
    assert!(!durable_json.contains(raw_host_session));
    assert!(!durable_json.contains(&binding_id));

    let missing = runtime
        .job_terminal_continuation_bind_for_window(Some(&auth), None, wait_id, app_binding(23))
        .await;
    assert!(!missing.success);
    assert_eq!(
        missing.output["error_kind"],
        "job_terminal_client_window_unavailable"
    );
}

#[tokio::test]
async fn retained_terminal_receipt_reconciles_waiting_registration_after_store_reopen() {
    let (temp, runtime, db) = attention_runtime().await;
    let db_path = temp.path().join("job-terminal-attention.db");
    let auth = shared_key_auth_context(&"f".repeat(64));
    let (job_id, request) = start_owned_job(&runtime, "e3-restart", "project-restart", &auth).await;
    complete_job(&runtime, "e3-restart", &request, Some(1)).await;

    let access = crate::runner_http::runner_access_from_auth(Some(&auth));
    let snapshot = runtime
        .runner_registry
        .job_terminal_registration_snapshot_for_auth(access.as_ref(), &job_id)
        .await
        .unwrap();
    let source = crate::job_terminal_attention::source_from_snapshot(&snapshot);
    let principal = principal_for_auth(Some(&auth));
    let now = chrono::Utc::now().timestamp();
    let waiting = db
        .create_job_terminal_wait(
            &principal,
            webcodex_store::NewJobTerminalWait {
                source,
                idempotency_key: "e3-restart-wait".to_string(),
                expires_at: snapshot.wait_expires_at,
                already_terminal: None,
            },
            now,
        )
        .unwrap();
    assert_eq!(waiting.wait.state, JobTerminalWaitState::Waiting);
    let wait_id = waiting.wait.wait_id.clone();

    drop(runtime);
    drop(db);
    let reopened = Arc::new(crate::Database::open(&db_path).unwrap());
    reopened
        .recover_job_terminal_deliveries_after_restart(chrono::Utc::now().timestamp())
        .unwrap();
    let controller = JobTerminalContinuationController::new(reopened.clone());
    let _registry = crate::job_receipts::production_registry_with_terminal_attention(
        reopened.clone(),
        controller,
    )
    .await;

    let restored = reopened
        .read_job_terminal_wait(&principal, &wait_id, chrono::Utc::now().timestamp())
        .unwrap();
    assert_eq!(restored.state, JobTerminalWaitState::Triggered);
    assert_eq!(restored.delivery_state, JobTerminalDeliveryState::Pending);
    assert_eq!(restored.terminal_status.as_deref(), Some("completed"));
    assert_eq!(restored.terminal_outcome.as_deref(), Some("succeeded"));
}
#[tokio::test]
async fn registration_concurrent_with_terminalization_has_exactly_one_triggered_result() {
    let (_temp, runtime, db) = attention_runtime().await;
    let hook = super::super::job_terminal_wait::JobTerminalRegistrationTestHook::new();
    let first_snapshot = hook.first_snapshot.clone();
    let resume = hook.resume_after_terminal.clone();
    let runtime = Arc::new(runtime.with_job_terminal_registration_test_hook(hook));
    let auth = shared_key_auth_context(&"e".repeat(64));
    let (job_id, request) = start_owned_job(&runtime, "e3-race", "project-race", &auth).await;

    let runtime_for_arm = runtime.clone();
    let auth_for_arm = auth.clone();
    let job_for_arm = job_id.clone();
    let arm = tokio::spawn(async move {
        runtime_for_arm
            .dispatch_with_auth(
                ToolCall::WaitForJobTerminal {
                    job_id: job_for_arm,
                    idempotency_key: "e3-race-arm".to_string(),
                },
                Some(&auth_for_arm),
            )
            .await
    });

    first_snapshot.wait().await;
    complete_job(&runtime, "e3-race", &request, Some(1)).await;
    resume.wait().await;

    let armed = arm.await.unwrap();
    assert!(armed.success, "{:?}", armed.error);
    assert_eq!(armed.output["state"], "triggered");
    assert_eq!(armed.output["delivery_state"], "pending");

    let principal = principal_for_auth(Some(&auth));
    let wait_id = armed.output["wait_id"].as_str().unwrap();
    let stored = db
        .read_job_terminal_wait(&principal, wait_id, chrono::Utc::now().timestamp())
        .unwrap();
    assert_eq!(stored.state, JobTerminalWaitState::Triggered);
    assert_eq!(stored.delivery_state, JobTerminalDeliveryState::Pending);
}

#[tokio::test]
async fn readiness_cancellation_and_server_restart_need_no_persistent_wait_recovery() {
    use webcodex_tool_contracts::tool_call::JobReadinessMode::Any;
    let (temp, runtime, db) = attention_runtime().await;
    let auth = shared_key_auth_context(&"a".repeat(64));
    let (id, request) = start_owned_job(&runtime, "readiness-restart", "project-r", &auth).await;
    {
        let wait = runtime.wait_for_job_readiness(vec![id.clone()], Any, 45, Some(&auth));
        tokio::pin!(wait);
        assert!(futures_util::poll!(&mut wait).is_pending());
    }
    complete_job(&runtime, "readiness-restart", &request, Some(1)).await;
    let path = temp.path().join("job-terminal-attention.db");
    let conn = rusqlite::Connection::open(&path).unwrap();
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM wc_job_terminal_waits", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(rows, 0);
    drop(conn);
    drop(runtime);
    drop(db);
    // Reopen actual retained Job receipts with a fresh registry, not wait recovery.
    let db = Arc::new(crate::Database::open(&path).unwrap());
    let controller = JobTerminalContinuationController::new(db.clone());
    let registry = Arc::new(
        crate::job_receipts::production_registry_with_terminal_attention(db, controller).await,
    );
    let restarted = ToolRuntime::new(registry, Arc::new(RuntimeInfo::default()));
    let result = restarted
        .wait_for_job_readiness(vec![id], Any, 1, Some(&auth))
        .await;
    assert!(result.success, "{:?}", result.error);
    assert_eq!(result.output["wait_state"], "ready");
    assert_eq!(result.output["waited_ms"], 0);
}

#[tokio::test]
async fn readiness_activity_project_comes_only_from_exact_authorized_job_provenance() {
    let (_temp, runtime, _db) = attention_runtime().await;
    let owner = shared_key_auth_context(&"c".repeat(64));
    let other = shared_key_auth_context(&"d".repeat(64));
    let (first, first_request) = start_owned_job(&runtime, "ready-project-a", "one", &owner).await;

    assert_eq!(
        runtime
            .common_job_activity_project_for_auth(std::slice::from_ref(&first), Some(&owner))
            .await
            .as_deref(),
        Some("agent:ready-project-a:one")
    );
    complete_job(&runtime, "ready-project-a", &first_request, Some(1)).await;
    let mut correlation = super::super::window_activity::ToolCallCorrelation::default();
    let dispatched = runtime
        .dispatch_job_tool(
            ToolCall::WaitForJobReadiness {
                job_ids: vec![first.clone()],
                mode: webcodex_tool_contracts::tool_call::JobReadinessMode::Any,
                wait_secs: 1,
            },
            Some(&owner),
            None,
            &mut correlation,
            super::super::sessions::SessionTransport::Mcp,
        )
        .await;
    assert!(dispatched.success, "{:?}", dispatched.error);
    assert_eq!(dispatched.output["wait_state"], "ready");
    assert_eq!(
        correlation.resolved_project.as_deref(),
        Some("agent:ready-project-a:one")
    );
    assert!(runtime
        .common_job_activity_project_for_auth(std::slice::from_ref(&first), Some(&other))
        .await
        .is_none());

    let (second, _) = start_owned_job(&runtime, "ready-project-b", "two", &owner).await;
    assert!(
        runtime
            .common_job_activity_project_for_auth(&[first.clone(), second], Some(&owner))
            .await
            .is_none(),
        "mixed-Project wait sets must remain unanchored"
    );
    assert!(
        runtime
            .common_job_activity_project_for_auth(
                &[first, "wc_job_missing".to_string()],
                Some(&owner)
            )
            .await
            .is_none(),
        "unknown Job identity must never produce a Project anchor"
    );
}

#[tokio::test]
async fn readiness_mixed_visibility_hides_the_whole_set_even_if_visible_job_is_terminal() {
    use webcodex_tool_contracts::tool_call::JobReadinessMode::{All, Any};
    let (_temp, runtime, _db) = attention_runtime().await;
    let owner = shared_key_auth_context(&"a".repeat(64));
    let other = shared_key_auth_context(&"b".repeat(64));
    let (visible, request) = start_owned_job(&runtime, "ready-visible", "one", &owner).await;
    let (hidden, _) = start_owned_job(&runtime, "ready-hidden", "two", &other).await;
    complete_job(&runtime, "ready-visible", &request, Some(1)).await;
    for mode in [Any, All] {
        for invalid in [&hidden, "wc_job_missing", "invalid/job/id"] {
            let result = runtime
                .wait_for_job_readiness(
                    vec![visible.clone(), invalid.to_string()],
                    mode,
                    45,
                    Some(&owner),
                )
                .await;
            assert!(!result.success);
            assert!(result.output.is_null());
            assert_eq!(
                result.error.as_deref(),
                Some("Job readiness set is unavailable or identity-invalid")
            );
        }
    }
}
#[tokio::test]
async fn orca_host_intent_recovery_tool_is_owner_scoped_and_read_only() {
    let (_temp, runtime, db) = attention_runtime().await;
    let owner = shared_key_auth_context(&"c".repeat(64));
    let foreign = shared_key_auth_context(&"d".repeat(64));
    let key = "original_orca_intent_test_1234";
    let principal = principal_for_auth(Some(&owner));
    let reservation = db
        .reserve_orca_host_intent(
            &principal,
            &webcodex_store::NewOrcaHostIntent {
                idempotency_key: key.to_owned(),
                request_digest: "a".repeat(64),
                project_id: "agent:laptop-id:isolated-project".to_owned(),
                run_id: "run_original".to_owned(),
                coordinator_handle: "term_original".to_owned(),
            },
            1000,
        )
        .unwrap();
    assert!(!reservation.replayed);

    let exact = || ToolCall::ReadOrcaHostIntent {
        idempotency_key: key.to_owned(),
    };
    let read = runtime.dispatch_with_auth(exact(), Some(&owner)).await;
    assert!(read.success, "{:?}", read.error);
    assert_eq!(read.output["status"], "recorded");
    assert_eq!(read.output["state"], "reserved");
    assert_eq!(read.output["project_id"], "agent:laptop-id:isolated-project");
    assert_eq!(read.output["requires_reconciliation"], true);
    assert_eq!(read.output["host_wake_verified"], false);
    let foreign_read = runtime.dispatch_with_auth(exact(), Some(&foreign)).await;
    assert!(foreign_read.success, "{:?}", foreign_read.error);
    assert_eq!(foreign_read.output["status"], "unrecorded");
    assert_eq!(foreign_read.output["job_id"], serde_json::Value::Null);
    let unauthorized = runtime.dispatch_with_auth(exact(), None).await;
    assert!(!unauthorized.success);
    let still_original = db
        .read_orca_host_intent(&principal, key)
        .unwrap()
        .unwrap();
    assert_eq!(still_original.state, "reserved");
    assert!(still_original.job_id.is_none());
}

#[test]
fn read_orca_host_intent_accepts_only_bounded_canonical_tool_arguments() {
    use serde_json::json;
    let call = ToolCall::from_tool_name(
        "read_orca_host_intent",
        json!({"idempotency_key":"original_orca_intent_test_1234"}),
    )
    .unwrap();
    assert!(matches!(call, ToolCall::ReadOrcaHostIntent { .. }));
    assert!(ToolCall::from_tool_name(
        "read_orca_host_intent",
        json!({"idempotency_key":"original_orca_intent_test_1234","project":"injected"}),
    )
    .is_err());
}

#[tokio::test]
async fn arm_and_present_job_terminal_continuation_uses_one_authorized_original_job_and_wait() {
    let (_temp,runtime,db)=attention_runtime().await;
    let owner=shared_key_auth_context(&"e".repeat(64));
    let foreign=shared_key_auth_context(&"f".repeat(64));
    let (job_id,request)=start_owned_job(&runtime,"orca-onecall","project-orca",&owner).await;
    let keyed=||ToolCall::ArmAndPresentJobTerminalContinuation {
        job_id:job_id.clone(),
        idempotency_key:"orca_onecall_stable_arm".to_owned(),
    };
    let denied=runtime.dispatch_with_auth(keyed(),Some(&foreign)).await;
    assert!(!denied.success);
    let first=runtime.dispatch_with_auth(keyed(),Some(&owner)).await;
    assert!(first.success,"{:?}",first.error);
    assert_eq!(first.output["registration_state"],"registered");
    assert_eq!(first.output["presentation_state"],"waiting_card_presented");
    assert_eq!(first.output["state"],"waiting");
    assert_eq!(first.output["host_wake_verified"],false);
    assert_eq!(first.output["automatic_resume_available"],false);
    assert_eq!(first.output["job_terminal_continuation"]["wait_id"],first.output["wait_id"]);
    let wait_id=first.output["wait_id"].as_str().unwrap().to_owned();
    let second=runtime.dispatch_with_auth(keyed(),Some(&owner)).await;
    assert!(second.success,"{:?}",second.error);
    assert_eq!(second.output["wait_id"],wait_id);
    assert_eq!(second.output["replayed"],true);
    let principal=principal_for_auth(Some(&owner));
    complete_job(&runtime,"orca-onecall",&request,Some(1)).await;
    let durable=db.read_job_terminal_wait(&principal,&wait_id,chrono::Utc::now().timestamp()).unwrap();
    assert_eq!(durable.state,JobTerminalWaitState::Triggered);
    assert_eq!(durable.delivery_state,JobTerminalDeliveryState::Pending);
    let settled=runtime.dispatch_with_auth(keyed(),Some(&owner)).await;
    assert!(settled.success,"{:?}",settled.error);
    assert_eq!(settled.output["presentation_state"],"already_terminal_handle_now");
    assert_eq!(settled.output["terminal_status"],"completed");
    assert!(settled.output.get("job_terminal_continuation").is_none());
}
#[tokio::test]
async fn arm_and_present_job_terminal_continuation_handles_completed_job_without_card() {
    let (_temp,runtime,_db)=attention_runtime().await;
    let owner=shared_key_auth_context(&"2".repeat(64));
    let (job_id,request)=start_owned_job(&runtime,"orca-fastjob","project-fast",&owner).await;
    complete_job(&runtime,"orca-fastjob",&request,Some(1)).await;
    let result=runtime.dispatch_with_auth(ToolCall::ArmAndPresentJobTerminalContinuation{
        job_id,
        idempotency_key:"orca_fastjob_exact_once".to_owned(),
    },Some(&owner)).await;
    assert!(result.success,"{:?}",result.error);
    assert_eq!(result.output["state"],"triggered");
    assert_eq!(result.output["presentation_state"],"already_terminal_handle_now");
    assert_eq!(result.output["terminal_outcome"],"succeeded");
    assert!(result.output.get("job_terminal_continuation").is_none());
}
#[test]
fn arm_and_present_job_terminal_continuation_contract_refuses_injected_fields() {
    let valid=ToolCall::from_tool_name("arm_and_present_job_terminal_continuation",
       json!({"job_id":"wc_job_12345678","idempotency_key":"original_stable_key"})).unwrap();
    assert!(matches!(valid,ToolCall::ArmAndPresentJobTerminalContinuation{..}));
    assert!(ToolCall::from_tool_name("arm_and_present_job_terminal_continuation",
       json!({"job_id":"wc_job_12345678","idempotency_key":"original_stable_key","command":"echo unauthorized"})).is_err());
}

#[tokio::test]
async fn prepare_orca_host_worker_continuation_durably_fences_authorized_project_and_spec() {
    let (_temp,runtime,db)=attention_runtime().await;
    let mut owner=shared_key_auth_context(&"3".repeat(64));
    let foreign=shared_key_auth_context(&"4".repeat(64));
    let project="agent:orc-prep:proj-intent".to_string();
    register_agent_projects_for_auth(
        &runtime,"orc-prep",&owner,
        RunnerCapabilities::default(),
        vec![registered_project("proj-intent","/tmp/proj-intent")],
    ).await;
    let make=|spec:&str|ToolCall::PrepareOrcaHostWorkerContinuation {
        project_id:project.clone(),
        run_id:"run_prepared".to_owned(),
        coordinator_handle:"term_owned".to_owned(),
        idempotency_key:"original_worker_prepare_12345".to_owned(),
        worker_agent:"codex".to_owned(),
        worktree:"test-worktree".to_owned(),
        worker_spec:spec.to_owned(),
    };
    let denied=runtime.dispatch_with_auth(make("Read-only task"),Some(&owner)).await;
    assert!(!denied.success);
    assert_eq!(denied.output["error_kind"],"orca_host_intent_unauthorized");
    owner.scopes.push(crate::auth::SCOPE_PLUGIN_INVOKE.to_owned());
    let first=runtime.dispatch_with_auth(make("Read-only task"),Some(&owner)).await;
    assert!(first.success,"{:?} {:?}",first.error,first.output);
    assert_eq!(first.output["state"],"reserved");
    assert_eq!(first.output["worker_started"],false);
    assert_eq!(first.output["host_wake_verified"],false);
    assert_eq!(first.output["replayed"],false);
    let replay=runtime.dispatch_with_auth(make("Read-only task"),Some(&owner)).await;
    assert!(replay.success,"{:?}",replay.error);
    assert_eq!(replay.output["replayed"],true);
    let changed=runtime.dispatch_with_auth(make("Changed task"),Some(&owner)).await;
    assert!(!changed.success);
    assert_eq!(changed.output["error_kind"],"orca_host_intent_conflict");
    let principal=principal_for_auth(Some(&owner));
    let saved=db.read_orca_host_intent(
        &principal,"original_worker_prepare_12345"
    ).unwrap().unwrap();
    assert_eq!(saved.state,"reserved");
    assert_eq!(saved.project_id,project);
    assert!(saved.job_id.is_none());
    assert_ne!(saved.request_digest,"Read-only task");
    assert_eq!(saved.request_digest.len(),64);
    let dispatched=db.transition_orca_host_intent(
        &principal,"original_worker_prepare_12345",
        "reserved","worker_start_dispatched",None,None,None,None,
        chrono::Utc::now().timestamp()
    ).unwrap();
    assert_eq!(dispatched.state,"worker_start_dispatched");
    let uncertain=runtime.dispatch_with_auth(make("Read-only task"),Some(&owner)).await;
    assert!(uncertain.success,"{:?}",uncertain.error);
    assert_eq!(uncertain.output["status"],"original_host_intent_recovered");
    assert_eq!(uncertain.output["worker_started"],false);
    assert_eq!(uncertain.output["prior_effect_may_have_happened"],true);
    db.transition_orca_host_intent(
        &principal,"original_worker_prepare_12345",
        "worker_start_dispatched","worker_start_confirmed",
        Some(("ctx_original","task_original")),None,None,None,
        chrono::Utc::now().timestamp()
    ).unwrap();
    let confirmed=runtime.dispatch_with_auth(make("Read-only task"),Some(&owner)).await;
    assert!(confirmed.success,"{:?}",confirmed.error);
    assert_eq!(confirmed.output["state"],"worker_start_confirmed");
    assert_eq!(confirmed.output["worker_started"],true);
    assert_eq!(confirmed.output["prior_effect_may_have_happened"],true);
    let foreign_read=runtime.dispatch_with_auth(ToolCall::ReadOrcaHostIntent {
        idempotency_key:"original_worker_prepare_12345".to_owned(),
    },Some(&foreign)).await;
    assert!(foreign_read.success);
    assert_eq!(foreign_read.output["status"],"unrecorded");
}

#[test]
fn prepare_orca_host_worker_continuation_rejects_injected_tool_fields() {
    let args=json!({
        "project_id":"agent:orc-prep:proj-intent",
        "run_id":"run_prepared",
        "coordinator_handle":"term_owned",
        "idempotency_key":"original_worker_prepare_12345",
        "worker_agent":"codex",
        "worktree":"test-worktree",
        "worker_spec":"simple"
    });
    let parsed=ToolCall::from_tool_name(
        "prepare_orca_host_worker_continuation",args.clone()
    ).unwrap();
    assert!(matches!(parsed,ToolCall::PrepareOrcaHostWorkerContinuation{..}));
    let mut bad=args;
    bad["execute_shell"]=json!(true);
    assert!(ToolCall::from_tool_name(
        "prepare_orca_host_worker_continuation",bad
    ).is_err());
}

#[tokio::test]
async fn start_orca_worker_with_host_intent_never_dispatches_on_unbound_plugin_or_replay() {
    let (_temp,runtime,db)=attention_runtime().await;
    let mut owner=shared_key_auth_context(&"7".repeat(64));
    owner.scopes.push(crate::auth::SCOPE_PLUGIN_INVOKE.to_owned());
    let project="agent:orca-launch:project-launch".to_string();
    register_agent_projects_for_auth(&runtime,"orca-launch",&owner,
        RunnerCapabilities::default(),
        vec![registered_project("project-launch","/tmp/project-launch")],
    ).await;
    let make=||ToolCall::StartOrcaWorkerWithHostIntent{
        project_id:project.clone(),
        run_id:"run_launch".to_string(),
        coordinator_handle:"term_launch".to_string(),
        idempotency_key:"orca_worker_launch_123456".to_string(),
        worker_agent:"codex".to_string(),
        worktree:"test-worktree".to_string(),
        worker_spec:"Test only".to_string(),
        plugin_binding:"wc_pbind_NON_EXISTENT".to_string(),
    };
    let first=runtime.dispatch_with_auth(make(),Some(&owner)).await;
    assert!(!first.success);
    assert_eq!(first.output["error_kind"],"orca_host_worker_binding_invalid");
    let principal=principal_for_auth(Some(&owner));
    let original=db.read_orca_host_intent(&principal,
        "orca_worker_launch_123456").unwrap().unwrap();
    assert_eq!(original.state,"reserved");
    assert!(original.dispatch_id.is_none());
    db.transition_orca_host_intent(&principal,"orca_worker_launch_123456",
        "reserved","worker_start_dispatched",None,None,None,None,
        chrono::Utc::now().timestamp()).unwrap();
    let replay=runtime.dispatch_with_auth(make(),Some(&owner)).await;
    assert!(replay.success,"{:?}",replay.error);
    assert_eq!(replay.output["status"],"original_worker_launch_requires_reconciliation");
    assert_eq!(replay.output["state"],"worker_start_dispatched");
    assert_eq!(replay.output["watcher_job_started"],false);
    assert_eq!(db.read_orca_host_intent(&principal,
        "orca_worker_launch_123456").unwrap().unwrap().state,
        "worker_start_dispatched");
}
#[test]
fn start_orca_worker_with_host_intent_rejects_extra_command_in_schema() {
    let value=json!({
        "project_id":"agent:orca-launch:project-launch",
        "run_id":"run_launch",
        "coordinator_handle":"term_launch",
        "idempotency_key":"orca_worker_launch_123456",
        "worker_agent":"codex",
        "worktree":"test-worktree",
        "worker_spec":"Test only",
        "plugin_binding":"wc_pbind_aaaaaaaaaaaaaaaaaaaaaA"
    });
    assert!(matches!(ToolCall::from_tool_name(
        "start_orca_worker_with_host_intent",value.clone()).unwrap(),
        ToolCall::StartOrcaWorkerWithHostIntent{..}));
    let mut injected=value;
    injected["command"]=json!("echo bypass");
    assert!(ToolCall::from_tool_name("start_orca_worker_with_host_intent",
        injected).is_err());
}

#[tokio::test]
async fn start_orca_host_watcher_job_refuses_missing_detach_and_never_replays_unknown_watch() {
    let (_tmp,runtime,db)=attention_runtime().await;
    let mut owner=shared_key_auth_context(&"8".repeat(64));
    let key="original_watcher_test_123456";
    let principal=principal_for_auth(Some(&owner));
    db.reserve_orca_host_intent(&principal,&webcodex_store::NewOrcaHostIntent{
        idempotency_key:key.to_owned(),request_digest:"a".repeat(64),
        project_id:"agent:watch-runner:watch-project".to_owned(),
        run_id:"run_watch".to_owned(),coordinator_handle:"term_watch".to_owned(),
    },1000).unwrap();
    db.transition_orca_host_intent(&principal,key,
        "reserved","worker_start_dispatched",None,None,None,None,1001).unwrap();
    db.transition_orca_host_intent(&principal,key,
        "worker_start_dispatched","worker_start_confirmed",
        Some(("ctx_watch","task_watch")),None,None,None,1002).unwrap();
    let missing=runtime.start_orca_host_watcher_job(key.to_owned(),Some(&owner)).await;
    assert!(!missing.success);
    assert_eq!(missing.output["error_kind"],"orca_host_watcher_unauthorized");
    let unchanged=db.read_orca_host_intent(&principal,key).unwrap().unwrap();
    assert_eq!(unchanged.state,"worker_start_confirmed");
    owner.scopes.push(crate::auth::SCOPE_JOB_DETACH.to_owned());
    let unowned_project=runtime.start_orca_host_watcher_job(key.to_owned(),Some(&owner)).await;
    assert!(!unowned_project.success);
    assert_eq!(unowned_project.output["error_kind"],"orca_host_watcher_project_unauthorized");
    assert_eq!(db.read_orca_host_intent(&principal,key).unwrap().unwrap().state,
       "worker_start_confirmed");
    db.transition_orca_host_intent(&principal,key,"worker_start_confirmed",
       "watch_start_dispatched",None,None,None,None,1003).unwrap();
    let uncertain=runtime.start_orca_host_watcher_job(key.to_owned(),Some(&owner)).await;
    assert!(!uncertain.success);
    assert_eq!(uncertain.output["error_kind"],"orca_host_watcher_phase_conflict");
    assert_eq!(db.read_orca_host_intent(&principal,key).unwrap().unwrap().state,
       "watch_start_dispatched");
}
#[test]
fn start_orca_host_watcher_job_parser_forbids_external_code_and_path() {
    let value=json!({"idempotency_key":"original_watcher_test_123456"});
    assert!(matches!(ToolCall::from_tool_name("start_orca_host_watcher_job",value.clone()).unwrap(),
       ToolCall::StartOrcaHostWatcherJob{..}));
    let mut injected=value;
    injected["script"]="C:\\untrusted\\watch.js".into();
    assert!(ToolCall::from_tool_name("start_orca_host_watcher_job",injected).is_err());
}

#[tokio::test]
async fn register_orca_host_wait_and_present_uses_exact_prior_job_and_no_second_wait() {
    let (_temp,runtime,db)=attention_runtime().await;
    let owner=shared_key_auth_context(&"9".repeat(64));
    let foreign=shared_key_auth_context(&"a".repeat(64));
    let key="original_orca_wait_test_12345";
    let principal=principal_for_auth(Some(&owner));
    let (job_id,request)=start_owned_job(
        &runtime,"host-wait-project","orch-project",&owner).await;
    db.reserve_orca_host_intent(&principal,&webcodex_store::NewOrcaHostIntent{
        idempotency_key:key.to_owned(),request_digest:"b".repeat(64),
        project_id:"agent:host-wait-project:orch-project".to_owned(),
        run_id:"run_original".to_owned(),
        coordinator_handle:"term_original".to_owned(),
    },1000).unwrap();
    db.transition_orca_host_intent(&principal,key,"reserved",
        "worker_start_dispatched",None,None,None,None,1001).unwrap();
    db.transition_orca_host_intent(&principal,key,"worker_start_dispatched",
        "worker_start_confirmed",Some(("ctx_original","task_original")),
        None,None,None,1002).unwrap();
    db.transition_orca_host_intent(&principal,key,"worker_start_confirmed",
        "watch_start_dispatched",None,None,None,None,1003).unwrap();
    let (watch,_,_)=super::super::orca_host_watcher_job::first_watch_identity(
        "run_original","ctx_original","task_original");
    db.transition_orca_host_intent(&principal,key,"watch_start_dispatched",
        "watch_job_confirmed",None,Some(&watch),Some(&job_id),
        None,1004).unwrap();
    let foreign_read=runtime.register_orca_host_wait_and_present(
        key.to_owned(),Some(&foreign)).await;
    assert!(!foreign_read.success);
    assert_eq!(foreign_read.output["error_kind"],"orca_host_wait_intent_unknown");
    let first=runtime.register_orca_host_wait_and_present(
        key.to_owned(),Some(&owner)).await;
    assert!(first.success,"{:?} {:?}",first.error,first.output);
    assert_eq!(first.output["state"],"registered");
    assert_eq!(first.output["job_id"],job_id);
    assert_eq!(first.output["host_wake_verified"],false);
    assert_eq!(first.output["automatic_resume_available"],false);
    let saved=db.read_orca_host_intent(&principal,key).unwrap().unwrap();
    assert_eq!(saved.state,"registered");
    assert_eq!(saved.wait_id.as_deref(),first.output["wait_id"].as_str());
    let replay=runtime.register_orca_host_wait_and_present(
        key.to_owned(),Some(&owner)).await;
    assert!(replay.success,"{:?}",replay.error);
    assert_eq!(replay.output["status"],"original_host_wait_recovered");
    assert_eq!(replay.output["wait_id"],first.output["wait_id"]);
    assert_eq!(replay.output["automatic_resume_available"],false);
    complete_job(&runtime,"host-wait-project",&request,Some(1)).await;
    let terminal=db.read_job_terminal_wait(
        &principal,first.output["wait_id"].as_str().unwrap(),
        chrono::Utc::now().timestamp()).unwrap();
    assert_eq!(terminal.state,JobTerminalWaitState::Triggered);
}
#[test]
fn register_orca_host_wait_and_present_rejects_injected_job_selector() {
    let good=json!({"idempotency_key":"original_orca_wait_test_12345"});
    assert!(matches!(ToolCall::from_tool_name("register_orca_host_wait_and_present",good.clone()).unwrap(),
        ToolCall::RegisterOrcaHostWaitAndPresent{..}));
    let mut bad=good;
    bad["job_id"]=json!("wc_job_foreign");
    assert!(ToolCall::from_tool_name("register_orca_host_wait_and_present",bad).is_err());
}

#[tokio::test]
async fn start_orca_worker_with_continuation_preflights_detached_authority_before_any_effect(){
    let (_tmp,runtime,db)=attention_runtime().await;
    let mut owner=shared_key_auth_context(&"a".repeat(64));
    owner.scopes.push(crate::auth::SCOPE_PLUGIN_INVOKE.to_owned());
    let key="orca_full_workflow_preflight_123";
    let make=||ToolCall::StartOrcaWorkerWithContinuation{
        project_id:"agent:missing-runner:missing-project".to_string(),
        run_id:"run_full".to_string(),
        coordinator_handle:"term_full".to_string(),
        idempotency_key:key.to_string(),
        worker_agent:"codex".to_string(),
        worktree:"test-worktree".to_string(),
        worker_spec:"Test only".to_string(),
        plugin_binding:"wc_pbind_aaaaaaaaaaaaaaaaaaaaaA".to_string(),
    };
    let missing=runtime.dispatch_with_auth(make(),Some(&owner)).await;
    assert!(!missing.success);
    let principal=principal_for_auth(Some(&owner));
    assert!(db.read_orca_host_intent(&principal,key).unwrap().is_none());
    owner.scopes.push(crate::auth::SCOPE_JOB_DETACH.to_owned());
    let unowned=runtime.dispatch_with_auth(make(),Some(&owner)).await;
    assert!(!unowned.success);
    assert!(db.read_orca_host_intent(&principal,key).unwrap().is_none());
}
#[test]
fn start_orca_worker_with_continuation_rejects_extra_arbitrary_executable(){
    let args=json!({
        "project_id":"agent:runner:project","run_id":"run_demo",
        "coordinator_handle":"term_owned",
        "idempotency_key":"orca_full_workflow_preflight_123",
        "worker_agent":"codex","worktree":"worktree","worker_spec":"Test only",
        "plugin_binding":"wc_pbind_aaaaaaaaaaaaaaaaaaaaaA"
    });
    assert!(matches!(ToolCall::from_tool_name(
        "start_orca_worker_with_continuation",args.clone()).unwrap(),
        ToolCall::StartOrcaWorkerWithContinuation{..}));
    let mut bad=args;
    bad["executable"]=json!("powershell");
    assert!(ToolCall::from_tool_name("start_orca_worker_with_continuation",bad).is_err());
}
#[tokio::test]
async fn register_orca_host_wait_and_present_reports_already_terminal_without_card() {
    let (_temp,runtime,db)=attention_runtime().await;
    let owner=shared_key_auth_context(&"b".repeat(64));
    let key="original_orca_wait_terminal_race";
    let principal=principal_for_auth(Some(&owner));
    let (job_id,request)=start_owned_job(
        &runtime,"host-wait-fast","orch-fast",&owner).await;
    // Worker finished before Host was able to arm the terminal attention.
    complete_job(&runtime,"host-wait-fast",&request,Some(1)).await;
    db.reserve_orca_host_intent(&principal,&webcodex_store::NewOrcaHostIntent{
        idempotency_key:key.to_owned(),request_digest:"b".repeat(64),
        project_id:"agent:host-wait-fast:orch-fast".to_owned(),
        run_id:"run_fast".to_owned(),
        coordinator_handle:"term_fast".to_owned(),
    },1000).unwrap();
    db.transition_orca_host_intent(&principal,key,"reserved",
        "worker_start_dispatched",None,None,None,None,1001).unwrap();
    db.transition_orca_host_intent(&principal,key,"worker_start_dispatched",
        "worker_start_confirmed",Some(("ctx_fast","task_fast")),None,None,None,1002).unwrap();
    db.transition_orca_host_intent(&principal,key,"worker_start_confirmed",
        "watch_start_dispatched",None,None,None,None,1003).unwrap();
    let (watch,_,_)=super::super::orca_host_watcher_job::first_watch_identity(
        "run_fast","ctx_fast","task_fast");
    db.transition_orca_host_intent(&principal,key,"watch_start_dispatched",
        "watch_job_confirmed",None,Some(&watch),Some(&job_id),
        None,1004).unwrap();
    let result=runtime.register_orca_host_wait_and_present(
        key.to_owned(),Some(&owner)).await;
    assert!(result.success,"{:?} {:?}",result.error,result.output);
    assert_eq!(result.output["presentation_state"],"already_terminal_handle_now");
    assert_eq!(result.output["wait_state"],"triggered");
    assert_eq!(result.output["automatic_resume_available"],false);
    assert_eq!(result.output["host_wake_verified"],false);
    assert!(result.output.get("job_terminal_continuation").is_none());
    let saved=db.read_orca_host_intent(&principal,key).unwrap().unwrap();
    assert_eq!(saved.state,"registered");
    let wait_id=result.output["wait_id"].as_str().unwrap();
    let wait=db.read_job_terminal_wait(
        &principal,wait_id,chrono::Utc::now().timestamp()).unwrap();
    assert_eq!(wait.state,JobTerminalWaitState::Triggered);
    let replay=runtime.register_orca_host_wait_and_present(
        key.to_owned(),Some(&owner)).await;
    assert!(replay.success);
    assert_eq!(replay.output["wait_id"],wait_id);
}
