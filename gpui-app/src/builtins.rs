//! The builtin capabilities (WF-IMP-023).
//!
//! Builtins are what the product needs whether or not plugins ever arrive. They
//! run in-process, and a plugin can be put behind the same capability name
//! later without the loop changing — which is the whole point of naming the
//! action rather than the implementation.
//!
//! Every builtin here is a real local side effect, not a stub. They are
//! deliberately boring so the execution contract can be proved without
//! coupling to a provider:
//!
//! - `note.write` appends a file under the state directory.
//! - `chat.reply` records the reply, so it can be read back afterwards.
//!
//! The names are the registry's, not the runner's old ones. `chat.ack` and
//! `subagent.delegate` were descriptions of what the code did; the product
//! surface says `chat.reply` and `subagent.spawn`.

use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::registry::Capability;
use crate::runs::Action;

/// What running a capability produced.
#[derive(Debug, Clone, Default)]
pub struct BuiltinOutcome {
    /// Lines shown while the action runs.
    pub progress: Vec<String>,
    /// The result the run ledger records.
    pub result: String,
}

/// What a builtin is given.
pub struct BuiltinContext {
    /// The action being carried out.
    pub action: Action,
    /// The input the action was given, when it was given one.
    pub input: Option<String>,
    /// Where this host keeps what it writes.
    pub state_dir: PathBuf,
}

impl BuiltinContext {
    /// The directory notes are written to.
    pub fn notes_dir(&self) -> PathBuf {
        self.state_dir.join("notes")
    }

    /// The file replies are recorded in.
    pub fn replies_path(&self) -> PathBuf {
        self.state_dir.join("replies.md")
    }
}

/// A capability the host itself provides.
#[async_trait]
pub trait Builtin: Send + Sync {
    /// The capability this builtin provides.
    fn capability(&self) -> Capability;
    /// The sentence shown in the chat while this action runs.
    fn describe(&self, action: &Action) -> String;
    async fn execute(&self, context: &BuiltinContext) -> Result<BuiltinOutcome>;
}

/// Record a note. The file is the side effect: something a person can read.
pub struct NoteWrite;

#[async_trait]
impl Builtin for NoteWrite {
    fn capability(&self) -> Capability {
        Capability::NoteWrite
    }

    fn describe(&self, action: &Action) -> String {
        action.outcome.clone()
    }

    async fn execute(&self, context: &BuiltinContext) -> Result<BuiltinOutcome> {
        let directory = context.notes_dir();
        std::fs::create_dir_all(&directory)
            .map_err(|e| Error::State(format!("could not create the notes directory: {e}")))?;
        let file = directory.join(format!("{}.md", context.action.id));
        let body = format!(
            "# {}\n\n{}\n",
            context.action.outcome,
            context.input.clone().unwrap_or_default()
        );
        std::fs::write(&file, body)
            .map_err(|e| Error::State(format!("could not write the note: {e}")))?;
        Ok(BuiltinOutcome {
            progress: Vec::new(),
            result: format!("note.write completed for action {}", context.action.id),
        })
    }
}

/// Record a reply, so the conversation has something to show afterwards.
pub struct ChatReply;

#[async_trait]
impl Builtin for ChatReply {
    fn capability(&self) -> Capability {
        Capability::ChatReply
    }

    fn describe(&self, action: &Action) -> String {
        action.outcome.clone()
    }

    async fn execute(&self, context: &BuiltinContext) -> Result<BuiltinOutcome> {
        let reply = context.input.clone().unwrap_or_default();
        if let Some(parent) = context.replies_path().parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::State(format!("could not create the state directory: {e}")))?;
        }
        let mut existing = std::fs::read_to_string(context.replies_path()).unwrap_or_default();
        existing.push_str(&format!("- [{}] {reply}\n", context.action.id));
        std::fs::write(context.replies_path(), existing)
            .map_err(|e| Error::State(format!("could not record the reply: {e}")))?;
        Ok(BuiltinOutcome {
            progress: vec!["acknowledged".to_string(), "recorded".to_string()],
            result: format!("chat.reply recorded for action {}", context.action.id),
        })
    }
}

