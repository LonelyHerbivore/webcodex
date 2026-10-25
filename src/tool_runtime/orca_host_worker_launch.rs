//! Authorized Host-owned Orca Worker launch through an exact bound Plugin
//! gateway. A SQLite intent fence is committed BEFORE every external effect.
//! There is intentionally no automatic retry, fallback terminal invocation,
//! or implicit Host Job/Wait registration.
use super::{ToolResult,ToolRuntime};
use crate::auth::AuthContext;
use crate::job_terminal_attention::managed_orca_host_principal_for_auth;
use serde_json::{json,Value};
use webcodex_tool_contracts::{PluginToolAction,PluginToolCall};

fn exact_token(v: &str,prefix:&str) -> bool {
    v.len()>prefix.len() && v.len()<=128 && v.starts_with(prefix) &&
    v[prefix.len()..].bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-')
}

fn verified_worker_start(
    output:&Value,
    run:&str,
    coordinator:&str,
) -> Option<(String,String,Option<String>)> {
    let payload=output.get("structuredContent")?;
    if payload.get("ok")?.as_bool()!=Some(true) {return None}
    let result=payload.get("result")?;
    // The original Orca start receipt may include a Run. A conflicting
    // returned Run is stronger negative evidence than a caller-supplied
    // Host continuation plan and must not confirm the wrong Worker.
    if [result.get("runId"),result.pointer("/run/id")]
        .into_iter().flatten().any(|value|value.as_str()!=Some(run)) {
        return None
    }
    let dispatch=result.get("dispatchId").or_else(||result.pointer("/dispatch/id"))?
        .as_str()?;
    let task=result.get("taskId").or_else(||result.pointer("/task/id"))?.as_str()?;
    // If the original Orca receipt contains both top-level and nested
    // Dispatch/Task identities, independently verify BOTH. Selecting the
    // first via or_else would silently ignore a contradictory second value.
    if [result.get("dispatchId"),result.pointer("/dispatch/id")]
        .into_iter().flatten().any(|value|value.as_str()!=Some(dispatch)) ||
       [result.get("taskId"),result.pointer("/task/id")]
        .into_iter().flatten().any(|value|value.as_str()!=Some(task)) {
        return None
    }
    let host=payload.get("hostContinuation")?;
    if host.get("status")?.as_str()?!="requires_host_registration" ||
       host.get("runId")?.as_str()?!=run ||
       host.get("coordinatorHandle")?.as_str()?!=coordinator ||
       host.get("dispatchId")?.as_str()?!=dispatch ||
       host.get("taskId")?.as_str()?!=task ||
       !exact_token(dispatch,"ctx_") || !exact_token(task,"task_")
       {return None}
    let worker_handle=match host.get("workerHandle") {
        None|Some(Value::Null)=>None,
        Some(Value::String(handle)) if exact_token(handle,"term_")=>Some(handle.clone()),
        _=>return None,
    };
    Some((dispatch.to_owned(),task.to_owned(),worker_handle))
}

