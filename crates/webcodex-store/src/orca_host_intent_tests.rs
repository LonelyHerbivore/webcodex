use super::*;
use tempfile::tempdir;

fn principal(ch: char) -> JobTerminalWaitPrincipal {
    JobTerminalWaitPrincipal {
        kind: "managed_owner".to_owned(),
        digest: ch.to_string().repeat(64),
    }
}
fn request() -> NewOrcaHostIntent {
    NewOrcaHostIntent {
        idempotency_key: "orca_original_launch_123456".to_owned(),
        request_digest: "a".repeat(64),
        project_id: "agent:laptop-id:test-project".to_owned(),
        run_id: "run_sample".to_owned(),
        coordinator_handle: "term_original".to_owned(),
    }
}
fn transition(db:&Database,owner:&JobTerminalWaitPrincipal,from:&str,to:&str,
    worker:Option<(&str,&str)>,watch:Option<&str>,
    job:Option<&str>,wait:Option<&str>)->OrcaHostIntentRecord {
    db.transition_orca_host_intent(owner,&request().idempotency_key,
        from,to,worker,watch,job,wait,2000).unwrap()
}

#[test]
fn original_intent_persists_across_reopen_and_replay_does_not_repeat_effect() {
    let temp=tempdir().unwrap();
    let path=temp.path().join("orca-host-intent.db");
    let owner=principal('a');
    {
        let db=Database::open(&path).unwrap();
        let first=db.reserve_orca_host_intent(&owner,&request(),1000).unwrap();
        assert!(!first.replayed);
        assert_eq!(first.intent.state,"reserved");
        let replay=db.reserve_orca_host_intent(&owner,&request(),1001).unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.intent.created_at,1000);
        transition(&db,&owner,"reserved","worker_start_dispatched",None,None,None,None);
    }
    let db=Database::open(&path).unwrap();
    let restarted=db.reserve_orca_host_intent(&owner,&request(),5000).unwrap();
    assert!(restarted.replayed);
    assert_eq!(restarted.intent.state,"worker_start_dispatched");
    assert_eq!(db.read_orca_host_intent(&principal('b'),&request().idempotency_key)
        .unwrap(),None);
}
#[test]
fn same_key_changed_request_or_different_project_fails_closed() {
    let temp=tempdir().unwrap();
    let db=Database::open(&temp.path().join("state.db")).unwrap();
    let owner=principal('a');
    db.reserve_orca_host_intent(&owner,&request(),1000).unwrap();
    for changed in [
        NewOrcaHostIntent{request_digest:"b".repeat(64),..request()},
        NewOrcaHostIntent{project_id:"agent:other:project".to_owned(),..request()},
        NewOrcaHostIntent{run_id:"run_different".to_owned(),..request()},
    ]{
        let error=db.reserve_orca_host_intent(&owner,&changed,1001).unwrap_err();
        assert_eq!(error.code,"orca_host_intent_conflict");
    }
}
#[test]
fn state_machine_fences_exactly_one_forward_mutation_at_each_side_effect() {
    let temp=tempdir().unwrap();
    let db=Database::open(&temp.path().join("state.db")).unwrap();
    let owner=principal('a');
    db.reserve_orca_host_intent(&owner,&request(),1000).unwrap();
    let from=transition(&db,&owner,"reserved","worker_start_dispatched",None,None,None,None);
    assert_eq!(from.state,"worker_start_dispatched");
    assert_eq!(db.transition_orca_host_intent(&owner,&request().idempotency_key,
       "reserved","worker_start_dispatched",None,None,None,None,1002)
       .unwrap_err().code,"orca_host_intent_state_conflict");
    let worker=transition(&db,&owner,"worker_start_dispatched","worker_start_confirmed",
        Some(("ctx_dispatch","task_task")),None,None,None);
    assert_eq!(worker.dispatch_id.as_deref(),Some("ctx_dispatch"));
    transition(&db,&owner,"worker_start_confirmed","watch_start_dispatched",None,None,None,None);
    let watch_key=format!("orca_host_wd_{}","a".repeat(32));
    let job=transition(&db,&owner,"watch_start_dispatched","watch_job_confirmed",
        None,Some(&watch_key),Some("wc_job_12345678"),None);
    assert_eq!(job.job_id.as_deref(),Some("wc_job_12345678"));
    transition(&db,&owner,"watch_job_confirmed","wait_registration_dispatched",None,None,None,None);
    let wait=transition(&db,&owner,"wait_registration_dispatched","wait_registered",
        None,None,None,Some("wc_job_wait_12345678"));
    assert_eq!(wait.wait_id.as_deref(),Some("wc_job_wait_12345678"));
    let final_state=transition(&db,&owner,"wait_registered","registered",None,None,None,None);
    assert_eq!(final_state.state,"registered");
    assert_eq!(db.transition_orca_host_intent(&owner,&request().idempotency_key,
       "registered","worker_start_dispatched",None,None,None,None,1003)
       .unwrap_err().code,"orca_host_intent_invalid_transition");
}
#[test]
fn missing_identity_and_skipped_phase_cannot_be_recorded() {
    let temp=tempdir().unwrap();
    let db=Database::open(&temp.path().join("state.db")).unwrap();
    let owner=principal('a');
    db.reserve_orca_host_intent(&owner,&request(),1000).unwrap();
    assert_eq!(db.transition_orca_host_intent(&owner,&request().idempotency_key,
       "reserved","watch_job_confirmed",None,Some("orca_host_wd_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
       Some("wc_job_original"),None,1002).unwrap_err().code,
       "orca_host_intent_invalid_transition");
    transition(&db,&owner,"reserved","worker_start_dispatched",None,None,None,None);
    assert_eq!(db.transition_orca_host_intent(&owner,&request().idempotency_key,
       "worker_start_dispatched","worker_start_confirmed",None,None,None,None,1002)
       .unwrap_err().code,"orca_host_intent_invalid");
    let persisted=db.read_orca_host_intent(&owner,&request().idempotency_key).unwrap().unwrap();
    assert_eq!(persisted.state,"worker_start_dispatched");
}
#[test]
fn invalid_authority_shape_never_creates_an_intent() {
    let temp=tempdir().unwrap();
    let db=Database::open(&temp.path().join("state.db")).unwrap();
    let owner=principal('a');
    for bad in [
        NewOrcaHostIntent{project_id:"agent:other".to_owned(),..request()},
        NewOrcaHostIntent{project_id:"agent:other:../bad".to_owned(),..request()},
        NewOrcaHostIntent{idempotency_key:"short".to_owned(),..request()},
        NewOrcaHostIntent{request_digest:"not-a-hash".to_owned(),..request()},
    ]{
        assert_eq!(db.reserve_orca_host_intent(&owner,&bad,1000).unwrap_err().code,
            "orca_host_intent_invalid");
    }
    assert!(db.read_orca_host_intent(&owner,&request().idempotency_key).unwrap().is_none());
}
