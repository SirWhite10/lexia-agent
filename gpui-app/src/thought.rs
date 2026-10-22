//! The thought tree (WF-IMP-022).
//!
//! How the agent's routing reads: a question asked at the root, the answers it
//! admits, and the branch each answer opens. It is editable by the operator in
//! the UI and, once the runtime router exists, by the agent itself — which is
//! why it is plain rows with stable ids rather than an opaque document.
//!
//! A fresh workspace gets the default pipeline rather than an empty canvas: a
//! blank tree teaches nothing about what a node is for. The default is *added*,
//! never imposed: an operator's own edits and re-parenting survive, because
//! the pipeline is identified by what it is called rather than by an id that
//! would not survive a move between installations.

use std::sync::{Mutex, MutexGuard};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[cfg(test)]
mod tests;

/// Node columns are fixed-width on screen; the canvas snaps to this so a tree
/// stays readable after the operator has dragged things around.
pub const GRID_SIZE: f64 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThoughtNodeKind {
    Question,
    Route,
    Action,
}

impl ThoughtNodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ThoughtNodeKind::Question => "question",
            ThoughtNodeKind::Route => "route",
            ThoughtNodeKind::Action => "action",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "question" => Some(ThoughtNodeKind::Question),
            "route" => Some(ThoughtNodeKind::Route),
            "action" => Some(ThoughtNodeKind::Action),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub kind: ThoughtNodeKind,
    pub question: String,
    pub options: Vec<String>,
    pub notes: String,
    pub position_x: f64,
    pub position_y: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ThoughtNodeInput {
    pub id: Option<String>,
    pub parent_id: Option<String>,
    pub title: String,
    pub kind: ThoughtNodeKind,
    pub question: String,
    pub options: Vec<String>,
    pub notes: String,
    pub position_x: f64,
    pub position_y: f64,
}

/// The thought tree's store.
pub struct ThoughtTree {
    connection: Mutex<Connection>,
}

impl ThoughtTree {
    pub fn open(database: &std::path::Path) -> Result<Self> {
        let connection = Connection::open(database)
            .map_err(|e| Error::State(format!("thought tree: {e}")))?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 CREATE TABLE IF NOT EXISTS thought_nodes (
                     id TEXT PRIMARY KEY,
                     parent_id TEXT,
                     title TEXT NOT NULL,
                     kind TEXT NOT NULL CHECK (kind IN ('question', 'route', 'action')),
                     question TEXT NOT NULL,
                     options TEXT NOT NULL,
                     notes TEXT NOT NULL,
                     position_x REAL NOT NULL,
                     position_y REAL NOT NULL,
                     updated_at TEXT NOT NULL
                 );",
            )
            .map_err(|e| Error::State(format!("thought tree: {e}")))?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn connection(&self) -> MutexGuard<'_, Connection> {
        self.connection.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Every node, in the order the canvas draws them.
    pub fn list(&self) -> Result<Vec<ThoughtNode>> {
        let connection = self.connection();
        let mut statement = connection
            .prepare("SELECT * FROM thought_nodes ORDER BY position_y, position_x")
            .map_err(sqlite)?;
        let nodes = statement
            .query_map([], read_node)
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        Ok(nodes)
    }

    /// The nodes, seeding the default pipeline when the tree is empty.
    pub fn list_or_seed(&self) -> Result<Vec<ThoughtNode>> {
        let nodes = self.list()?;
        if nodes.is_empty() {
            self.add_default_pipeline()?;
            return self.list();
        }
        Ok(nodes)
    }