impl ToolRuntime {
    /// Read-only Project/binding check used before reservation and again
    /// immediately before the external dispatch CAS. Never caches authority.
    async fn verify_orca_host_worker_binding(
        &self,
        project_id:&str,
        plugin_binding:&str,
        auth:Option<&AuthContext>,
        state:&str,
    ) -> Result<(),ToolResult> {
        let resolved=match self.resolve_project_input_for_auth(project_id,auth).await {
            Ok(value) if value.resolved_id==project_id=>value,
            _=>return Err(ToolResult::err_with_output(
                "Original Runner Project authorization unavailable",
                json!({"error_kind":"orca_host_intent_project_unauthorized",
                    "state":state,"worker_started":false,"host_wake_verified":false})))
        };
        if !crate::plugin_gateway::is_exact_orca_worker_start_binding(
            self,plugin_binding,project_id,&resolved.config.client_id
        ) {
            return Err(ToolResult::err_with_output(
                "A pre-described Project-bound exact orca_start_worker binding is required",
                json!({"error_kind":"orca_host_worker_binding_invalid",
                    "state":state,"worker_started":false,"host_wake_verified":false})))
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn start_orca_worker_with_host_intent(
        &self,
        project_id:String,
        run_id:String,
        coordinator_handle:String,
        idempotency_key:String,
        worker_agent:String,
        worktree:String,
        worker_spec:String,
        plugin_binding:String,
        auth:Option<&AuthContext>,
        transport:super::sessions::SessionTransport,
    ) -> ToolResult {
        // A missing/foreign Project-bound binding is a caller error, not an
        // attempted external Worker operation. Reject it before reserving a
        // persistent idempotency key; otherwise an invalid first call can lock
        // out the legitimate first Worker launch on that key forever.
        //
        // Preserve recovery semantics: an existing intent (especially one in
        // worker_start_dispatched) must reach the authoritative preparation
        // replay path, even if its old Plugin binding has since expired.
        let has_required_scopes=auth.is_some_and(|owner|
            owner.has_scope(crate::auth::SCOPE_JOB_RUN)
            && owner.has_scope(crate::auth::SCOPE_PROJECT_WRITE)
            && owner.has_scope(crate::auth::SCOPE_PLUGIN_INVOKE));
        if has_required_scopes {
            let Some(store)=self.job_terminal_db.as_ref() else {
                return ToolResult::err_with_output("Host intent Store unavailable",
                    json!({"error_kind":"orca_host_intent_store_unavailable"}))
            };
            let Some(principal)=managed_orca_host_principal_for_auth(auth) else {
                return ToolResult::err_with_output(
                    "Managed Host owner with stable identity required",
                    json!({"error_kind":"orca_host_intent_unauthorized",
                        "worker_started":false,"host_wake_verified":false}))
            };
            let existing=match store.read_orca_host_intent(&principal,&idempotency_key) {
                Ok(record)=>record.is_some(),
                Err(_)=>return ToolResult::err_with_output(
                    "Cannot establish original Host intent state safely",
                    json!({"error_kind":"orca_host_intent_read_uncertain",
                        "status":"original_worker_launch_requires_reconciliation",
                        "worker_started":null,"host_wake_verified":false}))
            };
            if !existing {
                if let Err(denial)=self.verify_orca_host_worker_binding(
                    &project_id,&plugin_binding,auth,"unrecorded"
                ).await {return denial}
            }
        }
        // The existing preparation function is the single owner of identity,
        // principal-scoped reservation, Project and scope preflight.
        let prepared=self.prepare_orca_host_worker_continuation(
            project_id.clone(),run_id.clone(),coordinator_handle.clone(),
            idempotency_key.clone(),worker_agent.clone(),worktree.clone(),
            worker_spec.clone(),auth,
        ).await;
        if !prepared.success {return prepared}
        let state=prepared.output["state"].as_str().unwrap_or("unknown");
        if state!="reserved" {
            return ToolResult::ok(json!({
                "status":"original_worker_launch_requires_reconciliation",
                "state":state,
                "replayed":true,
                "project_id":project_id,"run_id":run_id,
                "worker_started":prepared.output["worker_started"],
                "watcher_job_started":false,"host_wake_verified":false,
                "guidance":"Original launch intent advanced. Inspect Orca and Host Store before any new action; this call did NOT relaunch the Worker."
            }));
        }
        let Some(db)=self.job_terminal_db.as_ref() else {
            return ToolResult::err_with_output("Host intent Store unavailable",
                json!({"error_kind":"orca_host_intent_store_unavailable"}))
        };
        // Revalidate immediately before CAS: a previously described Plugin
        // binding can expire or change while an intent is being reserved.
        if let Err(denial)=self.verify_orca_host_worker_binding(
            &project_id,&plugin_binding,auth,"reserved"
        ).await {return denial}
        let Some(principal)=managed_orca_host_principal_for_auth(auth) else {
            return ToolResult::err_with_output(
                "Managed Host owner changed during Worker launch",
                json!({"error_kind":"orca_host_intent_unauthorized",
                    "status":"original_worker_launch_requires_reconciliation",
                    "worker_started":false,"host_wake_verified":false}))
        };
        let now=chrono::Utc::now().timestamp();
        if let Err(error)=db.transition_orca_host_intent(
            &principal,&idempotency_key,"reserved","worker_start_dispatched",
            None,None,None,None,now
        ) {
            return ToolResult::err_with_output(
                "Original Host Worker was already claimed; never re-dispatch",
                json!({"error_kind":error.code,
                  "status":"original_worker_launch_requires_reconciliation",
                  "state":"unknown","host_wake_verified":false})
            );
        }

        // Call the official Plugin Gateway. It rechecks exact live Runner,
        // provider instance, schema, Project root, write permission, business
        // Session and Plugin invocation approval. No internal or shell bypass.
        let tool=PluginToolCall {
            action:PluginToolAction::Call,
            runner:None,plugin:None,tool:None,
            binding:Some(plugin_binding),project:None,
            arguments:Some(json!({
                "runId":run_id,
                "fromTerminal":coordinator_handle,
                "agent":worker_agent,
                "worktree":worktree,
                "spec":worker_spec
            })),
        };
        let executed=self.dispatch_plugin_gateway(tool,None,auth,transport).await;
        if !executed.success {
            return ToolResult::err_with_output(
                "Orca Plugin launch not authoritatively confirmed; never retry automatically",
                json!({
                    "error_kind":"orca_worker_launch_outcome_unconfirmed",
                    "status":"original_worker_launch_requires_reconciliation",
                    "state":"worker_start_dispatched",
                    "plugin_failure_kind":executed.output["failure_kind"],
                    "plugin_dispatch_certainty":executed.output["dispatch_certainty"],
                    "worker_started":null,
                    "watcher_job_started":false,"host_wake_verified":false,
                    "guidance":"Inspect exact original Host intent and Orca request journal; no duplicate Worker may be started, even when the gateway reports denied or not_started."
                }),
            );
        }
        let Some((dispatch,task,worker_handle))=verified_worker_start(
            &executed.output,&run_id,&coordinator_handle
        ) else {
            return ToolResult::err_with_output(
                "Orca start returned no independently correlated Worker/Task identity",
                json!({"error_kind":"orca_worker_identity_unconfirmed",
                  "status":"original_worker_launch_requires_reconciliation",
                  "state":"worker_start_dispatched","worker_started":null,
                  "watcher_job_started":false,"host_wake_verified":false}),
            );
        };
        match db.transition_orca_host_intent(
            &principal,&idempotency_key,"worker_start_dispatched",
            "worker_start_confirmed",Some((&dispatch,&task)),
            None,None,None,chrono::Utc::now().timestamp()
        ) {
            Ok(_)=>ToolResult::ok(json!({
                "status":"original_orca_worker_launch_confirmed",
                "state":"worker_start_confirmed",
                "project_id":project_id,"run_id":run_id,
                "coordinator_handle":coordinator_handle,
                "idempotency_key":idempotency_key,
                "worker_agent":worker_agent,
                "dispatch_id":dispatch,"task_id":task,
                "worker_handle_verified":worker_handle.is_some(),
                "worker_handle":worker_handle,
                "worker_started":true,
                "watcher_job_started":false,"host_wake_verified":false,
                "replayed":false,
                "guidance":"Exact Orca Worker identity durably saved; Host watcher and Wait registration still require separate authorization."
            })),
            Err(error)=>ToolResult::err_with_output(
                "Original Worker may have started; Host confirmation was not persisted",
                json!({"error_kind":error.code,
                  "status":"original_worker_launch_requires_reconciliation",
                  "state":"worker_start_dispatched","worker_started":null,
                  "watcher_job_started":false,"host_wake_verified":false}),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn confirmed_native_orca_worker_result_requires_correlated_identity() {
        let proof=json!({"structuredContent":{
            "ok":true,
            "result":{"dispatchId":"ctx_confirmed","taskId":"task_confirmed"},
            "hostContinuation":{"status":"requires_host_registration",
                "runId":"run_original","coordinatorHandle":"term_owned",
                "dispatchId":"ctx_confirmed","taskId":"task_confirmed"}
        }});
        assert_eq!(verified_worker_start(&proof,"run_original","term_owned"),
            Some(("ctx_confirmed".to_string(),"task_confirmed".to_string(),None)));
        let mut identified=proof.clone();
        identified["structuredContent"]["hostContinuation"]["workerHandle"]=json!("term_worker123");
        assert_eq!(verified_worker_start(&identified,"run_original","term_owned"),
            Some(("ctx_confirmed".to_string(),"task_confirmed".to_string(),
                Some("term_worker123".to_string()))));
        identified["structuredContent"]["hostContinuation"]["workerHandle"]=json!("invented_worker");
        assert_eq!(verified_worker_start(&identified,"run_original","term_owned"),None);
        identified["structuredContent"]["hostContinuation"]["workerHandle"]=json!(42);
        assert_eq!(verified_worker_start(&identified,"run_original","term_owned"),None);
        assert_eq!(verified_worker_start(&proof,"run_foreign","term_owned"),None);
        let mut wrong_run=proof.clone();
        wrong_run["structuredContent"]["result"]["runId"]=json!("run_foreign");
        assert_eq!(verified_worker_start(&wrong_run,"run_original","term_owned"),None);
        wrong_run["structuredContent"]["result"]["runId"]=json!("run_original");
        wrong_run["structuredContent"]["result"]["run"]["id"]=json!("run_foreign");
        assert_eq!(verified_worker_start(&wrong_run,"run_original","term_owned"),None);
        wrong_run["structuredContent"]["result"]["run"]["id"]=json!("run_original");
        assert_eq!(verified_worker_start(&wrong_run,"run_original","term_owned"),
            Some(("ctx_confirmed".to_string(),"task_confirmed".to_string(),None)));
        // Both representations of Dispatch/Task in the original start
        // receipt must be consistent even when the later Host plan matches.
        let mut receipt_conflict=proof.clone();
        receipt_conflict["structuredContent"]["result"]["dispatch"]["id"]=json!("ctx_foreign");
        assert_eq!(verified_worker_start(&receipt_conflict,"run_original","term_owned"),None);
        receipt_conflict["structuredContent"]["result"]["dispatch"]["id"]=json!("ctx_confirmed");
        assert_eq!(verified_worker_start(&receipt_conflict,"run_original","term_owned"),
            Some(("ctx_confirmed".to_owned(),"task_confirmed".to_owned(),None)));
        receipt_conflict["structuredContent"]["result"]["task"]["id"]=json!("task_foreign");
        assert_eq!(verified_worker_start(&receipt_conflict,"run_original","term_owned"),None);
        receipt_conflict["structuredContent"]["result"]["task"]["id"]=json!("task_confirmed");
        assert_eq!(verified_worker_start(&receipt_conflict,"run_original","term_owned"),
            Some(("ctx_confirmed".to_owned(),"task_confirmed".to_owned(),None)));
        receipt_conflict["structuredContent"]["result"]["dispatchId"]=json!("ctx_foreign");
        assert_eq!(verified_worker_start(&receipt_conflict,"run_original","term_owned"),None);
        receipt_conflict["structuredContent"]["result"]["dispatchId"]=json!("ctx_confirmed");
        receipt_conflict["structuredContent"]["result"]["taskId"]=json!("task_foreign");
        assert_eq!(verified_worker_start(&receipt_conflict,"run_original","term_owned"),None);
        let mut altered=proof.clone();
        altered["structuredContent"]["hostContinuation"]["taskId"]=json!("task_other");
        assert_eq!(verified_worker_start(&altered,"run_original","term_owned"),None);
        altered["structuredContent"]["hostContinuation"]["taskId"]=json!("task_confirmed");
        altered["structuredContent"]["hostContinuation"]["status"]=json!("needs_identity_reconciliation");
        assert_eq!(verified_worker_start(&altered,"run_original","term_owned"),None);
    }
}
