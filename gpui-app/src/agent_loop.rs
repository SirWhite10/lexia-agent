//! The agent loop (WF-IMP-023).
//!
//! Turns a decision into a real run graph, and runs the actions that graph
//! holds. The shape is the one the TypeScript loop has, and it is the shape
//! that matters: a reply action, a delegation behind it when the decision
//! asked for one, and a note behind the reply. Everything below this module —
//! the coordinator, the ledger, the clients — reads the run, not the decision.
//!
//! Two things are deliberate here:
//!
//! - **A capability the host cannot run is left queued.** It is not failed.
//!   The wake service, a plugin worker or a sub-agent may claim it later, and
//!   failing it would destroy work that is only waiting.
//! - **A dispatch pass sweeps until nothing more executes.** Completing an
//!   action can unblock a dependent that this pass has already visited, so a
//!   single pass would leave a run half done.

use std::path::Path;

use crate::builtins::{builtin_for, BuiltinContext, BuiltinOutcome};
use crate::error::Result;
use crate::registry::Capability;
use crate::router::TurnPlan;
use crate::runs::{Action, Coordinator, ExecutionMode, NewAction, NewRun, Run};
use crate::subagents::{RoleCard, SubAgentStore};

/// How long a claimed action's lease lasts.
const LEASE_TTL_MS: i64 = 120_000;

/// The run graph a turn produced.
#[derive(Debug, Clone)]
pub struct PlannedRun {
    pub run: Run,
    /// The actions, in the order they were planned.
    pub actions: Vec<Action>,
}

/// Build the run graph behind a turn.
///
/// The reply is the point of the turn, so it is planned first and the work
/// that depends on it is planned behind it. A delegation is planned before the
/// reply is claimed, so the sub-agent exists by the time the user sees the run.
pub fn plan_run(
    coordinator: &Coordinator,
    sub_agents: &SubAgentStore,
    plan: &TurnPlan,
    title: &str,
    message_id: Option<&str>,
) -> Result<PlannedRun> {
    let run = coordinator.create_run(NewRun {
        title: title.to_string(),
        message_id: message_id.map(str::to_string),
    })?;

    let reply = coordinator.add_action(NewAction {
        run_id: run.id.clone(),
        outcome: "Reply to the request".to_string(),
        capability: Capability::ChatReply.name().to_string(),
        execution_mode: Some(ExecutionMode::Immediate),
        ..Default::default()
    })?;

    let mut actions = vec![reply.clone()];

    if let Some(delegation) = &plan.subagent {
        let sub_agent = sub_agents.create_sub_agent(
            &delegation.name,
            &delegation.system_prompt,
            &RoleCard {
                focus: Some(delegation.focus.clone()),
                capabilities: Some(Capability::ChatReply.name().to_string()),
                ..Default::default()
            },
        )?;
        let spawned = coordinator.add_action(NewAction {
            run_id: run.id.clone(),
            outcome: format!("Delegate to the {}", sub_agent.sub_agent.name),
            capability: Capability::SubagentSpawn.name().to_string(),
            execution_mode: Some(ExecutionMode::Immediate),
            depends_on: vec![reply.id.clone()],
            // Seeded now so the capability knows which role card to run and what
            // for, without the run graph having to carry a second channel.
            input: Some(format!(
                "{{\"subAgentId\":\"{}\",\"task\":\"{}\"}}",
                sub_agent.sub_agent.id, delegation.task
            )),
            ..Default::default()
        })?;
        actions.push(spawned);
    }

    let note = coordinator.add_action(NewAction {
        run_id: run.id.clone(),
        outcome: "Record the request as a note".to_string(),
        capability: Capability::NoteWrite.name().to_string(),
        execution_mode: Some(ExecutionMode::Background),
        depends_on: vec![reply.id.clone()],
        ..Default::default()
    })?;
    actions.push(note);

    coordinator.commit_plan(&run.id)?;
    Ok(PlannedRun {
        run,
        actions,
    })
}

/// What one dispatch did.
#[derive(Debug, Clone, Default)]
pub struct DispatchReport {
    /// The actions that ran.
    pub executed: Vec<String>,
    /// The actions that failed. The reason is on the action itself.
    pub failed: Vec<String>,
    /// The actions left queued, because this host cannot run them.
    pub skipped: Vec<String>,
}