    /// Save a node, creating it or replacing it.
    ///
    /// A parent that is not in the tree is refused rather than stored: a stale
    /// tab re-parenting onto a node someone else deleted would otherwise leave
    /// a branch nothing can reach from the root.
    pub fn save(&self, input: &ThoughtNodeInput) -> Result<ThoughtNode> {
        let id = input
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let connection = self.connection();
        if let Some(parent) = &input.parent_id {
            let found: Option<String> = connection
                .query_row(
                    "SELECT id FROM thought_nodes WHERE id = ?1",
                    params![parent],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sqlite)?;
            if found.is_none() {
                return Err(Error::State(
                    "that node no longer exists, so it cannot hold a branch".into(),
                ));
            }
        }

        let options: Vec<String> = input
            .options
            .iter()
            .map(|option| option.trim().to_string())
            .filter(|option| !option.is_empty())
            .collect();
        let title = {
            let trimmed = input.title.trim();
            if trimmed.is_empty() {
                "Untitled node".to_string()
            } else {
                trimmed.to_string()
            }
        };

        connection
            .execute(
                "INSERT INTO thought_nodes (id, parent_id, title, kind, question, options, notes, position_x, position_y, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                    parent_id = excluded.parent_id,
                    title = excluded.title,
                    kind = excluded.kind,
                    question = excluded.question,
                    options = excluded.options,
                    notes = excluded.notes,
                    position_x = excluded.position_x,
                    position_y = excluded.position_y,
                    updated_at = excluded.updated_at",
                params![
                    id,
                    input.parent_id,
                    title,
                    input.kind.as_str(),
                    input.question.trim(),
                    serde_json::to_string(&options).unwrap_or_else(|_| "[]".to_string()),
                    input.notes.trim(),
                    input.position_x,
                    input.position_y,
                    updated_at,
                ],
            )
            .map_err(sqlite)?;

        let node = connection
            .query_row("SELECT * FROM thought_nodes WHERE id = ?1", params![id], read_node)
            .map_err(sqlite)?;
        Ok(node)
    }

    /// Removes a node and everything below it. Routing is a tree, so a
    /// half-deleted branch would leave questions whose parent no longer exists.
    pub fn delete(&self, id: &str) -> Result<()> {
        let connection = self.connection();
        let mut ids = vec![id.to_string()];
        let mut index = 0;
        while index < ids.len() {
            let mut statement = connection
                .prepare("SELECT id FROM thought_nodes WHERE parent_id = ?1")
                .map_err(sqlite)?;
            let children: Vec<String> = statement
                .query_map(params![ids[index].clone()], |row| row.get(0))
                .map_err(sqlite)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(sqlite)?;
            drop(statement);
            ids.extend(children);
            index += 1;
        }
        for node_id in ids {
            connection
                .execute("DELETE FROM thought_nodes WHERE id = ?1", params![node_id])
                .map_err(sqlite)?;
        }
        Ok(())
    }

    /// Adds any default node the tree is missing and leaves the rest alone.
    ///
    /// Matched by title: the pipeline is identified by what it is called, not
    /// by an id that would not survive a move between installations. A tree
    /// that has nodes but no root is damaged, and adding to it would hang the
    /// new branches off nothing, so nothing is added.
    pub fn add_default_pipeline(&self) -> Result<Vec<ThoughtNode>> {
        let existing = self.list()?;
        let mut root = existing
            .iter()
            .find(|node| node.parent_id.is_none())
            .map(|node| node.id.clone());
        if root.is_none() && !existing.is_empty() {
            return Ok(Vec::new());
        }

        let mut added = Vec::new();
        for template in default_pipeline() {
            let is_root = template.title == "Take in a turn";
            let parent_id = if is_root {
                None
            } else {
                root.clone()
            };
            if existing
                .iter()
                .any(|node| node.title == template.title && node.parent_id == parent_id)
            {
                continue;
            }
            let saved = self.save(&ThoughtNodeInput {
                parent_id,
                ..template
            })?;
            if is_root {
                root = Some(saved.id.clone());
            }
            added.push(saved);
        }
        Ok(added)
    }
}

fn sqlite(error: rusqlite::Error) -> Error {
    Error::State(format!("thought tree: {error}"))
}

fn read_node(row: &rusqlite::Row<'_>) -> rusqlite::Result<ThoughtNode> {
    let kind: String = row.get("kind")?;
    let options: String = row.get("options")?;
    Ok(ThoughtNode {
        id: row.get("id")?,
        parent_id: row.get("parent_id")?,
        title: row.get("title")?,
        kind: ThoughtNodeKind::parse(&kind).unwrap_or(ThoughtNodeKind::Question),
        question: row.get("question")?,
        // A hand-edited row must not take the whole editor down.
        options: serde_json::from_str::<Vec<String>>(&options).unwrap_or_default(),
        notes: row.get("notes")?,
        position_x: row.get("position_x")?,
        position_y: row.get("position_y")?,
        updated_at: row.get("updated_at")?,
    })
}

