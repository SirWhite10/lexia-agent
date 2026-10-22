//! Durable run/action state and the append-only per-run event log.
//!
//! A port of `src/lib/server/runs.ts`, which is the specification and the parity
//! oracle ([WF-IMP-020](../../docs/planning/tickets/WF-IMP-020-durable-run-coordinator-in-rust.md),
//! ADR 0002). Three properties are the reason this is not a mechanical rewrite,
//! and all three are places where the TypeScript is weaker than it looks:
//!
//! 1. **Fenced writes are atomic.** `completeAction` in the TypeScript reads the
//!    action, checks the lease, and *then* updates — with the read outside the
//!    transaction. A second worker could pass the same check between them. Here
//!    the guard is in the statement itself (`WHERE status = 'running' AND
//!    lease_id = ?`) and the row count is the answer, so there is no window.
//! 2. **Event sequences are assigned inside the writing transaction**, not read
//!    and computed around it. A log whose sequence is assigned outside the
//!    inserting transaction can replay into a different history than the one that
//!    happened.
//! 3. **Iteration order is deterministic.** The TypeScript reconciles a plan in
//!    whatever order SQLite returns rows; `created_at, id` is used here so the
//!    same plan always produces the same event order.
//!
//! Everything else — the statuses, the projection, the gating, the recovery
//! policy, the redaction — is carried over unchanged, because it is behaviour
//! that already works and is covered by invariants.

use std::path::Path;
use std::sync::Mutex;

use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

// ------------------------------------------------------------------ statuses

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Planning,
    Active,
    Waiting,
    Succeeded,
    Failed,
    Cancelled,
    PartiallyComplete,
}

impl RunStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::Active => "active",
            Self::Waiting => "waiting",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::PartiallyComplete => "partially_complete",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "planning" => Self::Planning,
            "active" => Self::Active,
            "waiting" => Self::Waiting,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            "partially_complete" => Self::PartiallyComplete,
            _ => return None,
        })
    }

    /// A terminal run is never re-cancelled; the first terminal status is the
    /// answer and a later transition is a caller bug, not a state change.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Planned,
    WaitingForInput,
    WaitingForApproval,
    BlockedByDependency,
    Queued,
    Running,
    NeedsReconciliation,
    Succeeded,
    Failed,
    Cancelled,
}

impl ActionStatus {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "planned" => Self::Planned,
            "waiting_for_input" => Self::WaitingForInput,
            "waiting_for_approval" => Self::WaitingForApproval,
            "blocked_by_dependency" => Self::BlockedByDependency,
            "queued" => Self::Queued,
            "running" => Self::Running,
            "needs_reconciliation" => Self::NeedsReconciliation,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
    }

    /// Terminal means "cannot change without an explicit decision". Note that
    /// `needs_reconciliation` is *not* terminal: it is a request for one, and the
    /// run projects as `waiting` while it stands.
    fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }

    fn blocks_dependents(self) -> bool {
        matches!(self, Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    Immediate,
    Background,
    Scheduled,
    Dependent,
}

impl ExecutionMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Immediate => "immediate",
            Self::Background => "background",
            Self::Scheduled => "scheduled",
            Self::Dependent => "dependent",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "immediate" => Self::Immediate,
            "background" => Self::Background,
            "scheduled" => Self::Scheduled,
            "dependent" => Self::Dependent,
            _ => return None,
        })
    }
}

/// How an action in `needs_reconciliation` was settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// Run it again. Only allowed when the action carries an idempotency key.
    Retry,
    Succeeded,
    Failed,
}

