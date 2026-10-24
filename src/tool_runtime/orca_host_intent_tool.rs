//! Principal-scoped, read-only recovery for Host-owned Orca continuation intents.
use super::{ToolResult, ToolRuntime};
use crate::auth::AuthContext;
use crate::job_terminal_attention::principal_for_auth;
use serde_json::json;

impl ToolRuntime {
    pub(crate) fn read_orca_host_intent_for_auth(
        &self,
        idempotency_key: &str,
        auth: Option<&AuthContext>,
    ) -> ToolResult {
        let Some(_auth) = auth else {
            return ToolResult::err_with_output(
                "Authenticated Host owner required",
                json!({"error_kind":"orca_host_intent_unauthorized"}),
            );
        };
        let principal = principal_for_auth(auth);
        // The internal/anonymous defaults never form a caller-owned Host intent.
        if matches!(
            principal.kind.as_str(),
            "internal" | "bootstrap" | "managed_unowned" | "open_anonymous"
        ) {
            return ToolResult::err_with_output(
                "A managed, authorized Host owner is required",
                json!({"error_kind":"orca_host_intent_unauthorized"}),
            );
        }
        let Some(db) = self.job_terminal_db.as_ref() else {
            return ToolResult::err_with_output(
                "Host intent Store is unavailable",
                json!({"error_kind":"orca_host_intent_store_unavailable"}),
            );
        };
        if idempotency_key.len() < 8
            || idempotency_key.len() > 128
            || !idempotency_key.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
            })
        {
            return ToolResult::err_with_output(
                "Invalid exact Host intent key",
                json!({"error_kind":"orca_host_intent_invalid"}),
            );
        }
        match db.read_orca_host_intent(&principal, idempotency_key) {
            Ok(Some(record)) => ToolResult::ok(json!({
                "status":"recorded",
                "state":record.state,
                "project_id":record.project_id,
                "run_id":record.run_id,
                "coordinator_handle":record.coordinator_handle,
                "dispatch_id":record.dispatch_id,
                "task_id":record.task_id,
                "watch_key":record.watch_key,
                "job_id":record.job_id,
                "wait_id":record.wait_id,
                "requires_reconciliation":true,
                "host_wake_verified":false,
                "guidance":"Reconcile original Orca Worker, Runner Job and Host Wait. This caller-owned record does not grant new mutation or ChatGPT continuation authority.",
            })),
            Ok(None) => ToolResult::ok(json!({
                "status":"unrecorded",
                "state":null,
                "project_id":null,
                "run_id":null,
                "coordinator_handle":null,
                "dispatch_id":null,
                "task_id":null,
                "watch_key":null,
                "job_id":null,
                "wait_id":null,
                "requires_reconciliation":true,
                "host_wake_verified":false,
                "guidance":"Missing intent is not proof that no earlier external Worker or Job was created. Check authoritative histories before retrying.",
            })),
            Err(error) => ToolResult::err_with_output(
                "Orca Host intent read failed",
                json!({"error_kind":error.code}),
            ),
        }
    }
}