/// The builtin for a capability, if the host provides one.
///
/// A capability the host does not provide is not an error here: it may be a
/// plugin's, and the loop resolves the name against both.
pub fn builtin_for(capability: Capability) -> Option<Box<dyn Builtin>> {
    match capability {
        Capability::NoteWrite => Some(Box::new(NoteWrite)),
        Capability::ChatReply => Some(Box::new(ChatReply)),
        _ => None,
    }
}

/// The capabilities the host provides itself.
pub fn builtin_capabilities() -> Vec<Capability> {
    Capability::ALL
        .into_iter()
        .filter(|capability| builtin_for(*capability).is_some())
        .collect()
}

/// Whether a state directory can be written to. A host that cannot write its
/// own state cannot run a builtin, and saying so is better than a note that
/// silently did not land.
pub fn state_is_writable(state_dir: &Path) -> bool {
    std::fs::create_dir_all(state_dir).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runs::{Coordinator, NewAction, NewRun};

    /// A real action, from a real run. A hand-built one would carry fields
    /// the coordinator never sets, and the builtin would be proved against a
    /// shape that does not exist.
    fn action(capability: &str, outcome: &str) -> Action {
        let coordinator = Coordinator::in_memory().expect("the coordinator");
        let run = coordinator
            .create_run(NewRun {
                title: "a turn".into(),
                message_id: None,
            })
            .expect("the run");
        coordinator
            .add_action(NewAction {
                run_id: run.id.clone(),
                outcome: outcome.into(),
                capability: capability.into(),
                ..Default::default()
            })
            .expect("the action")
    }

    fn context(root: &Path, input: Option<&str>) -> BuiltinContext {
        BuiltinContext {
            action: action("note.write", "Record the request as a note"),
            input: input.map(str::to_string),
            state_dir: root.to_path_buf(),
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "agent-builtins-test-{name}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("the directory");
        root
    }

    #[tokio::test]
    async fn note_write_writes_a_file_a_person_can_read() {
        let root = scratch("note");
        let builtin = NoteWrite;
        let context = context(&root, Some("the bedroom lights"));
        let id = context.action.id.clone();
        let outcome = builtin.execute(&context).await.expect("the note is written");
        assert!(outcome.result.contains(&id), "{}", outcome.result);

        let written =
            std::fs::read_to_string(root.join("notes").join(format!("{id}.md")))
                .expect("the note file exists");
        assert!(written.contains("Record the request as a note"), "{written}");
        assert!(written.contains("the bedroom lights"), "{written}");
    }

    #[tokio::test]
    async fn chat_reply_records_the_reply() {
        let root = scratch("reply");
        let builtin = ChatReply;
        let context = context(&root, Some("the lights are off"));
        let id = context.action.id.clone();
        let outcome = builtin.execute(&context).await.expect("the reply is recorded");
        assert_eq!(outcome.progress, vec!["acknowledged", "recorded"]);

        let recorded = std::fs::read_to_string(root.join("replies.md")).expect("the replies file");
        assert!(recorded.contains("the lights are off"), "{recorded}");
        assert!(recorded.contains(&id), "{recorded}");
    }

    #[tokio::test]
    async fn a_reply_is_appended_rather_than_replacing_the_last_one() {
        let root = scratch("append");
        let builtin = ChatReply;
        builtin
            .execute(&context(&root, Some("first")))
            .await
            .expect("the first reply is recorded");
        builtin
            .execute(&context(&root, Some("second")))
            .await
            .expect("the second reply is recorded");
        let recorded = std::fs::read_to_string(root.join("replies.md")).expect("the replies file");
        assert!(recorded.contains("first"), "{recorded}");
        assert!(recorded.contains("second"), "{recorded}");
    }

    /// The host provides what the product needs regardless of plugins. The
    /// rest may be a plugin's, and the loop resolves the name against both.
    #[test]
    fn the_host_provides_the_capabilities_the_product_needs() {
        let provided = builtin_capabilities();
        assert!(provided.contains(&Capability::NoteWrite));
        assert!(provided.contains(&Capability::ChatReply));
        assert!(!provided.contains(&Capability::MemoryWrite));
    }

    #[test]
    fn a_capability_the_host_does_not_provide_resolves_to_nothing() {
        assert!(builtin_for(Capability::MemoryWrite).is_none());
        assert!(builtin_for(Capability::SubagentSpawn).is_none());
    }
}
