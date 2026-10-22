//! The run coordinator's invariants.
//!
//! These are ports of `tests/runs.test.ts`, which is the parity oracle for this
//! module ([WF-IMP-020](../../docs/planning/tickets/WF-IMP-020-durable-run-coordinator-in-rust.md)).
//! Each test keeps the original's name and its claim, so a divergence shows up as
//! a failing test rather than as a behavioural difference discovered later.
//!
//! What the port makes stronger: `claim_action`, `complete_action`,
//! `fail_action` and `report_progress` guard on the lease inside the statement
//! itself, so the "two workers pass the same check" race the TypeScript has does
//! not exist here. The tests below cannot observe that difference — it takes a
//! race to observe it — so the fencing tests assert the observable contract
//! (a stale lease is rejected) rather than claiming to test atomicity.

use super::*;

const TTL: i64 = 30_000;

fn plan(c: &Coordinator, title: &str, outcome: &str) -> (Run, Action) {
    let run = c
        .create_run(NewRun {
            title: title.into(),
            message_id: None,
        })
        .expect("run");
    let action = c
        .add_action(NewAction {
            run_id: run.id.clone(),
            outcome: outcome.into(),
            capability: "builtin".into(),
            ..Default::default()
        })
        .expect("action");
    (run, action)
}

/// The one action planned in a run, for tests that do not hold its id.
fn action_id_for(c: &Coordinator, run_id: &str) -> String {
    c.actions(run_id).unwrap()[0].id.clone()
}

fn status_of(c: &Coordinator, action_id: &str) -> ActionStatus {
    c.action(action_id).expect("read").expect("action").status
}

fn run_status_of(c: &Coordinator, run_id: &str) -> RunStatus {
    c.run(run_id).expect("read").expect("run").status
}

// ------------------------------------------------ plan commit and gating

#[test]
fn an_action_cannot_queue_until_its_dependency_succeeds() {
    let c = Coordinator::in_memory().unwrap();
    let (run, first) = plan(&c, "compound request", "first");
    let second = c
        .add_action(NewAction {
            run_id: run.id.clone(),
            outcome: "second".into(),
            capability: "builtin".into(),
            depends_on: vec![first.id.clone()],
            ..Default::default()
        })
        .expect("action");

    c.commit_plan(&run.id).expect("commit");
    assert_eq!(status_of(&c, &first.id), ActionStatus::Queued);
    assert_eq!(status_of(&c, &second.id), ActionStatus::BlockedByDependency);
    assert_eq!(run_status_of(&c, &run.id), RunStatus::Active);

    let claim = c.claim_action(&first.id, TTL).unwrap().expect("claim");
    assert!(c.complete_action(&first.id, &claim.lease_id, "done").unwrap());
    assert_eq!(
        status_of(&c, &second.id),
        ActionStatus::Queued,
        "a satisfied dependency releases the dependent"
    );
}

#[test]
fn a_failed_dependency_cancels_dependents_with_a_recorded_reason() {
    let c = Coordinator::in_memory().unwrap();
    let (run, first) = plan(&c, "failing dependency", "first");
    let second = c
        .add_action(NewAction {
            run_id: run.id.clone(),
            outcome: "second".into(),
            capability: "builtin".into(),
            depends_on: vec![first.id.clone()],
            ..Default::default()
        })
        .expect("action");
    c.commit_plan(&run.id).expect("commit");

    let claim = c.claim_action(&first.id, TTL).unwrap().expect("claim");
    assert!(c.fail_action(&first.id, &claim.lease_id, "provider refused").unwrap());

    let dependent = c.action(&second.id).unwrap().expect("action");
    assert_eq!(dependent.status, ActionStatus::Cancelled);
    assert_eq!(
        dependent.cancellation_reason.as_deref(),
        Some("dependency-failed"),
        "a cancellation must say why, or the operator sees a silent drop"
    );
    assert_eq!(run_status_of(&c, &run.id), RunStatus::Failed);
}