impl Resolution {
    fn as_str(self) -> &'static str {
        match self {
            Self::Retry => "retry",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

// --------------------------------------------------------------------- rows

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    #[serde(rename = "messageId")]
    pub message_id: Option<String>,
    pub title: String,
    pub status: RunStatus,
    #[serde(rename = "cancelledReason")]
    pub cancelled_reason: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    #[serde(rename = "runId")]
    pub run_id: String,
    pub outcome: String,
    pub capability: String,
    #[serde(rename = "executionMode")]
    pub execution_mode: ExecutionMode,
    pub status: ActionStatus,
    pub result: Option<String>,
    pub error: Option<String>,
    #[serde(rename = "cancellationReason")]
    pub cancellation_reason: Option<String>,
    #[serde(rename = "idempotencyKey")]
    pub idempotency_key: Option<String>,
    #[serde(rename = "requiresInput")]
    pub requires_input: bool,
    #[serde(rename = "requiresApproval")]
    pub requires_approval: bool,
    pub progress: Option<String>,
    pub attempt: i64,
    #[serde(rename = "leaseId")]
    pub lease_id: Option<String>,
    #[serde(rename = "leaseExpiresAt")]
    pub lease_expires_at: Option<String>,
    #[serde(rename = "dependsOn")]
    pub depends_on: Vec<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    #[serde(rename = "runId")]
    pub run_id: String,
    pub seq: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub payload: serde_json::Value,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSnapshot {
    pub run: Run,
    pub actions: Vec<Action>,
    pub events: Vec<RunEvent>,
    #[serde(rename = "latestSeq")]
    pub latest_seq: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskSummary {
    #[serde(rename = "actionId")]
    pub action_id: String,
    #[serde(rename = "runId")]
    pub run_id: String,
    pub outcome: String,
    pub capability: String,
    pub status: ActionStatus,
    pub result: Option<String>,
    pub error: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// A claim: the lease id that fences every later report for this attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub action: Action,
    #[serde(rename = "leaseId")]
    pub lease_id: String,
}

#[derive(Debug, Clone)]
pub struct NewRun {
    pub title: String,
    pub message_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NewAction {
    pub run_id: String,
    pub outcome: String,
    pub capability: String,
    pub execution_mode: Option<ExecutionMode>,
    pub depends_on: Vec<String>,
    /// Seeds `result` at plan time: what the capability runs on, before the
    /// action has produced anything of its own.
    pub input: Option<String>,
    pub requires_input: bool,
    pub requires_approval: bool,
    pub idempotency_key: Option<String>,
}

// --------------------------------------------------------------- redaction

/// Keys whose values must never reach the event log or a run row. The adapter
/// boundary sanitises provider responses; this is the second, cheap net.
const REDACTED_KEYS: [&str; 7] = [
    "key",
    "secret",
    "token",
    "password",
    "authorization",
    "cookie",
    "cvv",
];

fn sanitize(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(sanitize).collect())
        }
        serde_json::Value::Object(map) => {
            let mut safe = serde_json::Map::with_capacity(map.len());
            for (key, entry) in map {
                let lower = key.to_lowercase();
                if REDACTED_KEYS.iter().any(|needle| lower.contains(needle)) {
                    safe.insert(key.clone(), serde_json::Value::String("[redacted]".into()));
                } else {
                    safe.insert(key.clone(), sanitize(entry));
                }
            }
            serde_json::Value::Object(safe)
        }
        other => other.clone(),
    }
}

// -------------------------------------------------------------------- clock

/// ISO-8601 with milliseconds and a `Z`, matching what the TypeScript wrote, so
/// the two implementations can read each other's rows during the migration.
fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn iso_in_ms(millis: i64) -> String {
    Utc::now()
        .checked_add_signed(chrono::Duration::milliseconds(millis))
        .unwrap_or_else(Utc::now)
        .to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ----------------------------------------------------------------- schema

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY,
    message_id TEXT,
    title TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('planning', 'active', 'waiting', 'succeeded', 'failed', 'cancelled', 'partially_complete')),
    cancelled_reason TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS actions (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(id),
    outcome TEXT NOT NULL,
    capability TEXT NOT NULL,
    execution_mode TEXT NOT NULL CHECK (execution_mode IN ('immediate', 'background', 'scheduled', 'dependent')),
    status TEXT NOT NULL CHECK (status IN ('planned', 'waiting_for_input', 'waiting_for_approval', 'blocked_by_dependency', 'queued', 'running', 'needs_reconciliation', 'succeeded', 'failed', 'cancelled')),
    result TEXT,
    error TEXT,
    cancellation_reason TEXT,
    idempotency_key TEXT,
    requires_input INTEGER NOT NULL DEFAULT 0,
    requires_approval INTEGER NOT NULL DEFAULT 0,
    progress TEXT,
    attempt INTEGER NOT NULL DEFAULT 0,
    lease_id TEXT,
    lease_expires_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS action_dependencies (
    action_id TEXT NOT NULL REFERENCES actions(id),
    depends_on_action_id TEXT NOT NULL REFERENCES actions(id),
    PRIMARY KEY (action_id, depends_on_action_id)
);

CREATE TABLE IF NOT EXISTS run_events (
    run_id TEXT NOT NULL REFERENCES runs(id),
    seq INTEGER NOT NULL,
    type TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (run_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_actions_run ON actions(run_id);
CREATE INDEX IF NOT EXISTS idx_actions_lease ON actions(status, lease_expires_at);
"#;

// -------------------------------------------------------------- coordinator

/// The coordinator owns one SQLite connection. Every mutation is a transaction
/// that commits its state change and its replay event together, so a crash can
/// never leave an action that moved without a record of why.
pub struct Coordinator {
    connection: Mutex<Connection>,
}

impl Coordinator {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path).map_err(sqlite)?;
        connection.execute_batch("PRAGMA journal_mode = WAL;").map_err(sqlite)?;
        connection.execute_batch("PRAGMA foreign_keys = ON;").map_err(sqlite)?;
        connection.execute_batch(SCHEMA).map_err(sqlite)?;
        // IMMEDIATE rather than DEFERRED: every transaction here writes, and a
        // deferred transaction that upgrades later can fail to upgrade when two
        // workers arrive together.
        connection.execute_batch("PRAGMA busy_timeout = 5000;").map_err(sqlite)?;
        let coordinator = Self {
            connection: Mutex::new(connection),
        };
        {
            let mut conn = coordinator.lock()?;
            conn.set_transaction_behavior(rusqlite::TransactionBehavior::Immediate);
        }
        Ok(coordinator)
    }

    /// An in-memory coordinator for tests. Same code path, no file.
    pub fn in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory().map_err(sqlite)?;
        connection.execute_batch("PRAGMA foreign_keys = ON;").map_err(sqlite)?;
        connection.execute_batch(SCHEMA).map_err(sqlite)?;
        let coordinator = Self {
            connection: Mutex::new(connection),
        };
        {
            let mut conn = coordinator.lock()?;
            conn.set_transaction_behavior(rusqlite::TransactionBehavior::Immediate);
        }
        Ok(coordinator)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        // A poisoned mutex means a thread panicked while holding it. The data is
        // still consistent — every mutation is a transaction — so recovering is
        // better than propagating a panic into every later request.
        Ok(self.connection.lock().unwrap_or_else(|e| e.into_inner()))
    }

    // ------------------------------------------------------------- creation

    pub fn create_run(&self, input: NewRun) -> Result<Run> {
        let id = new_id();
        let at = now_iso();
        let title = {
            let trimmed = input.title.trim();
            if trimmed.is_empty() { "Untitled request" } else { trimmed }
        }
        .to_string();

        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(sqlite)?;
        tx.execute(
            "INSERT INTO runs (id, message_id, title, status, cancelled_reason, created_at, updated_at)
             VALUES (?, ?, ?, 'planning', NULL, ?, ?)",
            params![id, input.message_id, title, at, at],
        )
        .map_err(sqlite)?;
        append_event(
            &tx,
            &id,
            "run.created",
            serde_json::json!({ "title": title, "messageId": input.message_id }),
        )?;
        tx.commit().map_err(sqlite)?;

        Ok(Run {
            id,
            message_id: input.message_id,
            title,
            status: RunStatus::Planning,
            cancelled_reason: None,
            created_at: at.clone(),
            updated_at: at,
        })
    }

    pub fn add_action(&self, input: NewAction) -> Result<Action> {
        let id = new_id();
        let at = now_iso();
        let mode = input.execution_mode.unwrap_or(ExecutionMode::Immediate);

        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(sqlite)?;
        tx.execute(
            "INSERT INTO actions (
                id, run_id, outcome, capability, execution_mode, status, result, error,
                cancellation_reason, idempotency_key, requires_input, requires_approval,
                progress, attempt, lease_id, lease_expires_at, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, 'planned', ?, NULL, NULL, ?, ?, ?, NULL, 0, NULL, NULL, ?, ?)",
            params![
                id,
                input.run_id,
                input.outcome,
                input.capability,
                mode.as_str(),
                input.input,
                input.idempotency_key,
                i64::from(input.requires_input),
                i64::from(input.requires_approval),
                at,
                at,
            ],
        )
        .map_err(sqlite)?;

        for dependency in &input.depends_on {
            tx.execute(
                "INSERT INTO action_dependencies (action_id, depends_on_action_id) VALUES (?, ?)",
                params![id, dependency],
            )
            .map_err(sqlite)?;
        }

        append_event(
            &tx,
            &input.run_id,
            "action.planned",
            serde_json::json!({
                "actionId": id,
                "outcome": input.outcome,
                "capability": input.capability,
            }),
        )?;
        tx.commit().map_err(sqlite)?;

        Ok(Action {
            id,
            run_id: input.run_id,
            outcome: input.outcome,
            capability: input.capability,
            execution_mode: mode,
            status: ActionStatus::Planned,
            result: input.input,
            error: None,
            cancellation_reason: None,
            idempotency_key: input.idempotency_key,
            requires_input: input.requires_input,
            requires_approval: input.requires_approval,
            progress: None,
            attempt: 0,
            lease_id: None,
            lease_expires_at: None,
            depends_on: input.depends_on,
            created_at: at.clone(),
            updated_at: at,
        })
    }