/// The pipeline the host runs, as the default tree.
///
/// Every entry names a step that exists in code, not an aspiration: capture
/// and retrieval come from the turn loop, the slot order is the prompt's cache
/// contract, the run and its actions are built by the chat action, and
/// reflection runs on the boundary the turn loop defines.
fn default_pipeline() -> Vec<ThoughtNodeInput> {
    vec![
        ThoughtNodeInput {
            id: None,
            parent_id: None,
            title: "Take in a turn".into(),
            kind: ThoughtNodeKind::Question,
            question: "What did the user just ask for?".into(),
            options: vec![],
            notes: "The root of every turn. Everything below runs inside the chat action.".into(),
            position_x: 48.0,
            position_y: 48.0,
        },
        ThoughtNodeInput {
            id: None,
            parent_id: None,
            title: "Capture the turn".into(),
            kind: ThoughtNodeKind::Action,
            question: "Store the message and embed it for later retrieval.".into(),
            options: vec![],
            notes: "capture_turn: writes the verbatim log entry and embeds it, so the turn is retrievable on the very next request.".into(),
            position_x: 48.0,
            position_y: 192.0,
        },
        ThoughtNodeInput {
            id: None,
            parent_id: None,
            title: "Retrieve memory".into(),
            kind: ThoughtNodeKind::Action,
            question: "Which remembered facts and past turns apply to this one?".into(),
            options: vec![],
            notes: "retrieve_memory: vector search over facts and the log, budget-capped. Without a key or on a provider failure it degrades to an empty memory block rather than failing the turn.".into(),
            position_x: 336.0,
            position_y: 192.0,
        },
        ThoughtNodeInput {
            id: None,
            parent_id: None,
            title: "Assemble the prompt".into(),
            kind: ThoughtNodeKind::Action,
            question: "What does the model see before the request?".into(),
            options: vec![],
            notes: "assemble_prompt slots, in cache order: personality, then every skill in full, then retrieved memory, then the capabilities the router resolved. Everything above memory must stay byte-stable for the prefix to hit cache.".into(),
            position_x: 624.0,
            position_y: 192.0,
        },
        ThoughtNodeInput {
            id: None,
            parent_id: None,
            title: "Decide what this needs".into(),
            kind: ThoughtNodeKind::Route,
            question: "Which builtin, workflow, tool or model route answers this turn?".into(),
            options: vec![
                "Answer from the conversation".into(),
                "Write to memory or notes".into(),
                "Run a tool or workflow".into(),
                "Call a model route".into(),
            ],
            notes: "The router's classification. It is not implemented yet: the tools slot is assembled empty today, and this node is where its answer will land.".into(),
            position_x: 912.0,
            position_y: 192.0,
        },
        ThoughtNodeInput {
            id: None,
            parent_id: None,
            title: "Run the work".into(),
            kind: ThoughtNodeKind::Action,
            question: "What actually happens while the turn runs?".into(),
            options: vec![],
            notes: "One run per turn. chat.reply runs immediately and hands the turn to the agent loop; note.write runs behind it in the background. Both are leased actions, so a crash is retried rather than lost.".into(),
            position_x: 1200.0,
            position_y: 192.0,
        },
        ThoughtNodeInput {
            id: None,
            parent_id: None,
            title: "Reflect at the boundary".into(),
            kind: ThoughtNodeKind::Action,
            question: "Is this turn one where memory should be rewritten?".into(),
            options: vec![
                "Yes, distil facts and traits".into(),
                "No, leave memory alone".into(),
            ],
            notes: "maybe_reflect: every tenth user turn, counted from the persisted log, distil new facts and personality traits. A failed reflection never breaks the turn that triggered it.".into(),
            position_x: 1488.0,
            position_y: 192.0,
        },
    ]
}