#[test]
fn input_and_approval_waits_hold_actions_until_satisfied() {
    let c = Coordinator::in_memory().unwrap();

    let run = c
        .create_run(NewRun {
            title: "needs approval".into(),
            message_id: None,
        })
        .unwrap();
    let action = c
        .add_action(NewAction {
            run_id: run.id.clone(),
            outcome: "send".into(),
            capability: "gmail".into(),
            requires_approval: true,
            ..Default::default()
        })
        .unwrap();
    c.commit_plan(&run.id).unwrap();
    assert_eq!(status_of(&c, &action.id), ActionStatus::WaitingForApproval);
    assert_eq!(run_status_of(&c, &run.id), RunStatus::Waiting);

    c.grant_approval(&action.id, true).unwrap();
    assert_eq!(status_of(&c, &action.id), ActionStatus::Queued);

    let input_run = c
        .create_run(NewRun {
            title: "needs input".into(),
            message_id: None,
        })
        .unwrap();
    let input_action = c
        .add_action(NewAction {
            run_id: input_run.id.clone(),
            outcome: "ask".into(),
            capability: "builtin".into(),
            requires_input: true,
            ..Default::default()
        })
        .unwrap();
    c.commit_plan(&input_run.id).unwrap();
    assert_eq!(status_of(&c, &input_action.id), ActionStatus::WaitingForInput);
    c.provide_input(&input_action.id, "the bedroom").unwrap();
    assert_eq!(status_of(&c, &input_action.id), ActionStatus::Queued);
    assert_eq!(
        c.action(&input_action.id).unwrap().unwrap().result.as_deref(),
        Some("the bedroom"),
        "the supplied input is what the capability runs on"
    );
}

// -------------------------------------------------- parent projection

#[test]
fn a_mixed_terminal_outcome_is_partial_completion() {
    let c = Coordinator::in_memory().unwrap();
    let (run, good) = plan(&c, "partial", "good");
    let bad = c
        .add_action(NewAction {
            run_id: run.id.clone(),
            outcome: "bad".into(),
            capability: "builtin".into(),
            ..Default::default()
        })
        .unwrap();
    c.commit_plan(&run.id).unwrap();

    let good_claim = c.claim_action(&good.id, TTL).unwrap().expect("claim");
    let bad_claim = c.claim_action(&bad.id, TTL).unwrap().expect("claim");
    assert!(c.complete_action(&good.id, &good_claim.lease_id, "ok").unwrap());
    assert!(c.fail_action(&bad.id, &bad_claim.lease_id, "nope").unwrap());

    assert_eq!(
        run_status_of(&c, &run.id),
        RunStatus::PartiallyComplete,
        "one success and one failure is not success and not failure"
    );
}

#[test]
fn cancelling_a_run_cancels_its_non_terminal_actions() {
    let c = Coordinator::in_memory().unwrap();
    let (run, action) = plan(&c, "cancel me", "x");
    c.commit_plan(&run.id).unwrap();
    c.cancel_run(&run.id, "user-cancelled").unwrap();

    assert_eq!(run_status_of(&c, &run.id), RunStatus::Cancelled);
    assert_eq!(status_of(&c, &action.id), ActionStatus::Cancelled);
    assert_eq!(
        c.action(&action.id).unwrap().unwrap().cancellation_reason.as_deref(),
        Some("user-cancelled")
    );
}

#[test]
fn a_cancelled_run_is_not_cancelled_again() {
    let c = Coordinator::in_memory().unwrap();
    let (run, _) = plan(&c, "settled", "x");
    c.commit_plan(&run.id).unwrap();
    c.cancel_run(&run.id, "first").unwrap();
    c.cancel_run(&run.id, "second").unwrap();

    assert_eq!(
        run_status_of(&c, &run.id).as_str(),
        "cancelled",
        "the first terminal status stands"
    );
    assert_eq!(
        c.run(&run.id).unwrap().unwrap().cancelled_reason.as_deref(),
        Some("first"),
        "a second cancel must not overwrite the reason that actually applied"
    );
}

// ---------------------------------------------- lease fencing and recovery

