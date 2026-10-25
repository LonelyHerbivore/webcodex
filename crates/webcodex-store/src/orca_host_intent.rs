//! Durable, principal-scoped Orca Worker/Host continuation *intent*.
 //!
 //! This store does not start Workers or register waits. Callers must first
 //! authenticate and authorize the exact Host/Runner Project; an intent never
 //! grants execution authority or permits retrying an ambiguous effect.
 use super::{Database, job_terminal_wait::JobTerminalWaitPrincipal};
 use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
 use std::fmt;

 pub const MAX_ORCA_HOST_INTENTS_PER_PRINCIPAL: i64 = 256;
 pub const MAX_ORCA_HOST_INTENTS_GLOBAL: i64 = 4096;

 #[derive(Debug, Clone, PartialEq, Eq)]
 pub struct OrcaHostIntentError {
     pub code: &'static str,
     pub message: String,
 }
 impl fmt::Display for OrcaHostIntentError {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.message) }
 }
 impl std::error::Error for OrcaHostIntentError {}
 fn error(code: &'static str, message: impl Into<String>) -> OrcaHostIntentError {
     OrcaHostIntentError {code, message: message.into()}
 }
 fn sql_error(_: rusqlite::Error) -> OrcaHostIntentError {
     error("orca_host_intent_store_error","Host intent storage operation failed")
 }

 #[derive(Debug, Clone, PartialEq, Eq)]
 pub struct NewOrcaHostIntent {
     pub idempotency_key: String,
     /// A trusted Host-calculated SHA-256 digest of canonical authorized request.
     /// Raw prompts and credentials must never be persisted in this table.
     pub request_digest: String,
     pub project_id: String,
     pub run_id: String,
     pub coordinator_handle: String,
 }
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub struct OrcaHostIntentRecord {
     pub idempotency_key: String,
     pub request_digest: String,
     pub project_id: String,
     pub run_id: String,
     pub coordinator_handle: String,
     pub state: String,
     pub dispatch_id: Option<String>,
     pub task_id: Option<String>,
     pub watch_key: Option<String>,
     pub job_id: Option<String>,
     pub wait_id: Option<String>,
     pub created_at: i64,
     pub updated_at: i64,
 }
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub struct OrcaHostIntentMutation {
     pub intent: OrcaHostIntentRecord,
     pub replayed: bool,
 }

 pub(crate) const STEPS: [(&str, &str); 7] = [
     ("reserved","worker_start_dispatched"),
     ("worker_start_dispatched","worker_start_confirmed"),
     ("worker_start_confirmed","watch_start_dispatched"),
     ("watch_start_dispatched","watch_job_confirmed"),
     ("watch_job_confirmed","wait_registration_dispatched"),
     ("wait_registration_dispatched","wait_registered"),
     ("wait_registered","registered"),
 ];

 fn valid_token(input:&str, prefix:&str, max:usize) -> bool {
     input.len()>prefix.len() && input.len()<=max && input.starts_with(prefix)
         && input[prefix.len()..].bytes().all(|b| b.is_ascii_alphanumeric() || b==b'_' || b==b'-')
 }
 fn valid_sha256(value:&str) -> bool {
     value.len()==64 && value.bytes().all(|b| b.is_ascii_hexdigit())
 }
 fn validate_principal(principal:&JobTerminalWaitPrincipal)->Result<(),OrcaHostIntentError>{
     if principal.kind.is_empty() || principal.kind.len()>64 ||
         !principal.kind.bytes().all(|b| b.is_ascii_alphanumeric()||b==b'_'||b==b'-') ||
         !valid_sha256(&principal.digest) {
         return Err(error("orca_host_intent_invalid","Trusted Host principal is invalid"));
     }
     Ok(())
 }
 fn valid_project(value:&str)->bool {
     let mut parts=value.split(':');
     parts.next()==Some("agent") &&
       parts.next().is_some_and(|p|!p.is_empty()&&p.len()<=128&&p.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-')) &&
       parts.next().is_some_and(|p|!p.is_empty()&&p.len()<=128&&p.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-')) &&
       parts.next().is_none() && value.len()<=256
 }
 fn validate_request(req:&NewOrcaHostIntent)->Result<(),OrcaHostIntentError>{
     if req.idempotency_key.len()<8 || req.idempotency_key.len()>128 ||
         !req.idempotency_key.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'_'||b==b'-') ||
         !valid_sha256(&req.request_digest) ||
         !valid_project(&req.project_id) ||
         !valid_token(&req.run_id,"run_",128) ||
         !valid_token(&req.coordinator_handle,"term_",128) {
         return Err(error("orca_host_intent_invalid","Original request identity invalid"));
     }
     Ok(())
 }
 fn load(conn:&Connection,principal:&JobTerminalWaitPrincipal,key:&str)
     ->Result<Option<OrcaHostIntentRecord>,OrcaHostIntentError>{
     conn.query_row(
         "SELECT idempotency_key,request_digest,project_id,run_id,coordinator_handle,state,dispatch_id,task_id,watch_key,job_id,wait_id,created_at,updated_at
          FROM wc_orca_host_intents WHERE owner_kind=?1 AND owner_digest=?2 AND idempotency_key=?3",
         params![principal.kind,principal.digest,key],
         |r|Ok(OrcaHostIntentRecord{
             idempotency_key:r.get(0)?,request_digest:r.get(1)?,project_id:r.get(2)?,
             run_id:r.get(3)?,coordinator_handle:r.get(4)?,state:r.get(5)?,
             dispatch_id:r.get(6)?,task_id:r.get(7)?,watch_key:r.get(8)?,
             job_id:r.get(9)?,wait_id:r.get(10)?,created_at:r.get(11)?,updated_at:r.get(12)?,
         })
     ).optional().map_err(sql_error)
 }

 impl Database {
     pub(super) fn ensure_orca_host_intent_schema(conn:&mut Connection)->anyhow::Result<()> {
         conn.execute_batch(
             "CREATE TABLE IF NOT EXISTS wc_orca_host_intents (
                owner_kind TEXT NOT NULL, owner_digest TEXT NOT NULL,
                idempotency_key TEXT NOT NULL, request_digest TEXT NOT NULL,
                project_id TEXT NOT NULL, run_id TEXT NOT NULL,
                coordinator_handle TEXT NOT NULL,
                state TEXT NOT NULL CHECK (state IN ('reserved','worker_start_dispatched',
                  'worker_start_confirmed','watch_start_dispatched','watch_job_confirmed',
                  'wait_registration_dispatched','wait_registered','registered')),
                dispatch_id TEXT, task_id TEXT, watch_key TEXT, job_id TEXT, wait_id TEXT,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
                PRIMARY KEY (owner_kind,owner_digest,idempotency_key),
                CHECK ((dispatch_id IS NULL AND task_id IS NULL) OR
                       (dispatch_id IS NOT NULL AND task_id IS NOT NULL)),
                CHECK (wait_id IS NULL OR job_id IS NOT NULL),
                CHECK (job_id IS NULL OR watch_key IS NOT NULL)
             );
             CREATE INDEX IF NOT EXISTS idx_orca_host_intents_owner
              ON wc_orca_host_intents(owner_kind,owner_digest,updated_at);"
         )?;
         Ok(())
     }
     pub fn reserve_orca_host_intent(&self,principal:&JobTerminalWaitPrincipal,
         request:&NewOrcaHostIntent,now:i64)
         ->Result<OrcaHostIntentMutation,OrcaHostIntentError>{
         validate_principal(principal)?;
         validate_request(request)?;
         let mut conn=self.lock_connection(crate::StoreDomain::JobTerminalWait);
         let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql_error)?;
         if let Some(existing)=load(&tx,principal,&request.idempotency_key)? {
             if existing.request_digest!=request.request_digest ||
                 existing.project_id!=request.project_id ||
                 existing.run_id!=request.run_id ||
                 existing.coordinator_handle!=request.coordinator_handle {
                 return Err(error("orca_host_intent_conflict",
                      "Original idempotency key was already bound to different Worker inputs"));
             }
             tx.commit().map_err(sql_error)?;
             return Ok(OrcaHostIntentMutation{intent:existing,replayed:true});
         }
         let local:i64=tx.query_row(
             "SELECT COUNT(*) FROM wc_orca_host_intents WHERE owner_kind=?1 AND owner_digest=?2",
             params![principal.kind,principal.digest],|r|r.get(0)).map_err(sql_error)?;
         let global:i64=tx.query_row("SELECT COUNT(*) FROM wc_orca_host_intents",[],
             |r|r.get(0)).map_err(sql_error)?;
         if local>=MAX_ORCA_HOST_INTENTS_PER_PRINCIPAL ||global>=MAX_ORCA_HOST_INTENTS_GLOBAL{
             return Err(error("orca_host_intent_capacity",
                 "Durable Host intents at capacity; inspect and reconcile before new work"));
         }
         tx.execute(
             "INSERT INTO wc_orca_host_intents
              (owner_kind,owner_digest,idempotency_key,request_digest,project_id,run_id,coordinator_handle,state,created_at,updated_at)
              VALUES (?1,?2,?3,?4,?5,?6,?7,'reserved',?8,?8)",
             params![principal.kind,principal.digest,request.idempotency_key,
                 request.request_digest,request.project_id,request.run_id,
                 request.coordinator_handle,now]
         ).map_err(sql_error)?;
         let intent=load(&tx,principal,&request.idempotency_key)?
             .ok_or_else(||error("orca_host_intent_store_error","New intent could not be read"))?;
         tx.commit().map_err(sql_error)?;
         Ok(OrcaHostIntentMutation{intent,replayed:false})
     }

     /// One strict CAS state move. The Host must persist an effect-dispatched
     /// phase before invoking a Plugin/Runner/Wait. A lost reply leaves it there
     /// for authoritative reconciliation; no automatic rollback or replay.
     pub fn transition_orca_host_intent(&self,principal:&JobTerminalWaitPrincipal,
         key:&str,from:&str,to:&str,dispatch_task:Option<(&str,&str)>,
         watch_key:Option<&str>,job_id:Option<&str>,wait_id:Option<&str>,now:i64)
         ->Result<OrcaHostIntentRecord,OrcaHostIntentError>{
         validate_principal(principal)?;
         if !STEPS.iter().any(|p|p.0==from&&p.1==to){
             return Err(error("orca_host_intent_invalid_transition",
                 "Host intent transition is not an admitted forward step"));
         }
         if dispatch_task.is_some_and(|(d,t)|!valid_token(d,"ctx_",128)||!valid_token(t,"task_",128)) ||
             watch_key.is_some_and(|v|v.len()!=45||!valid_token(v,"orca_host_wd_",45)||
                 !v[13..].bytes().all(|b|b.is_ascii_hexdigit())) ||
             job_id.is_some_and(|v|!valid_token(v,"wc_job_",128)) ||
             wait_id.is_some_and(|v|!valid_token(v,"wc_job_wait_",128)){
             return Err(error("orca_host_intent_invalid","Unverified Host/Orca effect identity"));
         }
         let mut conn=self.lock_connection(crate::StoreDomain::JobTerminalWait);
         let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql_error)?;
         let prior=load(&tx,principal,key)?
             .ok_or_else(||error("orca_host_intent_not_found","Exact original intent is unavailable"))?;
         if prior.state!=from{
             return Err(error("orca_host_intent_state_conflict",
                 "Host intent has a different phase; reconcile the original effect first"));
         }
         let (dispatch,task)=dispatch_task.map_or((None,None),|(d,t)|(Some(d),Some(t)));
         if matches!(to,"worker_start_confirmed") != dispatch.is_some() ||
             (watch_key.is_some(),job_id.is_some()) !=
                 (matches!(to,"watch_job_confirmed"),matches!(to,"watch_job_confirmed")) ||
             matches!(to,"wait_registered") != wait_id.is_some() ||
             (!matches!(to,"worker_start_confirmed") && dispatch.is_some()) ||
             (!matches!(to,"watch_job_confirmed") && (watch_key.is_some()||job_id.is_some())) ||
             (!matches!(to,"wait_registered") && wait_id.is_some()){
             return Err(error("orca_host_intent_invalid","Transition is missing or has unexpected identity evidence"));
         }
         let changed=tx.execute(
             "UPDATE wc_orca_host_intents
              SET state=?4, dispatch_id=COALESCE(?5,dispatch_id),
                  task_id=COALESCE(?6,task_id),watch_key=COALESCE(?7,watch_key),
                  job_id=COALESCE(?8,job_id),wait_id=COALESCE(?9,wait_id),
                  updated_at=MAX(updated_at,?10)
              WHERE owner_kind=?1 AND owner_digest=?2 AND idempotency_key=?3 AND state=?11",
             params![principal.kind,principal.digest,key,to,dispatch,task,
                 watch_key,job_id,wait_id,now,from]
         ).map_err(sql_error)?;
         if changed!=1{return Err(error("orca_host_intent_state_conflict",
             "Concurrent Host intent transition changed the state"))}
         let result=load(&tx,principal,key)?
             .ok_or_else(||error("orca_host_intent_store_error","Updated intent unreadable"))?;
         tx.commit().map_err(sql_error)?;
         Ok(result)
     }
     pub fn read_orca_host_intent(&self,principal:&JobTerminalWaitPrincipal,key:&str)
         ->Result<Option<OrcaHostIntentRecord>,OrcaHostIntentError>{
         validate_principal(principal)?;
         let conn=self.lock_connection(crate::StoreDomain::JobTerminalWait);
         load(&conn,principal,key)
     }
 }