    /// Moves the plan out of `planning`: eligible actions queue, actions with
    /// unmet prerequisites wait, and an action whose dependency failed or was
    /// cancelled is cancelled with a reason rather than silently dropped.
    pub fn commit_plan(&self, run_id: &str) -> Result<Run> {
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(sqlite)?;
        let ids = action_ids(&tx, run_id)?;
        for id in ids {
            reconcile(&tx, &id)?;
        }
        refresh(&tx, run_id)?;
        tx.commit().map_err(sqlite)?;
        drop(conn);
        self.run(run_id)?
            .ok_or_else(|| Error::State(format!("run {run_id} vanished")))
    }

    pub fn provide_input(&self, action_id: &str, input: &str) -> Result<()> {
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(sqlite)?;
        let Some(action) = read_action(&tx, action_id)? else {
            return Ok(());
        };
        if action.status != ActionStatus::WaitingForInput {
            tx.rollback().map_err(sqlite)?;
            return Ok(());
        }
        tx.execute(
            "UPDATE actions SET requires_input = 0, result = ?, updated_at = ? WHERE id = ?",
            params![input, now_iso(), action_id],
        )
        .map_err(sqlite)?;
        reconcile(&tx, action_id)?;
        refresh(&tx, &action.run_id)?;
        tx.commit().map_err(sqlite)
    }

    pub fn grant_approval(&self, action_id: &str, approved: bool) -> Result<()> {
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(sqlite)?;
        let Some(action) = read_action(&tx, action_id)? else {
            return Ok(());
        };
        if action.status != ActionStatus::WaitingForApproval {
            tx.rollback().map_err(sqlite)?;
            return Ok(());
        }
        if approved {
            tx.execute(
                "UPDATE actions SET requires_approval = 0, updated_at = ? WHERE id = ?",
                params![now_iso(), action_id],
            )
            .map_err(sqlite)?;
            reconcile(&tx, action_id)?;
        } else {
            cancel_action(&tx, action_id, &action.run_id, "approval-denied")?;
        }
        refresh(&tx, &action.run_id)?;
        tx.commit().map_err(sqlite)
    }

