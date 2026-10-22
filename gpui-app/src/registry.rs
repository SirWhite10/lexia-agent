//! The capability registry: the authority on what exists.
//!
//! Every `agent.toml` grants capabilities by name, and the host refuses a name
//! it does not recognise — so the registry, not any manifest, is the authority.
//! A manifest that names a capability this list does not hold is refused with
//! that name in the error, rather than silently narrowed to the capabilities
//! that happen to exist. A silent narrowing is a manifest that says one thing
//! and means another, and nothing reports it.
//!
//! The vocabulary names the *action*, not the implementation. That is what lets
//! a builtin be replaced by a plugin later without the loop changing: the name
//! the user sees and the name the manifest grants stay the same either way.
//!
//! The list is the composer's list, plus `note.write`, which both sources
//! already agreed on. The composer and the registry are one list, not two that
//! drifted.

/// A capability the agent may be granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    /// Stream a reply back to the user.
    ChatReply,
    /// Record something as a note.
    NoteWrite,
    /// Fan work out to a sub-agent.
    SubagentSpawn,
    /// Read a file into the turn.
    FileRead,
    /// Emit an inline artifact.
    ArtifactWrite,
    /// Write a durable fact.
    MemoryWrite,
}

impl Capability {
    /// Every capability that exists.
    pub const ALL: [Capability; 6] = [
        Capability::ChatReply,
        Capability::NoteWrite,
        Capability::SubagentSpawn,
        Capability::FileRead,
        Capability::ArtifactWrite,
        Capability::MemoryWrite,
    ];

    /// The stable wire name. It appears in manifests, in the run ledger and in
    /// the composer, so it is never derived from anything that can change.
    pub fn name(self) -> &'static str {
        match self {
            Capability::ChatReply => "chat.reply",
            Capability::NoteWrite => "note.write",
            Capability::SubagentSpawn => "subagent.spawn",
            Capability::FileRead => "file.read",
            Capability::ArtifactWrite => "artifact.write",
            Capability::MemoryWrite => "memory.write",
        }
    }

    /// What the capability does, in one line. The composer shows this beside
    /// the name, so it is product surface rather than a comment.
    pub fn meaning(self) -> &'static str {
        match self {
            Capability::ChatReply => "Stream a reply back to the user.",
            Capability::NoteWrite => "Record something as a note.",
            Capability::SubagentSpawn => "Fan work out to a sub-agent.",
            Capability::FileRead => "Read a file into the turn.",
            Capability::ArtifactWrite => "Emit an inline artifact.",
            Capability::MemoryWrite => "Write a durable fact.",
        }
    }

    /// The capability a name refers to, or `None` when the name is not one the
    /// registry knows.
    pub fn parse(name: &str) -> Option<Self> {
        Capability::ALL.into_iter().find(|c| c.name() == name)
    }

    /// Whether a name is one the registry knows.
    pub fn is_known(name: &str) -> bool {
        Capability::parse(name).is_some()
    }

    /// The names the registry holds, in the order they are listed.
    pub fn names() -> Vec<&'static str> {
        Capability::ALL.into_iter().map(|c| c.name()).collect()
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The composer offers these, and the shipped manifest grants them. If the
    /// two drift, the composer offers a capability that cannot be granted.
    #[test]
    fn the_registry_holds_the_names_the_composer_offers() {
        let offered = [
            "chat.reply",
            "note.write",
            "subagent.spawn",
            "file.read",
            "artifact.write",
            "memory.write",
        ];
        for name in offered {
            assert!(Capability::is_known(name), "{name} is offered but not known");
        }
        assert_eq!(Capability::names().len(), offered.len());
    }

    #[test]
    fn a_name_round_trips() {
        for capability in Capability::ALL {
            assert_eq!(Capability::parse(capability.name()), Some(capability));
        }
    }

    /// The names the runner used are not the names the product uses. The
    /// composer and the registry are one list, and the old names are not in it.
    #[test]
    fn the_olds_names_are_not_capabilities() {
        assert!(!Capability::is_known("chat.ack"));
        assert!(!Capability::is_known("subagent.delegate"));
    }

    #[test]
    fn every_capability_says_what_it_does() {
        for capability in Capability::ALL {
            assert!(!capability.meaning().is_empty(), "{capability}");
        }
    }
}
