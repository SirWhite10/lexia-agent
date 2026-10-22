//! Standing sub-agents and the message log (WF-IMP-023).
//!
//! A *standing sub-agent* persists until it is explicitly retired; a one-shot
//! *agent task* dies with its run and auto-archives. This is the standing
//! side.
//!
//! Two rules run through the whole store, and both are load-bearing:
//!
//! - **A revised prompt creates a version.** It never overwrites one, so "what
//!   did it run with?" stays answerable.
//! - **Retirement archives; nothing is hard-deleted.** There is no delete
//!   path here at all, and that is deliberate: a retired sub-agent remains
//!   queryable, and a run that referenced it can still be read.

use std::sync::{Mutex, MutexGuard};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageAuthorKind {
    User,
    Agent,
    SubAgent,
}

impl MessageAuthorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MessageAuthorKind::User => "user",
            MessageAuthorKind::Agent => "agent",
            MessageAuthorKind::SubAgent => "sub_agent",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(MessageAuthorKind::User),
            "agent" => Some(MessageAuthorKind::Agent),
            "sub_agent" => Some(MessageAuthorKind::SubAgent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubAgentStatus {
    Standing,
    Retired,
}

impl SubAgentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SubAgentStatus::Standing => "standing",
            SubAgentStatus::Retired => "retired",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "standing" => Some(SubAgentStatus::Standing),
            "retired" => Some(SubAgentStatus::Retired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub author_kind: MessageAuthorKind,
    pub author_id: Option<String>,
    /// The sub-agent a message is addressed to, when it is addressed to one.
    pub addressee_id: Option<String>,
    pub run_id: Option<String>,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgent {
    pub id: String,
    pub name: String,
    pub focus: String,
    /// The memory scope this sub-agent retrieves within. A sub-agent names its
    /// own scope and never inherits the main agent's.
    pub memory_scope: String,
    /// The capabilities it may invoke, as the registry's names.
    pub capabilities: String,
    pub status: SubAgentStatus,
    pub created_at: String,
    pub archived_at: Option<String>,
}

/// A sub-agent plus the prompt version it currently runs with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentDetail {
    #[serde(flatten)]
    pub sub_agent: SubAgent,
    pub current_prompt: Option<String>,
    pub prompt_version: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentPromptVersion {
    pub sub_agent_id: String,
    pub version: i64,
    pub body: String,
    pub created_at: String,
}

/// The role card a sub-agent carries.
#[derive(Debug, Clone, Default)]
pub struct RoleCard {
    pub focus: Option<String>,
    pub memory_scope: Option<String>,
    pub capabilities: Option<String>,
}

/// The store: standing sub-agents, their prompt versions, and the message log.
pub struct SubAgentStore {
    connection: Mutex<Connection>,
}

impl SubAgentStore {
    pub fn open(database: &std::path::Path) -> Result<Self> {
        let connection = Connection::open(database)
            .map_err(|e| Error::State(format!("sub-agents: {e}")))?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA foreign_keys = ON;
                 CREATE TABLE IF NOT EXISTS messages (
                     id TEXT PRIMARY KEY,
                     author_kind TEXT NOT NULL CHECK (author_kind IN ('user', 'agent', 'sub_agent')),
                     author_id TEXT,
                     addressee_id TEXT,
                     run_id TEXT,
                     body TEXT NOT NULL,
                     created_at TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS sub_agents (
                     id TEXT PRIMARY KEY,
                     name TEXT NOT NULL,
                     focus TEXT NOT NULL DEFAULT '',
                     memory_scope TEXT NOT NULL DEFAULT '',
                     capabilities TEXT NOT NULL DEFAULT '',
                     status TEXT NOT NULL DEFAULT 'standing' CHECK (status IN ('standing', 'retired')),
                     created_at TEXT NOT NULL,
                     archived_at TEXT
                 );
                 CREATE TABLE IF NOT EXISTS sub_agent_prompt_versions (
                     sub_agent_id TEXT NOT NULL REFERENCES sub_agents(id),
                     version INTEGER NOT NULL,
                     body TEXT NOT NULL,
                     created_at TEXT NOT NULL,
                     PRIMARY KEY (sub_agent_id, version)
                 );",
            )
            .map_err(|e| Error::State(format!("sub-agents: {e}")))?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn connection(&self) -> MutexGuard<'_, Connection> {
        self.connection.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn now() -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    /// The agent authors the system prompt; the host persists it as version 1.
    pub fn create_sub_agent(
        &self,
        name: &str,
        system_prompt: &str,
        role_card: &RoleCard,
    ) -> Result<SubAgentDetail> {
        let trimmed = name.trim();
        let sub_agent = SubAgent {
            id: uuid::Uuid::new_v4().to_string(),
            name: if trimmed.is_empty() {
                "Untitled".to_string()
            } else {
                trimmed.to_string()
            },
            focus: role_card.focus.clone().unwrap_or_default().trim().to_string(),
            memory_scope: role_card
                .memory_scope
                .clone()
                .unwrap_or_default()
                .trim()
                .to_string(),
            capabilities: role_card
                .capabilities
                .clone()
                .unwrap_or_default()
                .trim()
                .to_string(),
            status: SubAgentStatus::Standing,
            created_at: Self::now(),
            archived_at: None,
        };

        {
            let mut connection = self.connection();
            let transaction = connection
                .transaction()
                .map_err(|e| Error::State(format!("could not begin the sub-agent transaction: {e}")))?;
            transaction
                .execute(
                    "INSERT INTO sub_agents (id, name, focus, memory_scope, capabilities, status, created_at, archived_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        sub_agent.id,
                        sub_agent.name,
                        sub_agent.focus,
                        sub_agent.memory_scope,
                        sub_agent.capabilities,
                        sub_agent.status.as_str(),
                        sub_agent.created_at,
                        sub_agent.archived_at,
                    ],
                )
                .map_err(sqlite)?;
            transaction
                .execute(
                    "INSERT INTO sub_agent_prompt_versions (sub_agent_id, version, body, created_at)
                     VALUES (?1, 1, ?2, ?3)",
                    params![sub_agent.id, system_prompt, Self::now()],
                )
                .map_err(sqlite)?;
            transaction
                .commit()
                .map_err(|e| Error::State(format!("could not commit the sub-agent: {e}")))?;
        }

        Ok(SubAgentDetail {
            sub_agent,
            current_prompt: Some(system_prompt.to_string()),
            prompt_version: Some(1),
        })
    }

    /// The sub-agents that are still standing.
    pub fn list_standing_sub_agents(&self) -> Result<Vec<SubAgent>> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(
                "SELECT id, name, focus, memory_scope, capabilities, status, created_at, archived_at
                 FROM sub_agents WHERE status = 'standing' ORDER BY created_at DESC",
            )
            .map_err(sqlite)?;
        let rows = statement
            .query_map([], read_sub_agent)
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        Ok(rows)
    }

    /// Every sub-agent, standing or retired. A retired one is still here: it
    /// is archived, not deleted.
    pub fn list_sub_agents(&self) -> Result<Vec<SubAgent>> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(
                "SELECT id, name, focus, memory_scope, capabilities, status, created_at, archived_at
                 FROM sub_agents ORDER BY created_at DESC",
            )
            .map_err(sqlite)?;
        let rows = statement
            .query_map([], read_sub_agent)
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        Ok(rows)
    }

    pub fn get_sub_agent(&self, id: &str) -> Result<Option<SubAgentDetail>> {
        let connection = self.connection();
        let sub_agent: Option<SubAgent> = connection
            .query_row(
                "SELECT id, name, focus, memory_scope, capabilities, status, created_at, archived_at
                 FROM sub_agents WHERE id = ?1",
                params![id],
                read_sub_agent,
            )
            .optional()
            .map_err(sqlite)?;
        let Some(sub_agent) = sub_agent else {
            return Ok(None);
        };
        let prompt: Option<(i64, String)> = connection
            .query_row(
                "SELECT version, body FROM sub_agent_prompt_versions WHERE sub_agent_id = ?1
                 ORDER BY version DESC LIMIT 1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(sqlite)?;
        Ok(Some(SubAgentDetail {
            sub_agent,
            current_prompt: prompt.as_ref().map(|(_, body)| body.clone()),
            prompt_version: prompt.map(|(version, _)| version),
        }))
    }

    /// Every version of the prompt, oldest first.
    pub fn list_sub_agent_prompt_versions(&self, id: &str) -> Result<Vec<SubAgentPromptVersion>> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(
                "SELECT sub_agent_id, version, body, created_at FROM sub_agent_prompt_versions
                 WHERE sub_agent_id = ?1 ORDER BY version ASC",
            )
            .map_err(sqlite)?;
        let rows = statement
            .query_map(params![id], |row| {
                Ok(SubAgentPromptVersion {
                    sub_agent_id: row.get(0)?,
                    version: row.get(1)?,
                    body: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        Ok(rows)
    }

    pub fn rename_sub_agent(&self, id: &str, name: &str) -> Result<()> {
        let trimmed = name.trim();
        let name = if trimmed.is_empty() {
            "Untitled"
        } else {
            trimmed
        };
        self.connection()
            .execute(
                "UPDATE sub_agents SET name = ?2 WHERE id = ?1",
                params![id, name],
            )
            .map_err(sqlite)?;
        Ok(())
    }

    /// Update the role card. A field that is not given is left alone.
    pub fn update_sub_agent_role_card(&self, id: &str, role_card: &RoleCard) -> Result<()> {
        let mut connection = self.connection();
        if let Some(focus) = &role_card.focus {
            connection
                .execute(
                    "UPDATE sub_agents SET focus = ?2 WHERE id = ?1",
                    params![id, focus.trim()],
                )
                .map_err(sqlite)?;
        }
        if let Some(scope) = &role_card.memory_scope {
            connection
                .execute(
                    "UPDATE sub_agents SET memory_scope = ?2 WHERE id = ?1",
                    params![id, scope.trim()],
                )
                .map_err(sqlite)?;
        }
        if let Some(capabilities) = &role_card.capabilities {
            connection
                .execute(
                    "UPDATE sub_agents SET capabilities = ?2 WHERE id = ?1",
                    params![id, capabilities.trim()],
                )
                .map_err(sqlite)?;
        }
        Ok(())
    }

    /// Retirement is a status, not a deletion. The sub-agent stays here.
    pub fn retire_sub_agent(&self, id: &str) -> Result<()> {
        self.connection()
            .execute(
                "UPDATE sub_agents SET status = 'retired' WHERE id = ?1",
                params![id],
            )
            .map_err(sqlite)?;
        Ok(())
    }

    /// Archive, never delete: retired sub-agents stay queryable.
    pub fn archive_sub_agent(&self, id: &str) -> Result<()> {
        self.connection()
            .execute(
                "UPDATE sub_agents SET archived_at = ?2 WHERE id = ?1 AND archived_at IS NULL",
                params![id, Self::now()],
            )
            .map_err(sqlite)?;
        Ok(())
    }

    /// Revisions never overwrite: "what did it run with?" must stay answerable.
    pub fn revise_sub_agent_prompt(&self, id: &str, body: &str) -> Result<i64> {
        let mut connection = self.connection();
        let next: i64 = connection
            .query_row(
                "SELECT COALESCE(MAX(version), 0) + 1 FROM sub_agent_prompt_versions WHERE sub_agent_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(sqlite)?;
        connection
            .execute(
                "INSERT INTO sub_agent_prompt_versions (sub_agent_id, version, body, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, next, body, Self::now()],
            )
            .map_err(sqlite)?;
        Ok(next)
    }

    /// Append a message to the conversation log.
    pub fn append_message(
        &self,
        author_kind: MessageAuthorKind,
        author_id: Option<&str>,
        addressee_id: Option<&str>,
        run_id: Option<&str>,
        body: &str,
    ) -> Result<Message> {
        let message = Message {
            id: uuid::Uuid::new_v4().to_string(),
            author_kind,
            author_id: author_id.map(str::to_string),
            addressee_id: addressee_id.map(str::to_string),
            run_id: run_id.map(str::to_string),
            body: body.to_string(),
            created_at: Self::now(),
        };
        self.connection()
            .execute(
                "INSERT INTO messages (id, author_kind, author_id, addressee_id, run_id, body, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    message.id,
                    message.author_kind.as_str(),
                    message.author_id,
                    message.addressee_id,
                    message.run_id,
                    message.body,
                    message.created_at,
                ],
            )
            .map_err(sqlite)?;
        Ok(message)
    }

    /// The message log, oldest first.
    pub fn list_messages(&self) -> Result<Vec<Message>> {
        let connection = self.connection();
        let mut statement = connection
            .prepare(
                "SELECT id, author_kind, author_id, addressee_id, run_id, body, created_at
                 FROM messages ORDER BY created_at ASC",
            )
            .map_err(sqlite)?;
        let rows = statement
            .query_map([], |row| {
                let kind: String = row.get(1)?;
                Ok(Message {
                    id: row.get(0)?,
                    author_kind: MessageAuthorKind::parse(&kind).unwrap_or(MessageAuthorKind::Agent),
                    author_id: row.get(2)?,
                    addressee_id: row.get(3)?,
                    run_id: row.get(4)?,
                    body: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        Ok(rows)
    }

    /// The last few messages as a transcript, the way the reflection reads it.
    pub fn transcript(&self, last: usize) -> Result<String> {
        let messages = self.list_messages()?;
        let start = messages.len().saturating_sub(last);
        Ok(messages[start..]
            .iter()
            .map(|message| format!("{}: {}", message.author_kind.as_str(), message.body))
            .collect::<Vec<String>>()
            .join("\n"))
    }
}

fn sqlite(error: rusqlite::Error) -> Error {
    Error::State(format!("sub-agents: {error}"))
}

fn read_sub_agent(row: &rusqlite::Row<'_>) -> rusqlite::Result<SubAgent> {
    let status: String = row.get(5)?;
    Ok(SubAgent {
        id: row.get(0)?,
        name: row.get(1)?,
        focus: row.get(2)?,
        memory_scope: row.get(3)?,
        capabilities: row.get(4)?,
        status: SubAgentStatus::parse(&status).unwrap_or(SubAgentStatus::Standing),
        created_at: row.get(6)?,
        archived_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch {
        root: std::path::PathBuf,
    }

    impl Scratch {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "agent-subagents-test-{name}-{}",
                uuid::Uuid::new_v4()
            ));
            std::fs::create_dir_all(&root).expect("the directory");
            Self { root }
        }

        fn store(&self) -> SubAgentStore {
            SubAgentStore::open(&self.root.join("agents.sqlite")).expect("the store opens")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn role_card() -> RoleCard {
        RoleCard {
            focus: Some("Research prices".into()),
            memory_scope: Some("research".into()),
            capabilities: Some("chat.reply".into()),
        }
    }

    /// A revised prompt creates a new version, and the prior versions stay
    /// viewable.
    #[test]
    fn a_revised_prompt_creates_a_version_and_keeps_the_others() {
        let scratch = Scratch::new("versions");
        let store = scratch.store();
        let created = store
            .create_sub_agent("Researcher", "You research prices.", &role_card())
            .expect("the sub-agent is created");
        assert_eq!(created.prompt_version, Some(1));
        assert_eq!(created.current_prompt.as_deref(), Some("You research prices."));

        let version = store
            .revise_sub_agent_prompt(&created.sub_agent.id, "You research prices, carefully.")
            .expect("the revision is recorded");
        assert_eq!(version, 2);

        let versions = store
            .list_sub_agent_prompt_versions(&created.sub_agent.id)
            .expect("the versions are listed");
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].body, "You research prices.");
        assert_eq!(versions[1].body, "You research prices, carefully.");

        // And the sub-agent runs with the new one.
        let current = store
            .get_sub_agent(&created.sub_agent.id)
            .expect("the sub-agent is read")
            .expect("it is there");
        assert_eq!(current.prompt_version, Some(2));
        assert_eq!(
            current.current_prompt.as_deref(),
            Some("You research prices, carefully.")
        );
    }

    /// A retired sub-agent is archived and remains queryable. There is no
    /// destructive delete path at all.
    #[test]
    fn a_retired_sub_agent_is_archived_and_still_queryable() {
        let scratch = Scratch::new("retire");
        let store = scratch.store();
        let created = store
            .create_sub_agent("Researcher", "You research prices.", &role_card())
            .expect("the sub-agent is created");

        store
            .retire_sub_agent(&created.sub_agent.id)
            .expect("the sub-agent is retired");
        store
            .archive_sub_agent(&created.sub_agent.id)
            .expect("the sub-agent is archived");

        assert!(
            store
                .list_standing_sub_agents()
                .expect("the standing sub-agents are listed")
                .is_empty(),
            "a retired sub-agent is not standing"
        );

        // It is still there.
        let all = store.list_sub_agents().expect("the sub-agents are listed");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, SubAgentStatus::Retired);
        assert!(all[0].archived_at.is_some(), "and it is archived");

        let detail = store
            .get_sub_agent(&created.sub_agent.id)
            .expect("the sub-agent is read")
            .expect("it is still queryable");
        assert_eq!(detail.sub_agent.status, SubAgentStatus::Retired);
    }

    #[test]
    fn a_sub_agent_carries_its_role_card() {
        let scratch = Scratch::new("role-card");
        let store = scratch.store();
        let created = store
            .create_sub_agent("Researcher", "You research prices.", &role_card())
            .expect("the sub-agent is created");
        assert_eq!(created.sub_agent.focus, "Research prices");
        assert_eq!(created.sub_agent.memory_scope, "research");
        assert_eq!(created.sub_agent.capabilities, "chat.reply");

        store
            .update_sub_agent_role_card(
                &created.sub_agent.id,
                &RoleCard {
                    focus: Some("Research suppliers".into()),
                    ..Default::default()
                },
            )
            .expect("the role card is updated");
        let updated = store
            .get_sub_agent(&created.sub_agent.id)
            .expect("the sub-agent is read")
            .expect("it is there");
        assert_eq!(updated.sub_agent.focus, "Research suppliers.");
        assert_eq!(
            updated.sub_agent.memory_scope, "research",
            "a field that was not given is left alone"
        );
    }

    /// The reflection reads the conversation as a transcript.
    #[test]
    fn the_message_log_reads_as_a_transcript() {
        let scratch = Scratch::new("messages");
        let store = scratch.store();
        store
            .append_message(MessageAuthorKind::User, None, None, None, "turn the lights off")
            .expect("the message is appended");
        store
            .append_message(
                MessageAuthorKind::SubAgent,
                Some("sub-1"),
                None,
                Some("run-1"),
                "the lights are off",
            )
            .expect("the message is appended");
        let transcript = store.transcript(10).expect("the transcript is read");
        assert!(transcript.contains("user: turn the lights off"), "{transcript}");
        assert!(transcript.contains("sub_agent: the lights are off"), "{transcript}");
    }
}