    // ---------------------------------------------------------------- leases

    /// Claims a queued action under a fresh lease. `None` when the action is not
    /// queued, including when it is already running — the only path to a new
    /// attempt is expiry, then reconciliation, then an explicit re-queue.
    pub fn claim_action(&self, action_id: &str, lease_ttl_ms: i64) -> Result<Option<Claim>> {
        let lease_id = new_id();
        let expires = iso_in_ms(lease_ttl_ms);

        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(sqlite)?;
        let Some(action) = read_action(&tx, action_id)? else {
            tx.rollback().map_err(sqlite)?;
            return Ok(None);
        };
        if action.status != ActionStatus::Queued {
            tx.rollback().map_err(sqlite)?;
            return Ok(None);
        }

        // The guard lives in the statement. `status = 'queued'` in the WHERE
        // clause means the check and the write cannot be separated by another
        // worker.
        let changed = tx
            .execute(
                "UPDATE actions
                    SET status = 'running', attempt = attempt + 1, lease_id = ?, lease_expires_at = ?, updated_at = ?
                  WHERE id = ? AND status = 'queued'",
                params![lease_id, expires, now_iso(), action_id],
            )
            .map_err(sqlite)?;
        if changed != 1 {
            tx.rollback().map_err(sqlite)?;
            return Ok(None);
        }

        append_event(
            &tx,
            &action.run_id,
            "action.running",
            serde_json::json!({ "actionId": action_id, "attempt": action.attempt + 1 }),
        )?;
        refresh(&tx, &action.run_id)?;
        let claimed = read_action(&tx, action_id)?.ok_or_else(|| Error::State("action vanished".into()))?;
        tx.commit().map_err(sqlite)?;
        drop(conn);

        Ok(Some(Claim { action: claimed, lease_id }))
    }

    pub fn renew_lease(&self, action_id: &str, lease_id: &str, lease_ttl_ms: i64) -> Result<bool> {
        let conn = self.lock()?;
        let changed = conn
            .execute(
                "UPDATE actions SET lease_expires_at = ?, updated_at = ?
                  WHERE id = ? AND status = 'running' AND lease_id = ?",
                params![iso_in_ms(lease_ttl_ms), now_iso(), action_id, lease_id],
            )
            .map_err(sqlite)?;
        Ok(changed == 1)
    }

    pub fn report_progress(&self, action_id: &str, lease_id: &str, progress: &str) -> Result<bool> {
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(sqlite)?;
        let Some(action) = read_action(&tx, action_id)? else {
            tx.rollback().map_err(sqlite)?;
            return Ok(false);
        };
        let changed = tx
            .execute(
                "UPDATE actions SET progress = ?, updated_at = ?
                  WHERE id = ? AND status = 'running' AND lease_id = ?",
                params![progress, now_iso(), action_id, lease_id],
            )
            .map_err(sqlite)?;
        if changed != 1 {
            tx.rollback().map_err(sqlite)?;
            return Ok(false);
        }
        append_event(
            &tx,
            &action.run_id,
            "action.progress",
            serde_json::json!({ "actionId": action_id, "progress": progress }),
        )?;
        tx.commit().map_err(sqlite)?;
        Ok(true)
    }

    pub fn complete_action(&self, action_id: &str, lease_id: &str, result: &str) -> Result<bool> {
        self.finish(action_id, lease_id, Finish::Succeeded, Some(result), None)
    }

    pub fn fail_action(&self, action_id: &str, lease_id: &str, error: &str) -> Result<bool> {
        self.finish(action_id, lease_id, Finish::Failed, None, Some(error))
    }

    fn finish(
        &self,
        action_id: &str,
        lease_id: &str,
        finish: Finish,
        result: Option<&str>,
        error: Option<&str>,
    ) -> Result<bool> {
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(sqlite)?;
        let Some(action) = read_action(&tx, action_id)? else {
            tx.rollback().map_err(sqlite)?;
            return Ok(false);
        };

        // The lease is cleared on completion. Leaving it set would let a late
        // report from this same attempt land on a finished action.
        let changed = tx
            .execute(
                "UPDATE actions
                    SET status = ?, result = COALESCE(?, result), error = ?,
                        lease_id = NULL, lease_expires_at = NULL, updated_at = ?
                  WHERE id = ? AND status = 'running' AND lease_id = ?",
                params![finish.as_str(), result, error, now_iso(), action_id, lease_id],
            )
            .map_err(sqlite)?;
        if changed != 1 {
            tx.rollback().map_err(sqlite)?;
            return Ok(false);
        }

        let (event, payload) = match finish {
            Finish::Succeeded => (
                "action.succeeded",
                serde_json::json!({ "actionId": action_id, "result": result }),
            ),
            Finish::Failed => (
                "action.failed",
                serde_json::json!({ "actionId": action_id, "error": error }),
            ),
        };
        append_event(&tx, &action.run_id, event, payload)?;
        unblock_dependents(&tx, action_id)?;
        refresh(&tx, &action.run_id)?;
        tx.commit().map_err(sqlite)?;
        Ok(true)
    }

