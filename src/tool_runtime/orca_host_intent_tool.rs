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
