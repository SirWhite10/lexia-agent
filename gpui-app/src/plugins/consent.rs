//! Consent: deny-by-default, per agent, revocable, immediate.
//!
//! Modelled on `studio-platform`'s plugin registry, whose own module summary is
//! the rule restated:
//!
//! > "Consent is deny-by-default: nothing is granted until an explicit user
//! > decision is recorded, every grant is scoped to `(project, plugin,
//! > capability)`, and revocation takes effect immediately by deactivating active
//! > extensions."
//!
//! Three properties that are easy to state and easy to get wrong, so they are
//! enforced here rather than left to each caller:
//!
//! 1. **Absent is denied.** A capability with no recorded decision is refused.
//!    There is no "not asked yet" state that behaves like a yes.
//! 2. **`Denied` is a value.** It is distinguishable from absent, so "the user
//!    said no" is not the same record as "nobody was asked", and neither is lost.
//! 3. **Revocation is a change of state, not a flag.** The caller is handed back
//!    what to deactivate, because a grant that only takes effect at the next load
//!    is not revoked.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::capabilities::Capability;

/// One recorded decision. Both outcomes are recorded so that "refused" and
/// "undecided" do not look the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Decision {
    Granted,
    Denied,
}

/// Scopes a decision to one agent. A grant made by one agent does not follow the
/// plugin to another: a sub-agent with its own memory scope and its own grant
/// list must not inherit its parent's reach by existing on the same machine.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConsentKey {
    pub agent: String,
    pub plugin: String,
    pub capability: Capability,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConsentLedger {
    decisions: BTreeMap<ConsentKey, Decision>,
}

