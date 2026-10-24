//! First-party Host-owned watcher admission for a durably confirmed Orca Worker.
//! Runner executes the embedded, reviewed Node source, never a Plugin-supplied
//! script path. Detached Job ownership is keyed and fenced before dispatch.
use super::{ExecutionPurpose, ToolResult, ToolRuntime};
use crate::auth::{AuthContext, SCOPE_JOB_RUN, SCOPE_PROJECT_WRITE};
use webcodex_core::authority::SCOPE_JOB_DETACH;
use crate::job_terminal_attention::principal_for_auth;
use crate::runner_http::RunnerFeature;
use serde_json::json;
use sha2::{Digest,Sha256};

const WATCHER_SOURCE: &str = include_str!("orca_host_watcher.mjs");

pub(super) fn first_watch_identity(run:&str,dispatch:&str,task:&str)->(String,String,String){
    let canonical=serde_json::to_string(&[
        "worker_done_host_job_v1",run,dispatch,task
    ]).expect("static strings always serialize");
    let digest=format!("{:x}",Sha256::digest(canonical.as_bytes()));
    let suffix=&digest[..32];
    (
        format!("orca_host_wd_{suffix}"),
        format!("orca_host_job_{suffix}"),
        format!("orca_host_wait_{suffix}"),
    )
}
fn valid_id(value:&str,prefix:&str)->bool{
    value.len()>prefix.len() && value.len()<=128
        && value.starts_with(prefix)
        && value[prefix.len()..].bytes()
            .all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-')
}
impl ToolRuntime {
    pub(crate) async fn start_orca_host_watcher_job(
        &self, idempotency_key:String,auth:Option<&AuthContext>,
    )->ToolResult{
        let Some(authority)=auth else {
            return ToolResult::err_with_output("Authenticated owner required",
                json!({"error_kind":"orca_host_watcher_unauthorized"}))
        };
        if ![SCOPE_JOB_RUN,SCOPE_JOB_DETACH,SCOPE_PROJECT_WRITE].iter()
            .all(|scope|authority.has_scope(scope)){
            return ToolResult::err_with_output("Host watcher Job permission missing",
                json!({"error_kind":"orca_host_watcher_unauthorized"}))
        }
        let principal=principal_for_auth(auth);
        if matches!(principal.kind.as_str(),"internal"|"bootstrap"|"managed_unowned"|"open_anonymous"){
            return ToolResult::err_with_output("Managed Host owner required",
                json!({"error_kind":"orca_host_watcher_unauthorized"}))
        }
        if !valid_id(&idempotency_key,""){
            return ToolResult::err_with_output("Exact original intent key invalid",
                json!({"error_kind":"orca_host_watcher_invalid"}))
        }
        let Some(db)=self.job_terminal_db.as_ref() else {
            return ToolResult::err_with_output("Durable Host Store unavailable",
                json!({"error_kind":"orca_host_watcher_store_unavailable"}))
        };
        let record=match db.read_orca_host_intent(&principal,&idempotency_key){
            Ok(Some(record))=>record,
            Ok(None)=>return ToolResult::err_with_output(
                "Original authorized Worker intent missing",
                json!({"error_kind":"orca_host_watcher_intent_unavailable"})),
            Err(error)=>return ToolResult::err_with_output(
                "Original Worker intent is unreadable",json!({"error_kind":error.code})),
        };
        if record.state=="watch_job_confirmed"||
            record.state=="wait_registration_dispatched"||
            record.state=="wait_registered"||record.state=="registered" {
            return ToolResult::ok(json!({
                "status":"original_watcher_job_recovered",
                "state":record.state,"watch_key":record.watch_key,
                "job_id":record.job_id,"watcher_job_started":null,
                "host_wake_verified":false,
                "guidance":"Original watcher identity recovered. Do not re-launch; reconcile original Runner Job and Wait."
            }))
        }
        if record.state!="worker_start_confirmed"{
            return ToolResult::err_with_output(
                "Original Worker/watcher phase needs reconciliation",
                json!({"error_kind":"orca_host_watcher_phase_conflict",
                    "state":record.state,"host_wake_verified":false}))
        }
        let (Some(dispatch),Some(task))=(record.dispatch_id.as_ref(),record.task_id.as_ref()) else {
            return ToolResult::err_with_output(
                "Original confirmed Dispatch/Task missing",
                json!({"error_kind":"orca_host_watcher_identity_unconfirmed"}))
        };
        if !valid_id(&record.run_id,"run_")||
           !valid_id(&record.coordinator_handle,"term_")||
           !valid_id(dispatch,"ctx_")||!valid_id(task,"task_"){
            return ToolResult::err_with_output(
                "Original Orca identity invalid",
                json!({"error_kind":"orca_host_watcher_identity_unconfirmed"}))
        }
        let project=match self.resolve_project_input_for_auth(
            &record.project_id,auth).await {
            Ok(project) if project.resolved_id==record.project_id=>project,
            _=>return ToolResult::err_with_output(
                "Runner Project authorization is stale",
                json!({"error_kind":"orca_host_watcher_project_unauthorized"})),
        };
        let access=crate::runner_http::runner_access_from_auth(auth);
        if self.runner_registry.assert_runner_access(
            access.as_ref(),&project.config.client_id).await.is_err(){
            return ToolResult::err_with_output(
                "Runner owner not authorized",
                json!({"error_kind":"orca_host_watcher_project_unauthorized"}))
        }
        let features=self.runner_registry
            .get_runner_feature_set(&project.config.client_id).await;
        if !matches!(features,Ok(features) if features.supports(RunnerFeature::DetachedProcessJobs)){
            return ToolResult::err_with_output(
                "Runner does not support durable detached Job supervision",
                json!({"error_kind":"orca_host_watcher_capability_unavailable"}))
        }
        let (watch_key,job_key,wait_key)=first_watch_identity(
            &record.run_id,dispatch,task
        );
        let params=vec![
            "--input-type=module".to_owned(),
            "--eval".to_owned(),
            WATCHER_SOURCE.to_owned(),
            "--".to_owned(),record.run_id.clone(),
            record.coordinator_handle.clone(),dispatch.clone(),task.clone(),
            watch_key.clone(),
        ];
        // The first irreversible boundary must be durable BEFORE the Runner
        // launch. A rejected/timeout/unknown process result stays dispatched.
        if let Err(error)=db.transition_orca_host_intent(
            &principal,&idempotency_key,"worker_start_confirmed",
            "watch_start_dispatched",None,None,None,None,
            chrono::Utc::now().timestamp()
        ){
            return ToolResult::err_with_output(
                "Another Host watcher transition already owns the original intent",
                json!({"error_kind":error.code,"host_wake_verified":false}))
        }
        let started=self.run_detached_process_with_contract(
            record.project_id.clone(),
            job_key,
            "node".to_owned(),params,None,Some(930),
            None,Some(ExecutionPurpose::Operation),
            None,None,auth,
        ).await;
        if !started.success {
            return ToolResult::err_with_output(
                "Original Runner watcher admission is uncertain; do not retry",
                json!({"status":"original_watcher_job_requires_reconciliation",
                    "state":"watch_start_dispatched",
                    "error_kind":"orca_host_watcher_admission_unknown",
                    "runner_failure_kind":started.output["failure_kind"],
                    "runner_job_hint":started.output["job_id"],
                    "host_wake_verified":false,
                    "guidance":"Use the exact original Runner detached Job key/Host Store history to reconcile. Never dispatch a second watcher."}),
            )
        }
        let Some(job_id)=started.output["job_id"].as_str()
            .filter(|id|valid_id(id,"wc_job_")) else{
            return ToolResult::err_with_output(
                "Runner accepted watcher without a verified durable Job identity",
                json!({"status":"original_watcher_job_requires_reconciliation",
                    "state":"watch_start_dispatched",
                    "error_kind":"orca_host_watcher_job_identity_unknown",
                    "host_wake_verified":false}),
            )
        };
        let job_id=job_id.to_owned();
        match db.transition_orca_host_intent(
            &principal,&idempotency_key,"watch_start_dispatched",
            "watch_job_confirmed",None,Some(&watch_key),Some(&job_id),None,
            chrono::Utc::now().timestamp()
        ){
            Ok(_)=>ToolResult::ok(json!({
                "status":"original_watcher_job_confirmed",
                "state":"watch_job_confirmed",
                "watch_key":watch_key,"job_id":job_id,
                "wait_idempotency_key":wait_key,
                "watcher_job_started":started.output["command_started"],
                "host_wake_verified":false,
                "guidance":"First-party Host watcher Job admitted. Register its original Host Wait; GPT turn unverified."
            })),
            Err(error)=>ToolResult::err_with_output(
                "Watcher Job admitted but Host Store confirmation failed",
                json!({"status":"original_watcher_job_requires_reconciliation",
                    "state":"watch_start_dispatched","job_id":job_id,
                    "error_kind":error.code,"host_wake_verified":false}),
            )
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_watcher_matches_plugin_first_cycle_key(){
        let (watch,job,wait)=first_watch_identity("run_demo","ctx_workerP0","task_workerP0");
        assert_eq!(watch,"orca_host_wd_8d3daefafe9ecd474d3afc837b704d42");
        assert!(job.starts_with("orca_host_job_"));
        assert_eq!(wait,watch.replacen("orca_host_wd_","orca_host_wait_",1));
        assert!(WATCHER_SOURCE.len()>1000&&WATCHER_SOURCE.len()<12000);
        assert!(!WATCHER_SOURCE.contains("eval("));
        assert!(!WATCHER_SOURCE.contains("shell:true"));
    }
}
