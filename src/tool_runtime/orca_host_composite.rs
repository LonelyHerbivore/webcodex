//! Compose the two existing authorized Host operations in one model-facing
//! invocation for a pre-existing Runner Job. Never launches an Orca Worker,
//! a process or a new GPT turn. All authority remains on the original Host
//! job registration and presenter checks.
use super::{ToolResult, ToolRuntime};
use crate::auth::AuthContext;
use serde_json::json;

impl ToolRuntime {
    pub(crate) async fn arm_and_present_job_terminal_continuation(
        &self,
        job_id: String,
        idempotency_key: String,
        auth: Option<&AuthContext>,
    ) -> ToolResult {
        // A model invocation without an authorized AuthContext may only use
        // existing transport defaults; do not silently claim owner identity.
        let registered = self
            .wait_for_job_terminal(job_id.clone(), idempotency_key, auth)
            .await;
        if !registered.success {
            return registered;
        }
        let Some(wait_id) = registered.output["wait_id"].as_str() else {
            return ToolResult::err_with_output(
                "Host wait registration returned an unknown original identity",
                json!({"registration_state":"outcome_unknown_requires_reconciliation",
                  "error_kind":"host_wait_identity_unknown","job_id":job_id,
                  "host_wake_verified":false}),
            );
        };
        let wait_id = wait_id.to_string();
        let state = registered.output["state"].as_str().unwrap_or("unknown");
        let replayed = registered.output["replayed"].as_bool().unwrap_or(false);
        if state != "waiting" {
            return ToolResult::ok(json!({
                "job_id":job_id,
                "wait_id":wait_id,
                "state":state,
                "replayed":replayed,
                "registration_state":"registered",
                "presentation_state":"already_terminal_handle_now",
                "terminal_status":registered.output["terminal_status"],
                "terminal_outcome":registered.output["terminal_outcome"],
                "host_wake_verified":false,
                "automatic_resume_available":registered.output["automatic_resume_available"],
                "guidance":"The original wait has terminal truth. Process its exact result now; never rearm or present an already-triggered wait.",
            }));
        }
        // Existing presentation always reauthorizes the exact original Wait
        // and underlying Runner Job. In particular, the card is not a durable
        // promise of automatic wake when no active Host carrier is installed.
        let presented = self
            .present_job_terminal_continuation(auth, wait_id.clone())
            .await;
        if !presented.success {
            return ToolResult::err_with_output(
                presented.error.unwrap_or_else(|| {
                    "Original wait is registered; Host presentation outcome is not confirmed"
                        .to_string()
                }),
                json!({
                    "job_id":job_id,
                    "wait_id":wait_id,
                    "state":state,
                    "replayed":replayed,
                    "registration_state":"registered",
                    "presentation_state":"presentation_requires_reconciliation",
                    "host_wake_verified":false,
                    "error_kind":presented.output["error_kind"],
                    "guidance":"Inspect exact original Host Wait and authorized Window. Never launch a new Job or rearm on uncertain card delivery.",
                }),
            );
        }
        ToolResult::ok(json!({
            "job_id":job_id,
            "wait_id":wait_id,
            "state":state,
            "replayed":replayed,
            "registration_state":"registered",
            "presentation_state":"waiting_card_presented",
            "host_wake_verified":false,
            "automatic_resume_available":registered.output["automatic_resume_available"],
            "job_terminal_continuation":presented.output["job_terminal_continuation"],
            "guidance":"The current authorized Host card was offered. This is not proof that a later ChatGPT model turn occurred.",
        }))
    }
}