impl ConsentLedger {
    /// An empty ledger, in which everything is denied.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn grant(&mut self, agent: &str, plugin: &str, capability: Capability) {
        self.decisions.insert(
            ConsentKey {
                agent: agent.into(),
                plugin: plugin.into(),
                capability,
            },
            Decision::Granted,
        );
    }

    pub fn deny(&mut self, agent: &str, plugin: &str, capability: Capability) {
        self.decisions.insert(
            ConsentKey {
                agent: agent.into(),
                plugin: plugin.into(),
                capability,
            },
            Decision::Denied,
        );
    }

    /// The effective decision. Absent is `Denied` — this is the deny-by-default
    /// rule, and it is why there is no `Option` in the return type.
    pub fn decision(&self, agent: &str, plugin: &str, capability: Capability) -> Decision {
        self.decisions
            .get(&ConsentKey {
                agent: agent.into(),
                plugin: plugin.into(),
                capability,
            })
            .copied()
            .unwrap_or(Decision::Denied)
    }

    /// Whether a call may proceed. The only question the runtime asks.
    pub fn allows(&self, agent: &str, plugin: &str, capability: Capability) -> bool {
        self.decision(agent, plugin, capability) == Decision::Granted
    }

    /// Whether a decision was ever recorded, as distinct from what it was.
    pub fn was_decided(&self, agent: &str, plugin: &str, capability: Capability) -> bool {
        self.decisions.contains_key(&ConsentKey {
            agent: agent.into(),
            plugin: plugin.into(),
            capability,
        })
    }

    /// What this agent may actually call, given the ledger and what the plugin
    /// asked for. An intersection: a grant for a capability the plugin never
    /// requested grants nothing, and a request the operator never granted is
    /// refused.
    pub fn effective(&self, agent: &str, plugin: &str, requested: &[Capability]) -> Vec<Capability> {
        requested
            .iter()
            .copied()
            .filter(|capability| self.allows(agent, plugin, *capability))
            .collect()
    }

    /// Revoke everything one agent holds for one plugin.
    ///
    /// Returns the capabilities revoked so the caller can deactivate the running
    /// instance *now*. A revocation that waits for the next load is not one.
    pub fn revoke_plugin(&mut self, agent: &str, plugin: &str) -> Vec<Capability> {
        self.revoke_agent_plugin(agent, plugin, None)
    }

    /// Revoke one capability, or all of them when `capability` is `None`.
    pub fn revoke_agent_plugin(
        &mut self,
        agent: &str,
        plugin: &str,
        capability: Option<Capability>,
    ) -> Vec<Capability> {
        let revoked: BTreeSet<Capability> = self
            .decisions
            .keys()
            .filter(|key| key.agent == agent && key.plugin == plugin)
            .filter(|key| capability.is_none_or(|wanted| key.capability == wanted))
            .filter(|key| {
                self.decisions
                    .get(*key)
                    .is_some_and(|decision| *decision == Decision::Granted)
            })
            .map(|key| key.capability)
            .collect();

        for capability in &revoked {
            self.decisions.insert(
                ConsentKey {
                    agent: agent.into(),
                    plugin: plugin.into(),
                    capability: *capability,
                },
                Decision::Denied,
            );
        }
        revoked.into_iter().collect()
    }

    /// Every `(agent, plugin)` pair with at least one decision, for a screen
    /// listing what has been asked.
    pub fn pairs(&self) -> BTreeSet<(String, String)> {
        self.decisions
            .keys()
            .map(|key| (key.agent.clone(), key.plugin.clone()))
            .collect()
    }

    pub fn decisions(&self) -> impl Iterator<Item = (&ConsentKey, &Decision)> {
        self.decisions.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requested() -> Vec<Capability> {
        vec![Capability::Log, Capability::Shell, Capability::HttpGet]
    }

    #[test]
    fn an_undecided_capability_is_denied_not_pending() {
        let ledger = ConsentLedger::new();
        assert!(!ledger.allows("lexosa", "echo", Capability::Log));
        assert_eq!(
            ledger.decision("lexosa", "echo", Capability::Log),
            Decision::Denied,
            "absent means denied; there is no pending state"
        );
        assert!(
            !ledger.was_decided("lexosa", "echo", Capability::Log),
            "and it is distinguishable from a recorded refusal"
        );
    }

    #[test]
    fn a_denial_is_recorded_and_distinguishable_from_absence() {
        let mut ledger = ConsentLedger::new();
        ledger.deny("lexosa", "echo", Capability::Shell);
        assert!(ledger.was_decided("lexosa", "echo", Capability::Shell));
        assert_eq!(
            ledger.decision("lexosa", "echo", Capability::Shell),
            Decision::Denied
        );
        assert!(!ledger.was_decided("lexosa", "echo", Capability::MemoryRead));
    }

    #[test]
    fn effective_grants_are_the_intersection_of_request_and_consent() {
        let mut ledger = ConsentLedger::new();
        // Granted, but the plugin never asked for it: grants nothing.
        ledger.grant("lexosa", "echo", Capability::MemoryWrite);
        // Asked for, but never granted: refused.
        ledger.grant("lexosa", "echo", Capability::Log);

        let effective = ledger.effective("lexosa", "echo", &requested());
        assert_eq!(
            effective,
            vec![Capability::Log],
            "a grant for an unrequested capability grants nothing"
        );
        assert!(!effective.contains(&Capability::Shell));
        assert!(!effective.contains(&Capability::HttpGet));
    }

    #[test]
    fn a_grant_does_not_follow_the_plugin_to_another_agent() {
        let mut ledger = ConsentLedger::new();
        ledger.grant("lexosa", "echo", Capability::Shell);
        assert!(ledger.allows("lexosa", "echo", Capability::Shell));
        assert!(
            !ledger.allows("code-reviewer", "echo", Capability::Shell),
            "a sub-agent's reach must not be inherited by existing"
        );
    }

    #[test]
    fn revocation_reports_what_to_deactivate_immediately() {
        let mut ledger = ConsentLedger::new();
        ledger.grant("lexosa", "echo", Capability::Log);
        ledger.grant("lexosa", "echo", Capability::Shell);
        ledger.deny("lexosa", "echo", Capability::HttpGet);

        let revoked = ledger.revoke_plugin("lexosa", "echo");
        assert_eq!(
            revoked,
            vec![Capability::Log, Capability::Shell],
            "only grants are revoked; a denial was never a grant"
        );
        for capability in [Capability::Log, Capability::Shell] {
            assert!(!ledger.allows("lexosa", "echo", capability));
            assert!(ledger.was_decided("lexosa", "echo", capability));
        }
        assert!(
            ledger.allows("other", "echo", Capability::Log).eq(&false),
            "revocation is scoped to the agent that asked"
        );
    }

    #[test]
    fn revoking_one_capability_leaves_the_others() {
        let mut ledger = ConsentLedger::new();
        ledger.grant("lexosa", "echo", Capability::Log);
        ledger.grant("lexosa", "echo", Capability::Shell);

        let revoked = ledger.revoke_agent_plugin("lexosa", "echo", Some(Capability::Shell));
        assert_eq!(revoked, vec![Capability::Shell]);
        assert!(ledger.allows("lexosa", "echo", Capability::Log));
        assert!(!ledger.allows("lexosa", "echo", Capability::Shell));
    }

    #[test]
    fn a_later_grant_replaces_an_earlier_denial() {
        let mut ledger = ConsentLedger::new();
        ledger.deny("lexosa", "echo", Capability::Shell);
        assert!(!ledger.allows("lexosa", "echo", Capability::Shell));
        ledger.grant("lexosa", "echo", Capability::Shell);
        assert!(ledger.allows("lexosa", "echo", Capability::Shell));
    }
}