/// Claim every queued action this host can run and run it under a lease.
///
/// Failures are recorded on the action, never thrown into the void: an
/// unhandled error must not erase a run.
pub async fn dispatch_queued_actions(
    coordinator: &Coordinator,
    state_dir: &Path,
    run_id: &str,
) -> Result<DispatchReport> {
    let mut report = DispatchReport::default();
    // Completing an action can unblock a dependent into `queued` after this
    // pass has already visited it, so keep sweeping until a pass executes
    // nothing. A capability with no builtin is left queued and stops the loop
    // naturally.
    loop {
        let mut executed_this_pass = 0;
        for action in coordinator.actions(run_id)? {
            if action.status != crate::runs::ActionStatus::Queued {
                continue;
            }
            let Some(capability) = Capability::parse(&action.capability) else {
                // A capability the registry does not know cannot be run by
                // anything. It stays queued rather than being failed: a
                // capability the host does not provide may be a plugin's.
                report.skipped.push(action.id.clone());
                continue;
            };
            let Some(builtin) = builtin_for(capability) else {
                report.skipped.push(action.id.clone());
                continue;
            };
            let Some(lease) = coordinator.claim_action(&action.id, LEASE_TTL_MS)? else {
                report.skipped.push(action.id.clone());
                continue;
            };
            let context = BuiltinContext {
                action: lease.action.clone(),
                // The action's seeded result is what the capability runs on.
                input: lease.action.result.clone(),
                state_dir: state_dir.to_path_buf(),
            };
            match builtin.execute(&context).await {
                Ok(BuiltinOutcome { progress, result }) => {
                    // Renew once so a slow builtin never trips its own lease.
                    coordinator.renew_lease(&action.id, &lease.lease_id, LEASE_TTL_MS)?;
                    for step in progress {
                        coordinator.report_progress(&action.id, &lease.lease_id, &step)?;
                    }
                    coordinator.complete_action(&action.id, &lease.lease_id, &result)?;
                    report.executed.push(action.id.clone());
                    executed_this_pass += 1;
                }
                Err(error) => {
                    coordinator.fail_action(&action.id, &lease.lease_id, &error.to_string())?;
                    report.failed.push(action.id.clone());
                }
            }
        }
        if executed_this_pass == 0 {
            return Ok(report);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runs::ActionStatus;

    struct Scratch {
        root: std::path::PathBuf,
    }

    impl Scratch {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "agent-loop-test-{name}-{}",
                uuid::Uuid::new_v4()
            ));
            std::fs::create_dir_all(&root).expect("the directory");
            Self { root }
        }

        fn coordinator(&self) -> Coordinator {
            Coordinator::open(&self.root.join("runs.sqlite")).expect("the coordinator opens")
        }

        fn sub_agents(&self) -> SubAgentStore {
            SubAgentStore::open(&self.root.join("agents.sqlite")).expect("the store opens")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    /// A turn produces a run whose actions the coordinator tracks: the reply,
    /// the note behind it, and the delegation when the decision asked for one.
    #[test]
    fn a_turn_produces_a_run_whose_actions_the_coordinator_tracks() {
        let scratch = Scratch::new("plan");
        let coordinator = scratch.coordinator();
        let sub_agents = scratch.sub_agents();
        let plan = TurnPlan {
            subagent: None,
        };

        let planned = plan_run(
            &coordinator,
            &sub_agents,
            &plan,
            "turn the lights off",
            Some("message-1"),
        )
        .expect("the run is planned");

        assert_eq!(planned.actions.len(), 2);
        let tracked = coordinator.actions(&planned.run.id).expect("the actions are read");
        assert_eq!(tracked.len(), 2);

        let reply = tracked
            .iter()
            .find(|action| action.capability == "chat.reply")
            .expect("the reply is planned");
        assert_eq!(reply.status, ActionStatus::Queued);
        assert_eq!(reply.execution_mode, ExecutionMode::Immediate);

        let note = tracked
            .iter()
            .find(|action| action.capability == "note.write")
            .expect("the note is planned");
        assert_eq!(note.depends_on, vec![reply.id.clone()]);
        assert_eq!(note.execution_mode, ExecutionMode::Background);
    }

    /// A delegation is planned before the reply is claimed, so the sub-agent
    /// exists by the time the user sees the run.
    #[test]
    fn a_delegation_is_planned_with_its_role_card() {
        let scratch = Scratch::new("delegation");
        let coordinator = scratch.coordinator();
        let sub_agents = scratch.sub_agents();
        let plan = crate::router::plan_turn("delegate this to a researcher and have them find the Q3 figures");

        let planned = plan_run(&coordinator, &sub_agents, &plan, "find the figures", None)
            .expect("the run is planned");

        let tracked = coordinator.actions(&planned.run.id).expect("the actions are read");
        assert_eq!(tracked.len(), 3);
        let spawn = tracked
            .iter()
            .find(|action| action.capability == "subagent.spawn")
            .expect("the delegation is planned");
        assert_eq!(spawn.depends_on.len(), 1);

        // The role card exists, and the action carries what to run and for.
        let standing = sub_agents.list_standing_sub_agents().expect("the sub-agents are listed");
        assert_eq!(standing.len(), 1);
        assert_eq!(standing[0].name, "Researcher");
        let input = spawn.result.clone().expect("the action is seeded");
        assert!(input.contains(&standing[0].id), "{input}");
        assert!(input.contains("Q3"), "{input}");
    }

    /// A dependent action runs only after its dependency succeeds.
    #[tokio::test]
    async fn a_dependent_action_runs_only_after_its_dependency_succeeds() {
        let scratch = Scratch::new("dependency");
        let coordinator = scratch.coordinator();
        let sub_agents = scratch.sub_agents();
        let plan = TurnPlan { subagent: None };
        let planned = plan_run(&coordinator, &sub_agents, &plan, "a turn", None).expect("planned");

        let reply = planned.actions[0].id.clone();
        let note = planned.actions[1].id.clone();

        // The note depends on the reply, so it is not claimable yet.
        let claimed = coordinator.claim_action(&note, LEASE_TTL_MS).expect("claim");
        assert!(claimed.is_none(), "the note waits for the reply");

        // The reply runs, and the note is unblocked.
        let lease = coordinator
            .claim_action(&reply, LEASE_TTL_MS)
            .expect("claim")
            .expect("the reply is claimable");
        coordinator
            .complete_action(&reply, &lease.lease_id, "the lights are off")
            .expect("the reply completes");
        let claimed = coordinator.claim_action(&note, LEASE_TTL_MS).expect("claim");
        assert!(claimed.is_some(), "the note is unblocked by the reply");
    }

    /// The dispatch runs the queued actions the host can run, and leaves the
    /// ones it cannot.
    #[tokio::test]
    async fn the_dispatch_runs_what_it_can_and_leaves_the_rest() {
        let scratch = Scratch::new("dispatch");
        let coordinator = scratch.coordinator();
        let sub_agents = scratch.sub_agents();
        let plan = TurnPlan { subagent: None };
        let planned = plan_run(&coordinator, &sub_agents, &plan, "a turn", None).expect("planned");
        let reply = planned.actions[0].id.clone();
        let note = planned.actions[1].id.clone();

        // The reply is completed by the model, not by a builtin.
        let lease = coordinator
            .claim_action(&reply, LEASE_TTL_MS)
            .expect("claim")
            .expect("the reply is claimable");
        coordinator
            .complete_action(&reply, &lease.lease_id, "the lights are off")
            .expect("the reply completes");

        let report = dispatch_queued_actions(&coordinator, &scratch.root, &planned.run.id)
            .await
            .expect("the dispatch runs");
        assert_eq!(report.executed, vec![note.clone()]);
        assert!(report.failed.is_empty());
        assert!(report.skipped.is_empty());

        // The note really is a file.
        let written = std::fs::read_to_string(scratch.root.join("notes").join(format!("{note}.md")))
            .expect("the note file exists");
        assert!(written.contains("Record the request as a note"), "{written}");

        let tracked = coordinator.actions(&planned.run.id).expect("the actions are read");
        let completed = tracked
            .iter()
            .find(|action| action.id == note)
            .expect("the note is tracked");
        assert_eq!(completed.status, ActionStatus::Succeeded);
    }

    /// A capability the host cannot run is left queued, not failed.
    #[tokio::test]
    async fn a_capability_the_host_cannot_run_is_left_queued() {
        let scratch = Scratch::new("skipped");
        let coordinator = scratch.coordinator();
        let sub_agents = scratch.sub_agents();
        let run = coordinator
            .create_run(NewRun {
                title: "a turn".into(),
                message_id: None,
            })
            .expect("the run");
        // A capability the host provides no builtin for.
        let action = coordinator
            .add_action(NewAction {
                run_id: run.id.clone(),
                outcome: "Write a fact".into(),
                capability: "memory.write".into(),
                ..Default::default()
            })
            .expect("the action");
        coordinator.commit_plan(&run.id).expect("the plan is committed");

        let report = dispatch_queued_actions(&coordinator, &scratch.root, &run.id)
            .await
            .expect("the dispatch runs");
        assert_eq!(report.skipped, vec![action.id.clone()]);
        assert!(report.executed.is_empty());
        assert!(report.failed.is_empty());

        let tracked = coordinator.actions(&run.id).expect("the actions are read");
        assert_eq!(
            tracked[0].status,
            ActionStatus::Queued,
            "an action this host cannot run stays queued"
        );
    }
}