    pub fn cancel_run(&self, run_id: &str, reason: &str) -> Result<()> {
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(sqlite)?;
        let Some(run) = read_run(&tx, run_id)? else {
            tx.rollback().map_err(sqlite)?;
            return Ok(());
        };
        if run.status.is_terminal() {
            tx.rollback().map_err(sqlite)?;
            return Ok(());
        }

        let mut statement = tx
            .prepare("SELECT id, status FROM actions WHERE run_id = ? AND status NOT IN ('succeeded', 'failed', 'cancelled')")
            .map_err(sqlite)?;
        let active: Vec<String> = statement
            .query_map(params![run_id], |row| row.get::<_, String>(0))
            .map_err(sqlite)?
            .collect::<std::result::Result<_, _>>()
            .map_err(sqlite)?;
        drop(statement);

        for id in active {
            tx.execute(
                "UPDATE actions
                    SET status = 'cancelled', cancellation_reason = ?, lease_id = NULL,
                        lease_expires_at = NULL, updated_at = ?
                  WHERE id = ?",
                params![reason, now_iso(), id],
            )
            .map_err(sqlite)?;
            append_event(
                &tx,
                run_id,
                "action.cancelled",
                serde_json::json!({ "actionId": id, "reason": reason }),
            )?;
        }

        tx.execute(
            "UPDATE runs SET status = 'cancelled', cancelled_reason = ?, updated_at = ? WHERE id = ?",
            params![reason, now_iso(), run_id],
        )
        .map_err(sqlite)?;
        append_event(&tx, run_id, "run.cancelled", serde_json::json!({ "reason": reason }))?;
        tx.commit().map_err(sqlite)
    }