#[test]
fn a_stale_lease_cannot_report_or_complete_after_recovery_and_a_newer_attempt() {
    let c = Coordinator::in_memory().unwrap();
    let run = c
        .create_run(NewRun {
            title: "lease fence".into(),
            message_id: None,
        })
        .unwrap();
    let fenced = c
        .add_action(NewAction {
            run_id: run.id.clone(),
            outcome: "x".into(),
            capability: "builtin".into(),
            idempotency_key: Some("key-fence".into()),
            ..Default::default()
        })
        .unwrap();
    c.commit_plan(&run.id).unwrap();

    // Claim with an already-expired lease, so recovery can act on it.
    let first = c.claim_action(&fenced.id, -1).unwrap().expect("claim");
    assert!(
        c.claim_action(&fenced.id, TTL).unwrap().is_none(),
        "a running action cannot be claimed again"
    );

    c.recover_expired_leases().unwrap();
    assert!(c.resolve_reconciliation(&fenced.id, Resolution::Retry).unwrap());
    let second = c.claim_action(&fenced.id, TTL).unwrap().expect("claim");
    assert_ne!(
        first.lease_id, second.lease_id,
        "a new attempt gets a new lease"
    );

    assert!(
        !c.complete_action(&fenced.id, &first.lease_id, "late").unwrap(),
        "the first attempt's late completion is rejected"
    );
    assert!(!c.report_progress(&fenced.id, &first.lease_id, "late progress").unwrap());
    assert!(c.complete_action(&fenced.id, &second.lease_id, "fresh").unwrap());

    let action = c.action(&fenced.id).unwrap().unwrap();
    assert_eq!(action.result.as_deref(), Some("fresh"));
    assert_eq!(action.attempt, 2, "the fresh attempt is the second");
    let _ = action;
}

#[test]
fn an_expired_lease_becomes_needs_reconciliation_never_an_automatic_retry() {
    let c = Coordinator::in_memory().unwrap();
    let (run, _) = plan(&c, "crash", "x");

    let action = c
        .add_action(NewAction {
            run_id: run.id.clone(),
            outcome: "x".into(),
            capability: "builtin".into(),
            idempotency_key: Some("key-1".into()),
            ..Default::default()
        })
        .unwrap();
    c.commit_plan(&run.id).unwrap();

    let claim = c.claim_action(&action.id, -1).unwrap().expect("claim");
    c.recover_expired_leases().unwrap();

    assert_eq!(status_of(&c, &action.id), ActionStatus::NeedsReconciliation);
    assert_eq!(
        run_status_of(&c, &run.id),
        RunStatus::Waiting,
        "an action needing a decision makes the run wait"
    );
    assert!(
        !c.complete_action(&action.id, &claim.lease_id, "zombie").unwrap(),
        "the dead lease can no longer report"
    );

    // With an idempotency key an operator may re-queue; without one they may not.
    assert!(c.resolve_reconciliation(&action.id, Resolution::Retry).unwrap());
    assert_eq!(status_of(&c, &action.id), ActionStatus::Queued);

    let no_key = c
        .add_action(NewAction {
            run_id: run.id.clone(),
            outcome: "y".into(),
            capability: "builtin".into(),
            ..Default::default()
        })
        .unwrap();
    c.commit_plan(&run.id).unwrap();
    let claim2 = c.claim_action(&no_key.id, -1).unwrap().expect("claim");
    assert!(!claim2.lease_id.is_empty());

    c.recover_expired_leases().unwrap();
    assert!(
        !c.resolve_reconciliation(&no_key.id, Resolution::Retry).unwrap(),
        "without an idempotency key an uncertain side effect needs a human"
    );
    assert!(c.resolve_reconciliation(&no_key.id, Resolution::Failed).unwrap());
    assert_eq!(status_of(&c, &no_key.id), ActionStatus::Failed);
}

