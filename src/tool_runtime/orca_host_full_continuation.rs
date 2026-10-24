//! Single Host call for the first authorized Orca Worker → watcher Job → Wait.
//! Each subordinate stage has a separate SQLite CAS before any effect.
//! This method never retries a dispatched/uncertain external action.
use super::{ToolResult,ToolRuntime};
use crate::auth::{AuthContext,SCOPE_JOB_RUN,SCOPE_PROJECT_WRITE,SCOPE_PLUGIN_INVOKE};
use crate::runner_http::RunnerFeature;
use serde_json::json;
use webcodex_core::authority::SCOPE_JOB_DETACH;

impl ToolRuntime {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn start_orca_worker_with_continuation(
        &self,
        project_id:String,run_id:String,coordinator_handle:String,
        idempotency_key:String,worker_agent:String,worktree:String,
        worker_spec:String,plugin_binding:String,
        auth:Option<&AuthContext>,
        transport:super::sessions::SessionTransport,
    )->ToolResult{
        let Some(owner)=auth else{
            return ToolResult::err_with_output("Managed Host owner required",
                json!({"error_kind":"orca_host_composite_unauthorized"}))
        };
        if ![SCOPE_JOB_RUN,SCOPE_PROJECT_WRITE,SCOPE_PLUGIN_INVOKE,SCOPE_JOB_DETACH]
            .iter().all(|scope|owner.has_scope(scope)){
            return ToolResult::err_with_output("Full Worker/Job/Plugin scope set missing",
                json!({"error_kind":"orca_host_composite_unauthorized"}))
        }
        let project=match self.resolve_project_input_for_auth(&project_id,auth).await{
            Ok(project) if project.resolved_id==project_id=>project,
            _=>return ToolResult::err_with_output("Exact Runner Project unavailable",
                json!({"error_kind":"orca_host_composite_project_unauthorized"})),
        };
        let access=crate::runner_http::runner_access_from_auth(auth);
        if self.runner_registry.assert_runner_access(
            access.as_ref(),&project.config.client_id).await.is_err(){
            return ToolResult::err_with_output("Runner owner unavailable",
                json!({"error_kind":"orca_host_composite_project_unauthorized"}))
        }
        let features=self.runner_registry
            .get_runner_feature_set(&project.config.client_id).await;
        if !matches!(features,Ok(f) if f.supports(RunnerFeature::DetachedProcessJobs)){
            return ToolResult::err_with_output(
                "Runner lacks durable detached watcher support",
                json!({"error_kind":"orca_host_composite_capability_unavailable",
                    "worker_started":false,"host_wake_verified":false}))
        }
        if !crate::plugin_gateway::is_exact_orca_worker_start_binding(
            self,&plugin_binding,&project_id,&project.config.client_id){
            return ToolResult::err_with_output(
                "Required original Project-bound Orca start binding unavailable",
                json!({"error_kind":"orca_host_composite_binding_invalid",
                    "worker_started":false,"host_wake_verified":false}))
        }
        let worker=self.start_orca_worker_with_host_intent(
            project_id.clone(),run_id,coordinator_handle,idempotency_key.clone(),
            worker_agent,worktree,worker_spec,plugin_binding,auth,transport
        ).await;
        if !worker.success ||
            worker.output["status"]!="original_orca_worker_launch_confirmed"{
            return ToolResult::err_with_output(
                "Original Orca Worker launch is unconfirmed or already dispatched",
                json!({"status":"orca_host_composite_requires_reconciliation",
                    "phase":"worker_start_dispatched_or_existing",
                    "error_kind":"orca_host_composite_worker_unconfirmed",
                    "worker_started":worker.output["worker_started"],
                    "dispatch_id":worker.output["dispatch_id"],
                    "task_id":worker.output["task_id"],
                    "host_wake_verified":false,
                    "guidance":"Inspect existing Host intent and Orca journal. Never launch a duplicate Worker."}))
        }
        let dispatch_id=worker.output["dispatch_id"].clone();
        let task_id=worker.output["task_id"].clone();
        let watcher=self.start_orca_host_watcher_job(
            idempotency_key.clone(),auth).await;
        if !watcher.success ||
            watcher.output["status"]!="original_watcher_job_confirmed"{
            return ToolResult::err_with_output(
                "Original Worker confirmed; watcher Job needs reconciliation",
                json!({"status":"orca_host_composite_requires_reconciliation",
                    "phase":"watch_start_dispatched_or_existing",
                    "error_kind":"orca_host_composite_watcher_unconfirmed",
                    "dispatch_id":dispatch_id,"task_id":task_id,
                    "job_id":watcher.output["job_id"],
                    "watcher_error_kind":watcher.output["error_kind"],
                    "worker_started":true,"host_wake_verified":false,
                    "guidance":"Retain the confirmed Worker and original watcher admission key. Do not start a second watcher."}))
        }
        let job_id=watcher.output["job_id"].clone();
        let wait=self.register_orca_host_wait_and_present(
            idempotency_key,auth).await;
        if !wait.success || wait.output["status"]!="original_host_wait_registered"{
            return ToolResult::err_with_output(
                "Original Worker and watcher Job confirmed; Host Wait needs reconciliation",
                json!({"status":"orca_host_composite_requires_reconciliation",
                    "phase":"wait_registration_dispatched_or_existing",
                    "error_kind":"orca_host_composite_wait_unconfirmed",
                    "dispatch_id":dispatch_id,"task_id":task_id,
                    "job_id":job_id,"wait_id":wait.output["wait_id"],
                    "wait_error_kind":wait.output["error_kind"],
                    "host_wake_verified":false,
                    "guidance":"Reconcile the exact original Wait. Never create replacement Worker or Job."}))
        }
        ToolResult::ok(json!({
            "status":"original_orca_host_continuation_registered",
            "phase":"registered",
            "dispatch_id":dispatch_id,"task_id":task_id,
            "job_id":job_id,"wait_id":wait.output["wait_id"],
            "presentation_state":wait.output["presentation_state"],
            "automatic_resume_available":wait.output["automatic_resume_available"],
            "continuation_capability_state":if wait.output["automatic_resume_available"]==true {
                "unattended_carrier_observed_turn_unverified"
            }else {
                "no_proven_unattended_carrier"
            },
            "host_wake_verified":false,
            "guidance":"Worker, first-party watcher and Wait registration are stored. The authorized Host offered a continuation; a later GPT turn must be verified independently."
        }))
    }
}