impl ToolRuntime {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn prepare_orca_host_worker_continuation(
        &self,
        project_id: String,
        run_id: String,
        coordinator_handle: String,
        idempotency_key: String,
        worker_agent: String,
        worker_spec: String,
        auth: Option<&AuthContext>,
    ) -> ToolResult {
        use crate::auth::{SCOPE_JOB_RUN, SCOPE_PLUGIN_INVOKE};
        use sha2::{Digest, Sha256};
        let Some(authority) = auth else {
            return ToolResult::err_with_output(
                "Authenticated Host owner required",
                json!({"error_kind":"orca_host_intent_unauthorized"}),
            );
        };
        let principal = principal_for_auth(auth);
        if matches!(principal.kind.as_str(), "internal" | "bootstrap" | "open_anonymous"
            | "managed_unowned") || !authority.has_scope(SCOPE_JOB_RUN)
            || !authority.has_scope(SCOPE_PLUGIN_INVOKE)
        {
            return ToolResult::err_with_output(
                "Host owner needs explicit Runner job and Plugin invoke scope",
                json!({"error_kind":"orca_host_intent_unauthorized"}),
            );
        }
        // This is a durable intent for a future *authorized* Worker action;
        // no new execution permission is granted by a reserved record.
        if !matches!(worker_agent.as_str(),
            "codex" | "claude" | "claudecode" | "agy" | "dsh" | "antigravity")
            || worker_spec.is_empty() || worker_spec.len()>4096
            || worker_spec.chars().any(|c| c == '\0')
            || !project_id.starts_with("agent:") || project_id.len()>256
        {
            return ToolResult::err_with_output(
                "Bounded exact Worker intent required",
                json!({"error_kind":"orca_host_intent_invalid"}),
            );
        }
        let resolved = match self.resolve_project_input_for_auth(&project_id, auth).await {
            Ok(project) if project.resolved_id == project_id => project,
            _ => return ToolResult::err_with_output(
                "Exact caller-authorized Runner Project could not be verified",
                json!({"error_kind":"orca_host_intent_project_unauthorized"}),
            ),
        };
        let access = crate::runner_http::runner_access_from_auth(auth);
        if self.runner_registry.assert_runner_access(
            access.as_ref(), &resolved.config.client_id).await.is_err() {
            return ToolResult::err_with_output(
                "Runner ownership could not be proven",
                json!({"error_kind":"orca_host_intent_project_unauthorized"}),
            );
        }
        let Some(db) = self.job_terminal_db.as_ref() else {
            return ToolResult::err_with_output(
                "Durable Host intent Store unavailable",
                json!({"error_kind":"orca_host_intent_store_unavailable"}),
            );
        };
        let mut digest=Sha256::new();
        digest.update(b"webcodex.orca-host-worker-intent.v1");
        for part in [
            project_id.as_str(), run_id.as_str(),
            coordinator_handle.as_str(), worker_agent.as_str(),
            worker_spec.as_str(),
        ] {
            digest.update((part.len() as u64).to_be_bytes());
            digest.update(part.as_bytes());
        }
        let spec_digest=format!("{:x}",digest.finalize());
        let request=webcodex_store::NewOrcaHostIntent {
            idempotency_key,
            request_digest:spec_digest,
            project_id,
            run_id,
            coordinator_handle,
        };
        match db.reserve_orca_host_intent(
            &principal,&request,chrono::Utc::now().timestamp()
        ) {
            Ok(mutation)=>{
                let stage=mutation.intent.state.as_str();
                let worker_started=matches!(stage,
                    "worker_start_confirmed"|"watch_start_dispatched"|
                    "watch_job_confirmed"|"wait_registration_dispatched"|
                    "wait_registered"|"registered");
                let prior_effect_may_have_happened=stage!="reserved";
                ToolResult::ok(json!({
                "status":if mutation.replayed {
                    "original_host_intent_recovered"
                } else {
                    "original_host_intent_reserved"
                },
                "state":mutation.intent.state,
                "replayed":mutation.replayed,
                "project_id":mutation.intent.project_id,
                "run_id":mutation.intent.run_id,
                "coordinator_handle":mutation.intent.coordinator_handle,
                "idempotency_key":mutation.intent.idempotency_key,
                "worker_started":worker_started,
                "prior_effect_may_have_happened":prior_effect_may_have_happened,
                "host_wake_verified":false,
                "guidance":if mutation.replayed {
                    "Prior Host intent was recovered. This replay starts no new Worker, Job or Wait. Inspect exact original state and independently reconcile uncertain effects."
                } else {
                    "Durable Host intent reserved. No Worker, Job or Wait started by this call; subsequent effects require Host ownership and stage transition."
                },
                }))
            },
            Err(error)=>ToolResult::err_with_output(
                "Original Host Worker intent reservation refused",
                json!({"error_kind":error.code}),
            ),
        }
    }
}
