//! Host-owned Wait registration after the first-party watcher Job was admitted.
//! Store phases and owner provenance are checked before native Host Wait calls.
use super::{ToolResult,ToolRuntime};
use crate::auth::{AuthContext,SCOPE_JOB_RUN};
use crate::job_terminal_attention::principal_for_auth;
use serde_json::json;

impl ToolRuntime {
    pub(crate) async fn register_orca_host_wait_and_present(
        &self,idempotency_key:String,auth:Option<&AuthContext>,
    )->ToolResult{
        let Some(owner)=auth else{
            return ToolResult::err_with_output("Authenticated Host owner required",
                json!({"error_kind":"orca_host_wait_unauthorized"}))
        };
        if !owner.has_scope(SCOPE_JOB_RUN){
            return ToolResult::err_with_output("Job attention scope unavailable",
                json!({"error_kind":"orca_host_wait_unauthorized"}))
        }
        let principal=principal_for_auth(auth);
        if matches!(principal.kind.as_str(),"internal"|"bootstrap"|"managed_unowned"|"open_anonymous"){
            return ToolResult::err_with_output("Managed Host owner required",
                json!({"error_kind":"orca_host_wait_unauthorized"}))
        }
        let Some(db)=self.job_terminal_db.as_ref() else{
            return ToolResult::err_with_output("Host Store unavailable",
                json!({"error_kind":"orca_host_wait_store_unavailable"}))
        };
        let record=match db.read_orca_host_intent(&principal,&idempotency_key){
            Ok(Some(value))=>value,
            Ok(None)=>return ToolResult::err_with_output(
                "Original Host intent not found",json!({"error_kind":"orca_host_wait_intent_unknown"})),
            Err(error)=>return ToolResult::err_with_output(
                "Original Host intent unreadable",json!({"error_kind":error.code})),
        };
        let job_id=match record.job_id{
            Some(ref job_id)=>job_id.to_owned(),
            None=>return ToolResult::err_with_output(
                "Original watcher Job not confirmed",
                json!({"error_kind":"orca_host_wait_watcher_not_confirmed",
                    "state":record.state,"host_wake_verified":false})),
        };
        if record.state=="registered"{
            let unattended_available=record.wait_id.as_deref().is_some_and(|id|
                self.job_terminal_continuations.as_ref().is_some_and(|controller|
                    controller.automatic_resume_available_for_wait(
                        &principal,id,chrono::Utc::now().timestamp()
                    )
                )
            );
            return ToolResult::ok(json!({
                "status":"original_host_wait_recovered","state":"registered",
                "job_id":job_id,"wait_id":record.wait_id,
                "presentation_state":"existing_wait_reconcile_current_host",
                "automatic_resume_available":unattended_available,
                "host_wake_verified":false,
                "guidance":"Do not register another Wait. An authorized Host may inspect/present this exact original Wait separately."
            }))
        }
        if record.state!="watch_job_confirmed" && record.state!="wait_registered"{
            return ToolResult::err_with_output(
                "Original Wait registration phase requires reconciliation",
                json!({"error_kind":"orca_host_wait_phase_conflict",
                    "state":record.state,"job_id":job_id,"host_wake_verified":false}))
        }
        // The durable Job-to-Project association must be independently visible
        // to the exact caller. The saved Host intent is not a permission grant.
        let project=self.common_job_activity_project_for_auth(
            std::slice::from_ref(&job_id),auth
        ).await;
        if project.as_deref()!=Some(record.project_id.as_str()){
            return ToolResult::err_with_output(
                "Original watcher Job and Project ownership cannot be proven",
                json!({"error_kind":"orca_host_wait_job_project_mismatch",
                    "job_id":job_id,"host_wake_verified":false}))
        }
        if record.state=="watch_job_confirmed"{
            if let Err(error)=db.transition_orca_host_intent(
                &principal,&idempotency_key,"watch_job_confirmed",
                "wait_registration_dispatched",None,None,None,None,
                chrono::Utc::now().timestamp()
            ){
                return ToolResult::err_with_output(
                    "Original Host Wait claimed by another execution",
                    json!({"error_kind":error.code,"host_wake_verified":false}))
            }
            let Some(watch_key)=record.watch_key.as_deref() else{
                return ToolResult::err_with_output(
                    "Original Host watcher key absent",
                    json!({"error_kind":"orca_host_wait_identity_unknown",
                        "state":"wait_registration_dispatched",
                        "job_id":job_id,"host_wake_verified":false}))
            };
            let Some(suffix)=watch_key.strip_prefix("orca_host_wd_")
                .filter(|s|s.len()==32&&s.bytes().all(|b|b.is_ascii_hexdigit())) else {
                return ToolResult::err_with_output(
                    "Original Host watcher identity invalid",
                    json!({"error_kind":"orca_host_wait_identity_unknown",
                        "state":"wait_registration_dispatched",
                        "job_id":job_id,"host_wake_verified":false}))
            };
            let key=format!("orca_host_wait_{suffix}");
            let registered=self.wait_for_job_terminal(job_id.clone(),key,auth).await;
            if !registered.success{
                return ToolResult::err_with_output(
                    "Original Wait registration returned no confirmed receipt",
                    json!({"error_kind":"orca_host_wait_registration_unknown",
                        "state":"wait_registration_dispatched",
                        "job_id":job_id,
                        "wait_id_hint":registered.output["wait_id"],
                        "host_wake_verified":false}),
                )
            }
            let Some(wait_id)=registered.output["wait_id"].as_str()
                .filter(|v|v.starts_with("wc_job_wait_")&&v.len()<=128) else{
                return ToolResult::err_with_output(
                    "Original Wait ID missing or invalid",
                    json!({"error_kind":"orca_host_wait_identity_unknown",
                        "state":"wait_registration_dispatched",
                        "job_id":job_id,"host_wake_verified":false}))
            };
            if let Err(error)=db.transition_orca_host_intent(
                &principal,&idempotency_key,"wait_registration_dispatched",
                "wait_registered",None,None,None,Some(wait_id),
                chrono::Utc::now().timestamp()
            ){
                return ToolResult::err_with_output(
                    "Original Wait was registered but its Host confirmation is uncertain",
                    json!({"error_kind":error.code,"state":"wait_registration_dispatched",
                        "job_id":job_id,"wait_id":wait_id,"host_wake_verified":false}))
            }
        }
        let confirmed=match db.read_orca_host_intent(&principal,&idempotency_key){
            Ok(Some(v)) if v.state=="wait_registered"=>v,
            _=>return ToolResult::err_with_output(
                "Original Wait must be reconciled before presentation",
                json!({"error_kind":"orca_host_wait_store_unavailable",
                    "host_wake_verified":false})),
        };
        if let Err(error)=db.transition_orca_host_intent(
            &principal,&idempotency_key,"wait_registered","registered",
            None,None,None,None,chrono::Utc::now().timestamp()
        ){
            return ToolResult::err_with_output(
                "Original Wait confirmed but final Host phase is uncertain",
                json!({"error_kind":error.code,"wait_id":confirmed.wait_id,
                    "host_wake_verified":false}))
        }
        let Some(wait_id)=confirmed.wait_id else{
            return ToolResult::err_with_output(
                "Original Wait identity missing from Host Store",
                json!({"error_kind":"orca_host_wait_identity_unknown",
                    "state":"registered","host_wake_verified":false}))
        };
        // Read the actual per-Wait Host carrier capability, not inferred from
        // successful Job/Wait creation. Unknown and user-confirmed modes cannot
        // claim unattended resume.
        let unattended_available=self.job_terminal_continuations.as_ref().is_some_and(
            |controller|controller.automatic_resume_available_for_wait(
                &principal,&wait_id,chrono::Utc::now().timestamp()
            )
        );
        // Runner may finish between the original Job snapshot and Wait
        // registration. A terminal Wait must not receive a waiting card.
        let current=match db.read_job_terminal_wait(
            &principal,&wait_id,chrono::Utc::now().timestamp()
        ){
            Ok(value)=>value,
            Err(error)=>return ToolResult::err_with_output(
                "Original Host Wait became unreadable after registration",
                json!({"error_kind":error.code,
                    "job_id":job_id,"wait_id":wait_id,
                    "state":"registered","host_wake_verified":false}))
        };
        if current.state!=webcodex_store::JobTerminalWaitState::Waiting{
            let terminal=current.state==webcodex_store::JobTerminalWaitState::Triggered;
            return ToolResult::ok(json!({
                "status":"original_host_wait_registered",
                "state":"registered","job_id":job_id,"wait_id":wait_id,
                "presentation_state":if terminal {
                    "already_terminal_handle_now"
                } else {
                    "original_wait_nonwaiting_requires_reconciliation"
                },
                "wait_state":format!("{:?}",current.state).to_lowercase(),
                "automatic_resume_available":unattended_available,
                "host_wake_verified":false,
                "guidance":"Original Wait is not waiting. Inspect its terminal event and exact Runner Job now. Do not rearm or present a triggered Wait."
            }))
        }
        // Presentation is independently authorized against exact Job/Wait.
        let presented=self.present_job_terminal_continuation(auth,wait_id.clone()).await;
        if !presented.success{
            return ToolResult::ok(json!({
                "status":"original_host_wait_registered",
                "state":"registered","job_id":job_id,"wait_id":wait_id,
                "presentation_state":"presentation_requires_reconciliation",
                "automatic_resume_available":unattended_available,
                "host_wake_verified":false,
                "guidance":"Original Wait is registered. Present exact existing Wait separately; do not create another Worker/Job/Wait."
            }))
        }
        ToolResult::ok(json!({
            "status":"original_host_wait_registered",
            "state":"registered","job_id":job_id,"wait_id":wait_id,
            "presentation_state":"existing_host_card_offered",
            "automatic_resume_available":unattended_available,
            "job_terminal_continuation":presented.output["job_terminal_continuation"],
            "host_wake_verified":false,
            "guidance":"Host continuation was offered in the authorized Window; any later GPT turn still requires independent verification."
        }))
    }
}