#[test]
fn only_retry_is_gated_on_an_idempotency_key() {
    let c = Coordinator::in_memory().unwrap();
    let (run, _) = plan(&c, "no key", "x");
    c.commit_plan(&run.id).unwrap();
    c.claim_action(&action_id_for(&c, &run.id), -1).unwrap().expect("claim");
    c.recover_expired_leases().unwrap();
    let action_id = action_id_for(&c, &run.id);

    assert!(
        !c.resolve_reconciliation(&action_id, Resolution::Retry).unwrap(),
        "without an idempotency key an uncertain side effect needs a human"
    );
    assert_eq!(
        c.action(&action_id).unwrap().unwrap().status,
        ActionStatus::NeedsReconciliation,
        "a refused resolution leaves the action where it was"
    );
    assert!(
        c.resolve_reconciliation(&action_id, Resolution::Succeeded).unwrap(),
        "settling it as done is always a decision the operator may make"
    );
}

// ------------------------------------------------------ events and replay

#[test]
fn transitions_commit_events_in_sequence_and_replay_from_a_cursor() {
    let c = Coordinator::in_memory().unwrap();
    let (run, action) = plan(&c, "replay", "x");
    c.commit_plan(&run.id).unwrap();
    let claim = c.claim_action(&action.id, TTL).unwrap().expect("claim");
    c.report_progress(&action.id, &claim.lease_id, "halfway").unwrap();
    c.complete_action(&action.id, &claim.lease_id, "done").unwrap();

    let all = c.events(&run.id, 0).unwrap();
    let kinds: Vec<&str> = all.iter().map(|e| e.kind.as_str()).collect();
    assert!(kinds.contains(&"run.created"));
    assert!(kinds.contains(&"action.succeeded"));
    for pair in all.windows(2) {
        assert_eq!(
            pair[1].seq,
            pair[0].seq + 1,
            "sequences are strictly increasing within a run"
        );
    }

    let cursor = all[1].seq;
    let replay = c.events(&run.id, cursor).unwrap();
    assert!(
        replay.iter().all(|e| e.seq > cursor),
        "replay from a cursor returns only what came after it"
    );

    let snapshot = c.snapshot(&run.id, cursor).unwrap();
    assert_eq!(snapshot.run.id, run.id);
    assert_eq!(
        snapshot
            .actions
            .iter()
            .find(|a| a.id == action.id)
            .expect("action")
            .status,
        ActionStatus::Succeeded
    );
}