    /// Startup recovery: a running action whose lease expired moves to
    /// `needs_reconciliation`. It is never blindly retried, because the side
    /// effect may already have happened; settling it is an explicit decision.
    pub fn recover_expired_leases(&self) -> Result<Vec<String>> {
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(sqlite)?;
        let now = now_iso();

        let mut statement = tx
            .prepare("SELECT id, run_id FROM actions WHERE status = 'running' AND lease_expires_at IS NOT NULL AND lease_expires_at < ?")
            .map_err(sqlite)?;
        let expired: Vec<(String, String)> = statement
            .query_map(params![now], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(sqlite)?
            .collect::<std::result::Result<_, _>>()
            .map_err(sqlite)?;
        drop(statement);

        if expired.is_empty() {
            tx.rollback().map_err(sqlite)?;
            return Ok(Vec::new());
        }

        let mut touched_runs: Vec<String> = Vec::new();
        for (id, run_id) in &expired {
            tx.execute(
                "UPDATE actions SET status = 'needs_reconciliation', lease_id = NULL, lease_expires_at = NULL, updated_at = ? WHERE id = ?",
                params![now, id],
            )
            .map_err(sqlite)?;
            append_event(
                &tx,
                run_id,
                "action.needs_reconciliation",
                serde_json::json!({ "actionId": id }),
            )?;
            if !touched_runs.contains(run_id) {
                touched_runs.push(run_id.clone());
            }
        }
        for run_id in &touched_runs {
            refresh(&tx, run_id)?;
        }
        tx.commit().map_err(sqlite)?;

        Ok(expired.into_iter().map(|(id, _)| id).collect())
    }

    pub fn resolve_reconciliation(&self, action_id: &str, resolution: Resolution) -> Result<bool> {
        let mut conn = self.lock()?;
        let tx = conn.transaction().map_err(sqlite)?;
        let Some(action) = read_action(&tx, action_id)? else {
            tx.rollback().map_err(sqlite)?;
            return Ok(false);
        };
        if action.status != ActionStatus::NeedsReconciliation {
            tx.rollback().map_err(sqlite)?;
            return Ok(false);
        }
        // Automatic retry requires a provider-backed idempotency key. Without
        // one the uncertain side effect needs a human decision.
        if resolution == Resolution::Retry && action.idempotency_key.is_none() {
            tx.rollback().map_err(sqlite)?;
            return Ok(false);
        }

        match resolution {
            Resolution::Retry => {
                tx.execute(
                    "UPDATE actions SET status = 'queued', updated_at = ? WHERE id = ?",
                    params![now_iso(), action_id],
                )
                .map_err(sqlite)?;
                append_event(
                    &tx,
                    &action.run_id,
                    "action.requeued",
                    serde_json::json!({ "actionId": action_id, "reason": "reconciled" }),
                )?;
            }
            Resolution::Succeeded | Resolution::Failed => {
                let error = if resolution == Resolution::Failed {
                    Some("reconciled-as-failed")
                } else {
                    None
                };
                tx.execute(
                    "UPDATE actions SET status = ?, error = ?, updated_at = ? WHERE id = ?",
                    params![resolution.as_str(), error, now_iso(), action_id],
                )
                .map_err(sqlite)?;
                append_event(
                    &tx,
                    &action.run_id,
                    &format!("action.{}", resolution.as_str()),
                    serde_json::json!({ "actionId": action_id }),
                )?;
                unblock_dependents(&tx, action_id)?;
            }
        }

        refresh(&tx, &action.run_id)?;
        tx.commit().map_err(sqlite)?;
        Ok(true)
    }

    // ----------------------------------------------------------------- reads

    pub fn run(&self, id: &str) -> Result<Option<Run>> {
        read_run(&*self.lock()?, id)
    }

    pub fn runs(&self, limit: u32) -> Result<Vec<Run>> {
        let conn = self.lock()?;
        let mut statement = conn
            .prepare("SELECT id, message_id, title, status, cancelled_reason, created_at, updated_at FROM runs ORDER BY created_at DESC LIMIT ?")
            .map_err(sqlite)?;
        let rows = statement
            .query_map(params![limit], read_run_row)
            .map_err(sqlite)?
            .collect::<std::result::Result<_, _>>()
            .map_err(sqlite)?;
        Ok(rows)
    }

    pub fn action(&self, id: &str) -> Result<Option<Action>> {
        read_action(&*self.lock()?, id)
    }

    pub fn actions(&self, run_id: &str) -> Result<Vec<Action>> {
        actions_of(&*self.lock()?, run_id)
    }

    pub fn events(&self, run_id: &str, after_seq: i64) -> Result<Vec<RunEvent>> {
        events_after(&*self.lock()?, run_id, after_seq)
    }

    /// A fresh connection's baseline: latest state plus every event after the
    /// cursor.
    pub fn snapshot(&self, run_id: &str, after_seq: i64) -> Result<RunSnapshot> {
        let conn = self.lock()?;
        let run = read_run(&conn, run_id)?
            .ok_or_else(|| Error::UnknownRun { id: run_id.to_string() })?;
        let events = events_after(&conn, run_id, after_seq)?;
        let latest = events.last().map(|e| e.seq).unwrap_or(after_seq);
        Ok(RunSnapshot {
            run,
            actions: actions_of(&conn, run_id)?,
            events,
            latest_seq: latest,
        })
    }

    /// One-shot delegations, newest first: every non-immediate action dispatched
    /// under a run. Standing sub-agents are role cards, so this is their task
    /// history.
    pub fn agent_tasks(&self, limit: u32) -> Result<Vec<AgentTaskSummary>> {
        let conn = self.lock()?;
        let mut statement = conn
            .prepare(
                "SELECT id, run_id, outcome, capability, status, result, error, created_at, updated_at
                   FROM actions WHERE execution_mode != 'immediate' ORDER BY created_at DESC LIMIT ?",
            )
            .map_err(sqlite)?;
        let rows = statement
            .query_map(params![limit], |row| {
                Ok(AgentTaskSummary {
                    action_id: row.get(0)?,
                    run_id: row.get(1)?,
                    outcome: row.get(2)?,
                    capability: row.get(3)?,
                    status: parse_status(&row.get::<_, String>(4)?),
                    result: row.get(5)?,
                    error: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(sqlite)?
            .collect::<std::result::Result<_, _>>()
            .map_err(sqlite)?;
        Ok(rows)
    }
}

#[derive(Clone, Copy)]
enum Finish {
    Succeeded,
    Failed,
}

impl Finish {
    fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

// -------------------------------------------------------------- row readers

fn read_run_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Run> {
    Ok(Run {
        id: row.get(0)?,
        message_id: row.get(1)?,
        title: row.get(2)?,
        status: parse_run_status(&row.get::<_, String>(3)?),
        cancelled_reason: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn read_action_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Action> {
    Ok(Action {
        id: row.get(0)?,
        run_id: row.get(1)?,
        outcome: row.get(2)?,
        capability: row.get(3)?,
        execution_mode: ExecutionMode::parse(&row.get::<_, String>(4)?).unwrap_or(ExecutionMode::Immediate),
        status: parse_status(&row.get::<_, String>(5)?),
        result: row.get(6)?,
        error: row.get(7)?,
        cancellation_reason: row.get(8)?,
        idempotency_key: row.get(9)?,
        requires_input: row.get::<_, i64>(10)? == 1,
        requires_approval: row.get::<_, i64>(11)? == 1,
        progress: row.get(12)?,
        attempt: row.get(13)?,
        lease_id: row.get(14)?,
        lease_expires_at: row.get(15)?,
        depends_on: Vec::new(),
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

/// An unrecognised status cannot happen — the schema CHECKs it — so falling back
/// to `planned` would hide a corruption. Panicking on a value the database
/// promised is the honest response.
fn parse_status(value: &str) -> ActionStatus {
    ActionStatus::parse(value).unwrap_or_else(|| panic!("unknown action status in database: {value}"))
}

fn parse_run_status(value: &str) -> RunStatus {
    RunStatus::parse(value).unwrap_or_else(|| panic!("unknown run status in database: {value}"))
}

const ACTION_COLUMNS: &str = "id, run_id, outcome, capability, execution_mode, status, result, error, cancellation_reason, idempotency_key, requires_input, requires_approval, progress, attempt, lease_id, lease_expires_at, created_at, updated_at";

fn read_run(conn: &Connection, id: &str) -> Result<Option<Run>> {
    conn.query_row(
        "SELECT id, message_id, title, status, cancelled_reason, created_at, updated_at FROM runs WHERE id = ?",
        params![id],
        read_run_row,
    )
    .optional()
    .map_err(sqlite)
}

fn read_action(conn: &Connection, id: &str) -> Result<Option<Action>> {
    let action = conn
        .query_row(
            &format!("SELECT {ACTION_COLUMNS} FROM actions WHERE id = ?"),
            params![id],
            read_action_row,
        )
        .optional()
        .map_err(sqlite)?;
    Ok(action.map(|mut action| {
        action.depends_on = dependencies_of(conn, &action.id).unwrap_or_default();
        action
    }))
}

fn dependencies_of(conn: &Connection, action_id: &str) -> Result<Vec<String>> {
    let mut statement = conn
        .prepare("SELECT depends_on_action_id FROM action_dependencies WHERE action_id = ? ORDER BY depends_on_action_id")
        .map_err(sqlite)?;
    let rows = statement
        .query_map(params![action_id], |row| row.get::<_, String>(0))
        .map_err(sqlite)?
        .collect::<std::result::Result<_, _>>()
        .map_err(sqlite)?;
    Ok(rows)
}

fn actions_of(conn: &Connection, run_id: &str) -> Result<Vec<Action>> {
    let mut statement = conn
        .prepare(&format!(
            "SELECT {ACTION_COLUMNS} FROM actions WHERE run_id = ? ORDER BY created_at ASC, id ASC"
        ))
        .map_err(sqlite)?;
    let mut actions: Vec<Action> = statement
        .query_map(params![run_id], read_action_row)
        .map_err(sqlite)?
        .collect::<std::result::Result<_, _>>()
        .map_err(sqlite)?;
    drop(statement);
    for action in &mut actions {
        action.depends_on = dependencies_of(conn, &action.id)?;
    }
    Ok(actions)
}

fn events_after(conn: &Connection, run_id: &str, after_seq: i64) -> Result<Vec<RunEvent>> {
    let mut statement = conn
        .prepare("SELECT run_id, seq, type, payload, created_at FROM run_events WHERE run_id = ? AND seq > ? ORDER BY seq ASC")
        .map_err(sqlite)?;
    let rows = statement
        .query_map(params![run_id, after_seq], |row| {
            let payload: String = row.get(3)?;
            Ok(RunEvent {
                run_id: row.get(0)?,
                seq: row.get(1)?,
                kind: row.get(2)?,
                // A payload we wrote is ours to read; one we cannot read means
                // corruption, which a silent default would turn into an empty
                // event rather than a visible failure.
                payload: serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null),
                created_at: row.get(4)?,
            })
        })
        .map_err(sqlite)?
        .collect::<std::result::Result<_, _>>()
        .map_err(sqlite)?;
    Ok(rows)
}

fn action_ids(conn: &Connection, run_id: &str) -> Result<Vec<String>> {
    let mut statement = conn
        .prepare("SELECT id FROM actions WHERE run_id = ? ORDER BY created_at ASC, id ASC")
        .map_err(sqlite)?;
    let rows = statement
        .query_map(params![run_id], |row| row.get::<_, String>(0))
        .map_err(sqlite)?
        .collect::<std::result::Result<_, _>>()
        .map_err(sqlite)?;
    Ok(rows)
}

// ------------------------------------------------------------------ helpers

/// Append a replay event. The sequence is read and written inside the caller's
/// transaction, so it cannot interleave with another writer's.
fn append_event(tx: &Transaction<'_>, run_id: &str, kind: &str, payload: serde_json::Value) -> Result<()> {
    let next: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM run_events WHERE run_id = ?",
            params![run_id],
            |row| row.get(0),
        )
        .map_err(sqlite)?;
    let encoded = serde_json::to_string(&sanitize(&payload)).map_err(|e| Error::State(e.to_string()))?;
    tx.execute(
        "INSERT INTO run_events (run_id, seq, type, payload, created_at) VALUES (?, ?, ?, ?, ?)",
        params![run_id, next, kind, encoded, now_iso()],
    )
    .map_err(sqlite)?;
    Ok(())
}

fn cancel_action(tx: &Transaction<'_>, action_id: &str, run_id: &str, reason: &str) -> Result<()> {
    tx.execute(
        "UPDATE actions SET status = 'cancelled', cancellation_reason = ?, lease_id = NULL, lease_expires_at = NULL, updated_at = ? WHERE id = ?",
        params![reason, now_iso(), action_id],
    )
    .map_err(sqlite)?;
    append_event(
        tx,
        run_id,
        "action.cancelled",
        serde_json::json!({ "actionId": action_id, "reason": reason }),
    )
}

/// Bring one action's status into line with its dependencies and its waits.
///
/// Order matters and is the whole function: a failed dependency cancels, an
/// in-flight dependency blocks, and only a satisfied action asks what it is
/// waiting for.
fn reconcile(tx: &Transaction<'_>, action_id: &str) -> Result<()> {
    let Some(action) = read_action(tx, action_id)? else {
        return Ok(());
    };
    if action.status.is_terminal() || action.status == ActionStatus::Running {
        return Ok(());
    }

    let mut statement = tx
        .prepare("SELECT a.status FROM actions a JOIN action_dependencies d ON d.depends_on_action_id = a.id WHERE d.action_id = ?")
        .map_err(sqlite)?;
    let dependencies: Vec<ActionStatus> = statement
        .query_map(params![action_id], |row| Ok(parse_status(&row.get::<_, String>(0)?)))
        .map_err(sqlite)?
        .collect::<std::result::Result<_, _>>()
        .map_err(sqlite)?;
    drop(statement);

    if dependencies.iter().any(|status| status.blocks_dependents()) {
        set_status(
            tx,
            action_id,
            "cancelled",
            "action.cancelled",
            serde_json::json!({ "actionId": action_id, "reason": "dependency-failed" }),
            Some(("cancellation_reason", "dependency-failed")),
        )?;
        return Ok(());
    }

    // Stay blocked only while a dependency is still in flight. Once every
    // dependency is terminal this falls through, which is what re-queues a
    // dependent after its prerequisite succeeds.
    if dependencies.iter().any(|status| !status.is_terminal()) {
        if action.status != ActionStatus::BlockedByDependency {
            set_status(tx, action_id, "blocked_by_dependency", "action.blocked", serde_json::json!({ "actionId": action_id }), None)?;
        }
        return Ok(());
    }

    if action.requires_input {
        return set_status(tx, action_id, "waiting_for_input", "action.waiting_input", serde_json::json!({ "actionId": action_id }), None);
    }
    if action.requires_approval {
        return set_status(tx, action_id, "waiting_for_approval", "action.waiting_approval", serde_json::json!({ "actionId": action_id }), None);
    }
    set_status(tx, action_id, "queued", "action.queued", serde_json::json!({ "actionId": action_id }), None)
}

/// Update an action's status, append its event, and republish the run status.
///
/// `extra` carries the one column that changes alongside the status, because
/// only cancellation sets a reason and writing it unconditionally would erase
/// one that was already there.
fn set_status(
    tx: &Transaction<'_>,
    action_id: &str,
    status: &str,
    event: &str,
    payload: serde_json::Value,
    extra: Option<(&str, &str)>,
) -> Result<()> {
    let run_id: String = tx
        .query_row("SELECT run_id FROM actions WHERE id = ?", params![action_id], |row| {
            row.get(0)
        })
        .map_err(sqlite)?;
    match extra {
        Some((column, value)) => tx
            .execute(
                &format!("UPDATE actions SET status = ?, {column} = ?, updated_at = ? WHERE id = ?"),
                params![status, value, now_iso(), action_id],
            )
            .map_err(sqlite)?,
        None => tx
            .execute(
                "UPDATE actions SET status = ?, updated_at = ? WHERE id = ?",
                params![status, now_iso(), action_id],
            )
            .map_err(sqlite)?,
    };
    append_event(tx, &run_id, event, payload)?;
    refresh(tx, &run_id)
}

fn unblock_dependents(tx: &Transaction<'_>, completed_action_id: &str) -> Result<()> {
    let mut statement = tx
        .prepare("SELECT action_id FROM action_dependencies WHERE depends_on_action_id = ?")
        .map_err(sqlite)?;
    let dependents: Vec<String> = statement
        .query_map(params![completed_action_id], |row| row.get::<_, String>(0))
        .map_err(sqlite)?
        .collect::<std::result::Result<_, _>>()
        .map_err(sqlite)?;
    drop(statement);

    for id in dependents {
        reconcile(tx, &id)?;
    }
    Ok(())
}

/// Parent status is a projection over child states, never a second mutable
/// truth. Waiting wins over active: a run that needs a human is waiting even if
/// other children could still progress.
fn project(conn: &Connection, run_id: &str) -> RunStatus {
    let mut statement = match conn.prepare("SELECT status FROM actions WHERE run_id = ?") {
        Ok(statement) => statement,
        Err(_) => return RunStatus::Succeeded,
    };
    let statuses: Vec<ActionStatus> = statement
        .query_map(params![run_id], |row| Ok(parse_status(&row.get::<_, String>(0)?)))
        .map_err(|_| rusqlite::Error::InvalidQuery)
        .and_then(|rows| rows.collect::<std::result::Result<Vec<_>, _>>())
        .unwrap_or_default();
    drop(statement);

    if statuses.is_empty() {
        return RunStatus::Succeeded;
    }
    if statuses.iter().any(|s| {
        matches!(
            s,
            ActionStatus::WaitingForInput | ActionStatus::WaitingForApproval | ActionStatus::NeedsReconciliation
        )
    }) {
        return RunStatus::Waiting;
    }
    if statuses.iter().any(|s| !s.is_terminal()) {
        return RunStatus::Active;
    }
    if statuses.iter().all(|s| *s == ActionStatus::Succeeded) {
        return RunStatus::Succeeded;
    }
    if statuses.iter().all(|s| *s == ActionStatus::Cancelled) {
        return RunStatus::Cancelled;
    }
    // No success anywhere plus at least one failure is failure, even when the
    // rest were cancelled; any other terminal mix is partial completion.
    if !statuses.contains(&ActionStatus::Succeeded) && statuses.contains(&ActionStatus::Failed) {
        return RunStatus::Failed;
    }
    RunStatus::PartiallyComplete
}

fn refresh(tx: &Transaction<'_>, run_id: &str) -> Result<()> {
    let status = project(tx, run_id);
    tx.execute(
        "UPDATE runs SET status = ?, updated_at = ? WHERE id = ?",
        params![status.as_str(), now_iso(), run_id],
    )
    .map_err(sqlite)?;
    Ok(())
}

fn sqlite(error: rusqlite::Error) -> Error {
    Error::State(format!("run store: {error}"))
}
#[cfg(test)]
mod tests;