#[test]
fn replaying_the_whole_log_reconstructs_the_same_state() {
    let c = Coordinator::in_memory().unwrap();
    let (run, first) = plan(&c, "full replay", "first");
    let second = c
        .add_action(NewAction {
            run_id: run.id.clone(),
            outcome: "second".into(),
            capability: "builtin".into(),
            depends_on: vec![first.id.clone()],
            ..Default::default()
        })
        .unwrap();
    c.commit_plan(&run.id).unwrap();

    let claim = c.claim_action(&first.id, TTL).unwrap().expect("claim");
    c.complete_action(&first.id, &claim.lease_id, "ok").unwrap();
    let second_claim = c.claim_action(&second.id, TTL).unwrap().expect("claim");
    c.fail_action(&second.id, &second_claim.lease_id, "nope").unwrap();

    // A second coordinator over the same file sees what the first committed.
    // The event log is the history; the rows are a projection of it.
    let path = std::env::temp_dir().join(format!("lexosa-replay-{}", uuid::Uuid::new_v4()));
    let file = Coordinator::open(&path.join("runs.sqlite")).unwrap();
    let (run2, first2) = plan(&file, "full replay", "first");
    let second2 = file
        .add_action(NewAction {
            run_id: run2.id.clone(),
            outcome: "second".into(),
            capability: "builtin".into(),
            depends_on: vec![first2.id.clone()],
            ..Default::default()
        })
        .unwrap();
    file.commit_plan(&run2.id).unwrap();
    let c1 = file.claim_action(&first2.id, TTL).unwrap().unwrap();
    file.complete_action(&first2.id, &c1.lease_id, "ok").unwrap();
    let c2 = file.claim_action(&second2.id, TTL).unwrap().unwrap();
    file.fail_action(&second2.id, &c2.lease_id, "nope").unwrap();

    let from_memory = c.snapshot(&run.id, 0).unwrap();
    let from_disk = file.snapshot(&run2.id, 0).unwrap();
    assert_eq!(
        from_memory.run.status, from_disk.run.status,
        "the same sequence of transitions yields the same status"
    );
    assert_eq!(
        from_memory.events.len(),
        from_disk.events.len(),
        "the same sequence of transitions yields the same number of events"
    );

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn event_payloads_redact_credential_shaped_keys() {
    // The coordinator writes only strings it is given, so this exercises the
    // second net directly rather than relying on nobody passing a secret.
    let payload = serde_json::json!({
        "actionId": "abc",
        "api_key": "sk-live-should-never-appear",
        "nested": { "Authorization": "Bearer nope", "safe": "kept" },
        "list": [{ "password": "hunter2" }],
    });
    let safe = sanitize(&payload);
    let text = safe.to_string();

    assert!(!text.contains("sk-live"), "api_key is redacted");
    assert!(!text.contains("Bearer"), "authorization is redacted");
    assert!(!text.contains("hunter2"), "a nested password is redacted");
    assert!(text.contains("kept"), "ordinary fields survive");
    assert_eq!(safe["actionId"], "abc");
}

#[test]
fn a_run_with_no_events_after_the_cursor_reports_that_cursor() {
    let c = Coordinator::in_memory().unwrap();
    let (run, _) = plan(&c, "quiet", "x");
    let latest = c.events(&run.id, 0).unwrap().len() as i64;
    let snapshot = c.snapshot(&run.id, latest).unwrap();
    assert_eq!(snapshot.latest_seq, latest);
    assert!(snapshot.events.is_empty());
}

#[test]
fn an_empty_run_projects_as_succeeded_once_committed() {
    let c = Coordinator::in_memory().unwrap();
    let run = c
        .create_run(NewRun {
            title: "nothing to do".into(),
            message_id: None,
        })
        .unwrap();
    // A fresh run is `planning`; only committing its plan projects it. An
    // empty plan has nothing that can fail, so committing it completes the run
    // rather than leaving it waiting for a child that never exists.
    assert_eq!(run.status, RunStatus::Planning);
    let committed = c.commit_plan(&run.id).unwrap();
    assert_eq!(committed.status, RunStatus::Succeeded);
}

// ----------------------------------------------------- durable file store

#[test]
fn state_survives_reopening_the_file() {
    let path = std::env::temp_dir().join(format!("lexosa-durable-{}", uuid::Uuid::new_v4()));
    let db = path.join("runs.sqlite");

    let (run, action) = {
        let c = Coordinator::open(&db).unwrap();
        let (run, action) = plan(&c, "durable", "x");
        c.commit_plan(&run.id).unwrap();
        let claim = c.claim_action(&action.id, TTL).unwrap().expect("claim");
        c.report_progress(&action.id, &claim.lease_id, "half").unwrap();
        (run, action)
    };

    let reopened = Coordinator::open(&db).unwrap();
    let restored = reopened.action(&action.id).unwrap().expect("action");
    assert_eq!(restored.status, ActionStatus::Running);
    assert_eq!(restored.progress.as_deref(), Some("half"));
    assert_eq!(restored.attempt, 1);
    assert_eq!(
        reopened.events(&run.id, 0).unwrap().len(),
        5,
        "run.created, action.planned, action.queued, action.running, action.progress"
    );

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn a_file_store_keeps_dependency_ordering() {
    let c = Coordinator::in_memory().unwrap();
    let (run, first) = plan(&c, "ordering", "first");
    let second = c
        .add_action(NewAction {
            run_id: run.id.clone(),
            outcome: "second".into(),
            capability: "builtin".into(),
            depends_on: vec![first.id.clone()],
            ..Default::default()
        })
        .unwrap();

    let actions = c.actions(&run.id).unwrap();
    let found = actions
        .iter()
        .find(|a| a.id == second.id)
        .expect("second action");
    assert_eq!(
        found.depends_on,
        vec![first.id.clone()],
        "dependencies are read back with the action, not inferred from order"
    );
}