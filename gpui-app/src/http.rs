//! The HTTP surface.
//!
//! Fixed here because the clients are written against it ([WF-IMP-012] onward),
//! and small on purpose: health, and the provider and model configuration the
//! Settings screens need. The turn endpoint and the run stream arrive with the
//! tickets that can answer them — [WF-IMP-023] and [WF-IMP-020] — rather than
//! as handlers that would have to say "not implemented".
//!
//! Every route here can actually serve its request. That is the constraint: a
//! path that exists but cannot answer is worse than a path that does not exist,
//! because a client will build against it.

use std::path::PathBuf;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post, put};
use axum::Router;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast};

use crate::config::{AgentConfig, split_model};
use crate::runs::Coordinator;
use crate::credentials::{CredentialStatus, Credentials, Source};
use crate::state::StateDir;

/// How many events a slow client may fall behind before its events are dropped.
/// The stream is diagnostic, not authoritative — the durable run log is that, and
/// it arrives with the coordinator — so dropping is better than blocking the
/// sender.
const EVENT_BUFFER: usize = 256;

/// How often the run stream looks for new events. The durable log is
/// authoritative and replayable; this only decides how quickly a live client sees
/// something, so a short interval costs nothing but a query.
const EVENT_POLL_MS: u64 = 400;

const DEFAULT_RUN_LIMIT: u32 = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ServerEvent {
    Started { agent: String, version: String },
    ConfigReloaded { agent: String },
    CredentialChanged { provider: String, source: CredentialSourceView },
    /// Sent once on connect so a client knows the stream is live rather than
    /// waiting for something that may never happen.
    StreamReady { agent: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CredentialSourceView {
    Environment,
    File,
    Absent,
}

impl From<crate::credentials::Source> for CredentialSourceView {
    fn from(source: crate::credentials::Source) -> Self {
        match source {
            crate::credentials::Source::Environment => Self::Environment,
            crate::credentials::Source::File => Self::File,
            crate::credentials::Source::Absent => Self::Absent,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<AgentConfig>>,
    /// The plugin host. Management lives here; *invoking* a plugin belongs to
    /// the agent loop ([WF-IMP-023](../../docs/planning/tickets/WF-IMP-023-agent-loop-sub-agents-and-builtins.md)),
    /// which is what decides that a capability is needed in the first place.
    pub plugins: Arc<crate::plugins::PluginHost>,
    /// The run coordinator. Its own store, its own lock; nothing above it holds a
    /// transaction open.
    pub runs: Arc<Coordinator>,
    pub credentials: Arc<Credentials>,
    pub state_dir: StateDir,
    pub events: broadcast::Sender<ServerEvent>,
    pub version: &'static str,
    /// The memory corpus: facts, the log and the vector index.
    pub memory: Arc<crate::memory::MemoryStore>,
    /// The standing sub-agents and the message log.
    pub sub_agents: Arc<crate::subagents::SubAgentStore>,
    /// The prompt assembler. It holds the cursor that makes the cache prefix
    /// contract work, so there is exactly one of them.
    ///
    /// An async mutex, because the assembler is held across the retrieval that
    /// assembles the prompt, and a guard held across an await has to be Send.
    pub prompt: Arc<tokio::sync::Mutex<crate::prompt::PromptAssembler>>,
    /// The agent's skills, read from its directory.
    pub skills: Arc<crate::skills::SkillIndex>,
    /// The agent's directory. The memory store writes its facts there.
    pub agent_dir: PathBuf,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/agent", get(describe_agent))
        .route("/api/turn", post(run_turn))
        .route("/api/providers", get(list_providers))
        .route("/api/providers/{id}/credential", put(set_credential))
        .route("/api/models", get(list_models))
        .route("/api/events", get(events))
        .route("/api/runs", get(list_runs).post(create_run))
        .route("/api/runs/{id}", get(get_run))
        .route("/api/runs/{id}/actions", get(list_run_actions))
        .route("/api/runs/{id}/events", get(run_events))
        .route("/api/runs/{id}/cancel", post(cancel_run))
        .route("/api/agent-tasks", get(list_agent_tasks))
        .route("/api/plugins", get(list_plugins))
        .route("/api/plugins/{name}/install", post(install_plugin))
        .route("/api/plugins/{name}/grants", get(plugin_grants))
        .route("/api/plugins/{name}/revoke", post(revoke_plugin))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let config = state.config.read().await;
    Json(serde_json::json!({
        "status": "ok",
        "agent": config.agent.name,
        "version": state.version,
        "providers": config.providers.len(),
        "state_dir": state.state_dir.root().display().to_string(),
        // Reported rather than assumed: a key that exists but is unreadable is
        // a different failure from one that was never written.
        "credentials_writable": state.state_dir.credentials().is_dir(),
    }))
}

#[derive(Serialize)]
struct ProviderView {
    id: String,
    /// The name a person sees. The Settings screen shows this rather than the
    /// id, because "openrouter" is not how a provider is written.
    label: String,
    /// One line saying what the provider is for.
    summary: String,
    /// The environment variable that outranks the stored key. The screen tells
    /// the operator which variable wins.
    env_var: String,
    /// The environment variable that moves a self-hosted provider, if it has
    /// one.
    base_url_env: Option<String>,
    /// Whether the provider publishes a model list the host can fetch. A
    /// provider that does not has its model ids typed by hand, and the screen
    /// has to say so rather than offer an empty picker.
    has_catalogue: bool,
    kind: &'static str,
    endpoint: String,
    modalities: Vec<String>,
    credential: CredentialSourceView,
    /// The environment credential is winning over a stored file. The settings
    /// screen has to be able to say that rather than show a masked file which is
    /// not in use.
    shadowed_file: bool,
}

/// The agent this host serves, and what it answers to.
///
/// The wake phrases are here because they are the agent's own data and the
/// clients need them to show the user what to say. The rules about what a
/// phrase may contain are enforced where the manifest is read, not here.
async fn describe_agent(State(state): State<AppState>) -> Json<AgentView> {
    let config = state.config.read().await;
    Json(AgentView {
        name: config.agent.name.clone(),
        title: config.agent.title.clone(),
        description: config.agent.description.clone(),
        wake: config
            .wake
            .iter()
            .map(|w| WakePhraseView {
                phrase: w.phrase.clone(),
                command: w.command.clone(),
            })
            .collect(),
    })
}

#[derive(Serialize)]
struct AgentView {
    name: String,
    title: String,
    description: String,
    wake: Vec<WakePhraseView>,
}

#[derive(Serialize)]
struct WakePhraseView {
    phrase: String,
    command: String,
}

async fn list_providers(State(state): State<AppState>) -> Json<Vec<ProviderView>> {
    let config = state.config.read().await;
    Json(
        config
            .providers
            .iter()
            .map(|provider| {
                let status = state.credentials.status(provider);
                ProviderView {
                    id: provider.id.clone(),
                    label: provider.display_label(),
                    summary: provider.summary.clone(),
                    env_var: provider.env_var_name(),
                    base_url_env: provider.base_url_env.clone(),
                    has_catalogue: provider.has_catalogue,
                    kind: match provider.kind {
                        crate::config::ProviderKind::OpenaiCompatible => "openai-compatible",
                        crate::config::ProviderKind::Anthropic => "anthropic",
                        crate::config::ProviderKind::Local => "local",
                    },
                    endpoint: provider.endpoint.clone(),
                    modalities: provider.modalities.clone(),
                    credential: status.source.into(),
                    shadowed_file: status.shadowed_file,
                }
            })
            .collect(),
    )
}

#[derive(Deserialize)]
struct CredentialInput {
    /// An empty value removes the stored key rather than storing an empty one,
    /// which would be indistinguishable from a truncated paste.
    value: String,
}

#[derive(Serialize)]
struct CredentialResponse {
    provider: String,
    credential: CredentialSourceView,
    shadowed_file: bool,
    /// True when a stored key was removed but an environment credential still
    /// outranks it. The removal succeeded; it did not take effect, and the client
    /// has to be able to tell the difference.
    removed_but_still_in_force: bool,
}

async fn set_credential(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<CredentialInput>,
) -> Result<Json<CredentialResponse>, ApiError> {
    let config = state.config.read().await;
    if config.provider(&id).is_none() {
        return Err(ApiError::unknown_provider(&id));
    }
    drop(config);

    let had_key = state.credentials.key_file(&id).is_file();
    let status: CredentialStatus = state.credentials.store(&id, &input.value)?;
    let removed = had_key && status.source == crate::credentials::Source::Absent;
    Ok(Json(CredentialResponse {
        provider: id,
        credential: status.source.into(),
        shadowed_file: status.shadowed_file,
        removed_but_still_in_force: removed && status.source == crate::credentials::Source::Environment,
    }))
}


#[derive(Deserialize)]
struct TurnInput {
    #[serde(rename = "messageId")]
    message_id: Option<String>,
    body: String,
}

#[derive(Serialize)]
struct TurnResponse {
    #[serde(rename = "runId")]
    run_id: String,
    #[serde(rename = "messageId")]
    message_id: String,
    /// How many memories were injected. Zero on a degraded turn.
    #[serde(rename = "memoryHits")]
    memory_hits: usize,
    /// True when the turn went out without memory, because the provider could
    /// not be reached or had no credential.
    degraded: bool,
    /// Why the reply did not happen, when it did not. A turn that failed is a
    /// reachable state, not a blank conversation.
    #[serde(rename = "replyFailed")]
    reply_failed: Option<String>,
}

/// The model a turn uses. Anything unlisted falls through to `always`, so the
/// fallback is the use-case named that and not the first one in the file.
fn model_for_turn(config: &AgentConfig) -> Option<&crate::config::UseCase> {
    config
        .use_cases
        .iter()
        .find(|use_case| use_case.id == "always")
        .or_else(|| config.use_cases.first())
}

/// The provider and credential a turn runs on, when there are any.
///
/// A missing credential is not an error here. The turn still goes out: it has
/// no memory and no answer, and says so, rather than refusing to exist.
fn turn_provider(
    config: &AgentConfig,
    credentials: &Credentials,
) -> Option<(Box<dyn crate::models::ModelProvider>, crate::error::Secret)> {
    let use_case = model_for_turn(config)?;
    let (provider_id, _) = crate::config::split_model(&use_case.model);
    let provider_id = provider_id?;
    let provider = config.provider(provider_id)?;
    let credential = credentials.resolve(provider).ok()?;
    let adapter = crate::models::provider_for(provider).ok()?;
    Some((adapter, credential))
}

/// One turn: capture it, prepare the prompt, plan the run, answer it, and run
/// whatever the plan left behind.
async fn run_turn(
    State(state): State<AppState>,
    Json(input): Json<TurnInput>,
) -> Result<Json<TurnResponse>, ApiError> {
    let body = input.body.trim().to_string();
    if body.is_empty() {
        return Err(ApiError::bad_plan("a turn needs a body".to_string()));
    }
    let config = state.config.read().await.clone();
    let message_id = input
        .message_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // The provider, with the credential in force.
    let (provider, credential) = match turn_provider(&config, &state.credentials) {
        Some((provider, credential)) => (Some(provider), Some(credential)),
        None => (None, None),
    };

    // The embedder goes through the same provider the completion does, so the
    // corpus is never indexed against a different endpoint than the one the
    // turn is answered by.
    // The embedder resolves its own credential. A `Secret` is not cloned, and
    // the embedder has to own one for as long as it lives.
    let embedder: Option<Box<dyn crate::memory::Embedder>> = match &provider {
        Some(provider) => {
            let resolved = model_for_turn(&config).and_then(|use_case| {
                let (provider_id, _) = crate::config::split_model(&use_case.model);
                provider_id
                    .and_then(|id| config.provider(id))
                    .and_then(|provider| state.credentials.resolve(provider).ok())
            });
            match resolved {
                Some(credential) => Some(Box::new(crate::memory::ProviderEmbedder::new(
                    provider.as_ref(),
                    credential,
                    crate::turn::EMBEDDING_MODEL,
                ))),
                None => None,
            }
        }
        None => None,
    };

    // Capture the turn, then assemble the prompt around it.
    let captured = crate::turn::capture_turn(
        &state.memory,
        embedder.as_deref(),
        &message_id,
        &body,
    )
    .await
    .map_err(ApiError::from)?;

    let skill_text: Vec<String> = state.skills.skills().iter().map(|s| s.body.clone()).collect();
    let mut assembler = state.prompt.lock().await;
    let turn = crate::turn::prepare_turn(
        &state.memory,
        &mut assembler,
        embedder.as_deref(),
        &skill_text,
        &body,
        None,
    )
    .await;
    drop(assembler);

    // The decision, and the run graph behind it.
    let plan = crate::router::plan_turn(&body);
    let planned = crate::agent_loop::plan_run(
        &state.runs,
        &state.sub_agents,
        &plan,
        &body,
        Some(&message_id),
    )
    .map_err(ApiError::from)?;

    // The reply. It is claimed, answered, and recorded — or it fails with a
    // reason the client can show.
    let reply_id = planned.actions[0].id.clone();
    let mut reply_failed = None;
    if let Some(lease) = state
        .runs
        .claim_action(&reply_id, crate::agent_loop::LEASE_TTL_MS)
        .map_err(ApiError::from)?
    {
        let model = model_for_turn(&config).map(|use_case| use_case.model.clone());
        let (provider_id, model_id) = model
            .as_deref()
            .map(crate::config::split_model)
            .map(|(provider, model)| (provider.map(str::to_string), model.to_string()))
            .unwrap_or((None, String::new()));

        let outcome: std::result::Result<String, String> = match (&provider, &credential, provider_id) {
            (Some(provider), Some(credential), Some(_)) => {
                let request = crate::models::CompletionRequest {
                    model: model_id,
                    messages: vec![
                        crate::models::Message::system(turn.prompt.system.clone()),
                        crate::models::Message::user(body.clone()),
                    ],
                    tools: Vec::new(),
                    response_schema: None,
                    temperature: None,
                    max_tokens: None,
                    context_window: None,
                };
                provider
                    .complete(credential, request)
                    .await
                    .map(|completion| completion.text)
                    .map_err(|error| error.to_string())
            }
            _ => Err(
                "no model is configured with a credential in force".to_string(),
            ),
        };

        match outcome {
            Ok(answer) => {
                let _ = state.sub_agents.append_message(
                    crate::subagents::MessageAuthorKind::Agent,
                    None,
                    None,
                    Some(&planned.run.id),
                    &answer,
                );
                let _ = state
                    .runs
                    .complete_action(&reply_id, &lease.lease_id, &answer);
            }
            Err(reason) => {
                reply_failed = Some(reason.clone());
                let _ = state
                    .runs
                    .fail_action(&reply_id, &lease.lease_id, &reason);
            }
        }
    }

    // Whatever the plan left behind that this host can run.
    let _ = crate::agent_loop::dispatch_queued_actions(
        &state.runs,
        state.state_dir.root(),
        &planned.run.id,
    )
    .await;

    // Reflection is a boundary effect: it may distil facts and traits, but it
    // never decides whether this turn succeeded.
    let transcript = state.sub_agents.transcript(20).unwrap_or_default();
    let _ = crate::turn::maybe_reflect(&state.memory, None, &transcript).await;

    let _ = captured;
    Ok(Json(TurnResponse {
        run_id: planned.run.id,
        message_id,
        memory_hits: turn.memory.len(),
        degraded: turn.degraded,
        reply_failed,
    }))
}

#[derive(Serialize)]
struct UseCaseView {
    id: String,
    title: String,
    description: String,
    model: String,
    provider: Option<String>,
    /// True when this row cannot actually be dispatched — its provider is
    /// declared but has no credential. Reported rather than hidden: a row that
    /// silently falls back to `always` looks like the setting worked.
    ///
    /// This cannot be known from the manifest alone, which is why it is computed
    /// here where the credential store is in reach.
    inert: bool,
    context_window: Option<u64>,
}

#[derive(Serialize)]
struct ModelsResponse {
    use_cases: Vec<UseCaseView>,
    // The provider list travels with the use-cases because the Settings
    // screen renders both: a use-case row names a provider, and the provider
    // card says what that provider is and whether it is configured.
    providers: Vec<ModelProviderView>,
    // `always` is what an unlisted dispatch falls back to, and a row whose
    // provider is unconfigured is marked `inert` rather than hidden — a silent
    // fallback looks like the setting worked.
}

async fn list_models(State(state): State<AppState>) -> Json<ModelsResponse> {
    let config = state.config.read().await;
    let use_cases: Vec<UseCaseView> = config
        .use_cases
        .iter()
        .map(|use_case| {
            let (provider, model_id) = split_model(&use_case.model);
            UseCaseView {
                id: use_case.id.clone(),
                title: use_case.title.clone(),
                description: use_case.description.clone(),
                model: use_case.model.clone(),
                provider: provider.map(str::to_string),
                inert: provider.is_some_and(|id| {
                    config
                        .provider(id)
                        .is_some_and(|p| state.credentials.status(p).source == Source::Absent)
                }),
                context_window: config.context_window(model_id),
            }
        })
        .collect();

    // The provider list travels with the use-cases because the Settings
    // screen renders both: a use-case row names a provider, and the provider
    // card says what that provider is and whether it is configured.
    let providers: Vec<ModelProviderView> = config
        .providers
        .iter()
        .map(|provider| ModelProviderView {
            id: provider.id.clone(),
            label: provider.display_label(),
            modalities: provider.modalities.clone(),
            has_catalogue: provider.has_catalogue,
            configured: state.credentials.status(provider).source != Source::Absent,
        })
        .collect();

    Json(ModelsResponse {
        use_cases,
        providers,
    })
}

#[derive(Serialize)]
struct ModelProviderView {
    id: String,
    label: String,
    modalities: Vec<String>,
    /// Whether a model list can be fetched for this provider. A provider that
    /// publishes none has its model ids typed by hand.
    has_catalogue: bool,
    /// Whether a credential is in force, from the environment or a stored file.
    configured: bool,
}

async fn events(
    State(state): State<AppState>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, std::io::Error>>> {
    let mut receiver = state.events.subscribe();
    let agent = state.config.read().await.agent.name.clone();

    let stream = async_stream::stream! {
        yield Ok(Event::default()
            .event("ready")
            .json_data(ServerEvent::StreamReady { agent: agent.clone() })
            .unwrap_or_else(|_| Event::default().event("ready").data("{}")));

        loop {
            match receiver.recv().await {
                Ok(event) => {
                    yield Ok(Event::default()
                        .event("event")
                        .json_data(event)
                        .unwrap_or_else(|_| Event::default().event("event").data("{}")));
                }
                Err(broadcast::error::RecvError::Lagged(missed)) => {
                    // Say so rather than pretending continuity. A client that
                    // missed N events can say "I missed N" instead of rendering
                    // a state it never received.
                    yield Ok(Event::default()
                        .event("lagged")
                        .data(format!("{{\"missed\":{missed}}}")));
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ------------------------------------------------------------ run surfaces

/// Submit a plan.
///
/// This is the seam a plan arrives through — from the agent loop once it exists,
/// or from an external harness driving the agent. The run, its actions and the
/// commit happen in one call, so a caller cannot leave a run sitting in
/// `planning` by forgetting a second request.
///
/// Dependencies are named by their position in the submitted array. A client
/// cannot know a UUID the server has not issued yet, and making it invent one
/// would be a way to reference nothing.
#[derive(Deserialize)]
struct PlanInput {
    #[serde(default)]
    title: String,
    #[serde(default, rename = "messageId")]
    message_id: Option<String>,
    #[serde(default)]
    actions: Vec<PlannedAction>,
}

#[derive(Deserialize)]
struct PlannedAction {
    outcome: String,
    capability: String,
    #[serde(default, rename = "executionMode")]
    execution_mode: Option<crate::runs::ExecutionMode>,
    /// Positions within this same plan. Renamed to `dependsOn` like every other
    /// field clients send; without it serde accepts `depends_on` and silently
    /// ignores the camelCase the API is documented to take, and a plan loses its
    /// dependencies with nothing to show for it.
    #[serde(default, rename = "dependsOn")]
    depends_on: Vec<usize>,
    #[serde(default)]
    input: Option<String>,
    #[serde(default, rename = "requiresInput")]
    requires_input: bool,
    #[serde(default, rename = "requiresApproval")]
    requires_approval: bool,
    #[serde(default, rename = "idempotencyKey")]
    idempotency_key: Option<String>,
}

async fn create_run(
    State(state): State<AppState>,
    Json(input): Json<PlanInput>,
) -> Result<(axum::http::StatusCode, Json<crate::runs::RunSnapshot>), ApiError> {
    for planned in &input.actions {
        if let Some(bad) = planned
            .depends_on
            .iter()
            .find(|index| **index >= input.actions.len())
        {
            return Err(ApiError::bad_plan(format!(
                "action `{}` depends on position {bad}, which is outside this plan of {}",
                planned.outcome,
                input.actions.len()
            )));
        }
    }

    let run = state.runs.create_run(crate::runs::NewRun {
        title: input.title,
        message_id: input.message_id,
    })?;

    let mut ids: Vec<String> = Vec::with_capacity(input.actions.len());
    for planned in &input.actions {
        let action = state.runs.add_action(crate::runs::NewAction {
            run_id: run.id.clone(),
            outcome: planned.outcome.clone(),
            capability: planned.capability.clone(),
            execution_mode: planned.execution_mode,
            depends_on: planned
                .depends_on
                .iter()
                .map(|index| ids[*index].clone())
                .collect(),
            input: planned.input.clone(),
            requires_input: planned.requires_input,
            requires_approval: planned.requires_approval,
            idempotency_key: planned.idempotency_key.clone(),
        })?;
        ids.push(action.id);
    }

    state.runs.commit_plan(&run.id)?;
    let snapshot = state.runs.snapshot(&run.id, 0)?;
    Ok((axum::http::StatusCode::CREATED, Json(snapshot)))
}

async fn list_runs(State(state): State<AppState>) -> Json<Vec<crate::runs::Run>> {
    Json(state.runs.runs(DEFAULT_RUN_LIMIT).expect("runs are readable"))
}

async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::runs::Run>, ApiError> {
    let run = state.runs.run(&id).map_err(ApiError::from)?;
    run.map(Json).ok_or_else(|| ApiError::unknown_run(&id))
}

async fn list_run_actions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<crate::runs::Action>>, ApiError> {
    if state.runs.run(&id).expect("runs are readable").is_none() {
        return Err(ApiError::unknown_run(&id));
    }
    Ok(Json(state.runs.actions(&id)?))
}

async fn list_agent_tasks(State(state): State<AppState>) -> Json<Vec<crate::runs::AgentTaskSummary>> {
    Json(state.runs.agent_tasks(DEFAULT_RUN_LIMIT).expect("tasks are readable"))
}

#[derive(Deserialize)]
struct CancelInput {
    #[serde(default = "default_cancel_reason")]
    reason: String,
}

fn default_cancel_reason() -> String {
    "user-cancelled".to_string()
}

async fn cancel_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<CancelInput>>,
) -> Result<Json<crate::runs::Run>, ApiError> {
    if state.runs.run(&id).expect("runs are readable").is_none() {
        return Err(ApiError::unknown_run(&id));
    }
    let reason = body.map(|Json(input)| input.reason).unwrap_or_else(default_cancel_reason);
    state.runs.cancel_run(&id, &reason)?;
    Ok(Json(
        state.runs.run(&id).expect("runs are readable").expect("exists"),
    ))
}

/// The run event stream.
///
/// Same contract as the endpoint it replaces: a snapshot first so a fresh client
/// has a baseline, then every event after the cursor, closing on a terminal run.
/// `Last-Event-ID` wins over `?after=` so a browser reconnect resumes exactly
/// where it dropped rather than replaying from the beginning.
async fn run_events(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<EventCursor>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, std::io::Error>>>, ApiError> {
    let cursor = cursor_from(&headers, &query);
    let snapshot = state.runs.snapshot(&id, cursor)?;

    // A client that supplied a cursor gets every event after it — including the
    // ones already in its snapshot. A fresh client, who has no cursor, starts at
    // the head, because its snapshot already carries everything before it.
    let resume_from = if cursor > 0 { cursor } else { snapshot.latest_seq };

    let coordinator = state.runs.clone();
    let run_id = id.clone();
    let stream = async_stream::stream! {
        let snap = serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".into());
        yield Ok(Event::default().event("snapshot").data(snap));

        let mut cursor = resume_from;
        loop {
            for event in coordinator.events(&run_id, cursor).unwrap_or_default() {
                cursor = event.seq;
                // Built field by field rather than as one formatted string: SSE
                // gives every field its own line, and a `data` value containing
                // raw newlines produces a frame no client can parse. The `id` is
                // what a reconnecting browser sends back as `Last-Event-ID`.
                yield Ok(Event::default()
                    .id(event.seq.to_string())
                    .event(event.kind.clone())
                    .data(event.payload.to_string()));
            }

            let terminal = coordinator
                .run(&run_id)
                .ok()
                .flatten()
                .is_some_and(|run| run.status.is_terminal());
            if terminal {
                yield Ok(Event::default()
                    .event("run.terminal")
                    .data(format!("{{\"latestSeq\":{cursor}}}")));
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(EVENT_POLL_MS)).await;
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

#[derive(Deserialize)]
struct EventCursor {
    #[serde(default)]
    after: Option<i64>,
}

/// `Last-Event-ID` is what the browser sends on an automatic reconnect, so it
/// has to win; `?after=` is for a client reconnecting by hand.
fn cursor_from(headers: &axum::http::HeaderMap, query: &EventCursor) -> i64 {
    headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<i64>().ok())
        .or(query.after)
        .unwrap_or(0)
        .max(0)
}

// ---------------------------------------------------------- plugin surface

#[derive(Serialize)]
struct PluginView {
    name: String,
    active: Option<u32>,
    generations: Vec<GenerationView>,
}

#[derive(Serialize)]
struct GenerationView {
    number: u32,
    hash: String,
    capabilities: Vec<String>,
    resources: ResourcesView,
}

#[derive(Serialize)]
struct ResourcesView {
    memory_mb: u32,
    fuel: u64,
    timeout_ms: u64,
}

async fn list_plugins(State(state): State<AppState>) -> Json<Vec<PluginView>> {
    let registry = state.plugins.registry();
    let Ok(names) = registry.plugins() else {
        return Json(Vec::new());
    };
    Json(
        names
            .into_iter()
            .map(|name| {
                let generations = registry.generations(&name).unwrap_or_default();
                let active = registry.active(&name).ok().flatten();
                PluginView {
                    name,
                    active: active.as_ref().map(|g| g.number),
                    generations: generations
                        .into_iter()
                        .map(|generation| GenerationView {
                            number: generation.number,
                            hash: generation.hash,
                            capabilities: generation
                                .capabilities
                                .iter()
                                .map(|c| c.name().to_string())
                                .collect(),
                            resources: ResourcesView {
                                memory_mb: generation.resources.memory_mb,
                                fuel: generation.resources.fuel,
                                timeout_ms: generation.resources.timeout_ms,
                            },
                        })
                        .collect(),
                }
            })
            .collect(),
    )
}

/// Install a build as a **candidate**. It does not become active: promotion is a
/// separate call, because the gap between "it compiled" and "it works" is where
/// the damage is done.
#[derive(Deserialize)]
struct InstallInput {
    /// The plugin's manifest, as TOML. Kept as text rather than a parsed struct
    /// so a manifest this host does not understand is rejected by name, at the
    /// point the person wrote it, rather than silently coerced.
    manifest: String,
    /// The wasm module, base64. A binary has no business travelling through JSON
    /// as an array of numbers.
    artifact: String,
}

async fn install_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(input): Json<InstallInput>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), ApiError> {
    use base64::Engine as _;

    let manifest = crate::plugins::Manifest::parse(&input.manifest)
        .map_err(|e| ApiError::bad_plan(e.to_string()))?;
    if manifest.name != name {
        return Err(ApiError::bad_plan(format!(
            "the manifest names `{}` but this route installs `{name}`",
            manifest.name
        )));
    }
    let artifact = base64::engine::general_purpose::STANDARD
        .decode(input.artifact.as_bytes())
        .map_err(|e| ApiError::bad_plan(format!("the artifact is not base64: {e}")))?;
    if !artifact.starts_with(b"\0asm") {
        return Err(ApiError::bad_plan(
            "the artifact is not a WebAssembly module".into(),
        ));
    }

    let number = state
        .plugins
        .registry()
        .install(&manifest, &artifact)
        .map_err(ApiError::from)?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(serde_json::json!({
            "plugin": name,
            "generation": number,
            "promoted": false,
            "requested": manifest.capabilities.iter().map(|c| c.name()).collect::<Vec<_>>(),
        })),
    ))
}

#[derive(Deserialize)]
struct GrantQuery {
    #[serde(default)]
    agent: String,
}

/// What was asked for versus what is actually allowed, per capability. The
/// distinction is the point: a plugin that requested `shell` and was refused is
/// not the same as one that never asked.
async fn plugin_grants(
    State(state): State<AppState>,
    Path(name): Path<String>,
    axum::extract::Query(query): axum::extract::Query<GrantQuery>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let requested = state
        .plugins
        .registry()
        .active(&name)
        .map_err(ApiError::from)?
        .map(|generation| generation.capabilities)
        .ok_or_else(|| ApiError::unknown_plugin(&name))?;
    let granted = state.plugins.effective(&query.agent, &name)?;

    Ok(Json(
        crate::plugins::Capability::ALL
            .into_iter()
            .map(|capability| {
                let asked = requested.contains(&capability);
                let has = granted.contains(&capability);
                serde_json::json!({
                    "capability": capability.name(),
                    "requested": asked,
                    "granted": has,
                    "state": match (asked, has) {
                        (true, true) => "granted",
                        (true, false) => "requested_not_granted",
                        (false, true) => "granted_not_requested",
                        (false, false) => "not_requested",
                    },
                })
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct RevokeInput {
    #[serde(default)]
    agent: String,
}

/// Revoke everything an agent holds for a plugin. The response names what was
/// revoked so the caller can deactivate a running instance now.
async fn revoke_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(input): Json<RevokeInput>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let revoked = state.plugins.revoke(&input.agent, &name);
    Ok(Json(serde_json::json!({
        "plugin": name,
        "agent": input.agent,
        "revoked": revoked.iter().map(|c| c.name()).collect::<Vec<_>>(),
    })))
}

#[derive(Serialize)]
struct ApiErrorBody {
    error: String,
}

#[derive(Debug)]
pub struct ApiError {
    status: axum::http::StatusCode,
    message: String,
}

impl ApiError {
    fn unknown_plugin(name: &str) -> Self {
        Self {
            status: axum::http::StatusCode::NOT_FOUND,
            message: format!("no plugin `{name}` is registered"),
        }
    }

    fn bad_plan(message: String) -> Self {
        Self {
            status: axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            message,
        }
    }

    fn unknown_run(id: &str) -> Self {
        Self {
            status: axum::http::StatusCode::NOT_FOUND,
            message: format!("no run `{id}`"),
        }
    }

    fn unknown_provider(id: &str) -> Self {
        Self {
            status: axum::http::StatusCode::NOT_FOUND,
            message: format!("no provider `{id}` is configured"),
        }
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(ApiErrorBody { error: self.message })).into_response()
    }
}

/// A host failure becomes a 500. The message is the host's own — written to be
/// safe to show an operator, and carrying no credential, because `Secret` does
/// not print its value.
impl From<crate::error::Error> for ApiError {
    fn from(error: crate::error::Error) -> Self {
        let status = match &error {
            crate::error::Error::UnknownRun { .. } => axum::http::StatusCode::NOT_FOUND,
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            message: error.to_string(),
        }
    }
}

/// Build the broadcast channel. Separated so tests and `main` create it the
/// same way.
pub fn event_channel() -> broadcast::Sender<ServerEvent> {
    broadcast::channel(EVENT_BUFFER).0
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Every field a client sends is camelCase on the wire. Serde accepts the
    /// Rust name as well, so a missing rename does not fail to parse — it fails
    /// to *do* anything, quietly. The first version of the plan endpoint lost
    /// every dependency this way, and nothing errored.
    #[test]
    fn a_plan_is_read_from_the_camel_case_names_clients_send() {
        let plan: PlanInput = serde_json::from_str(
            r#"{
                "title": "compound",
                "messageId": "m-1",
                "actions": [
                    { "outcome": "first", "capability": "builtin" },
                    {
                        "outcome": "second",
                        "capability": "builtin",
                        "dependsOn": [0],
                        "requiresApproval": true,
                        "idempotencyKey": "k-1",
                        "executionMode": "background"
                    }
                ]
            }"#,
        )
        .expect("camelCase plan parses");

        assert_eq!(plan.title, "compound");
        assert_eq!(plan.message_id.as_deref(), Some("m-1"));
        assert_eq!(plan.actions.len(), 2);
        assert_eq!(plan.actions[1].depends_on, vec![0]);
        assert!(plan.actions[1].requires_approval);
        assert_eq!(plan.actions[1].idempotency_key.as_deref(), Some("k-1"));
        assert_eq!(
            plan.actions[1].execution_mode,
            Some(crate::runs::ExecutionMode::Background)
        );
        assert!(plan.actions[0].depends_on.is_empty());
    }

    const PROVIDER_MANIFEST: &str = r#"
[agent]
name = "agent"
title = "Agent"
description = "The main agent."

[memory]
scope = "all"
facts = "memory/facts"

[[providers]]
id = "openrouter"
type = "openai-compatible"
endpoint = "https://openrouter.ai/api/v1"
modalities = ["text", "audio"]
key_file = "openrouter.key"
label = "OpenRouter"
summary = "Many labs behind one key."
"#;

    async fn state_for(config: &str) -> (AppState, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "agent-http-contract-{}",
            uuid::Uuid::new_v4()
        ));
        let state = StateDir::ensure(&directory).expect("state directory");
        // The memory corpus and the standing sub-agents are the agent's own.
        let memory = Arc::new(
            crate::memory::MemoryStore::open(state.root(), &state.root().join("agent"))
                .expect("the memory store opens"),
        );
        let sub_agents = Arc::new(
            crate::subagents::SubAgentStore::open(&state.root().join("subagents.sqlite"))
                .expect("the sub-agent store opens"),
        );
        let skills = Arc::new(crate::skills::SkillIndex::default());
        let agent_dir = state.root().join("agent");

        let app_state = AppState {
            config: Arc::new(RwLock::new(
                crate::AgentConfig::parse(config).expect("manifest"),
            )),
            plugins: Arc::new(
                crate::plugins::PluginHost::new(
                    crate::plugins::Registry::open(state.plugins_directory()).expect("registry"),
                )
                .expect("plugin host"),
            ),
            runs: Arc::new(Coordinator::open(&state.runs_database()).expect("coordinator")),
            // The environment is ignored on purpose. A test that depends on
            // the developer's shell is a test that fails on someone else's
            // machine, and the credential in force is part of what is being
            // asserted here.
            credentials: Arc::new(Credentials::with_env(state.clone(), |_| None)),
            state_dir: state,
            events: event_channel(),
            version: env!("CARGO_PKG_VERSION"),
            memory,
            sub_agents,
            prompt: Arc::new(tokio::sync::Mutex::new(
                crate::prompt::PromptAssembler::new(),
            )),
            skills,
            agent_dir,
        };
        (app_state, directory)
    }

    /// The Settings screen renders a provider card from these fields. A field
    /// the contract does not carry does not error: it renders as a blank where
    /// a name should be. So the contract is pinned here against the screen's
    /// own list.
    #[tokio::test]
    async fn the_provider_contract_carries_every_field_the_settings_screen_renders() {
        let (state, directory) = state_for(PROVIDER_MANIFEST).await;
        let Json(providers) = list_providers(State(state.clone())).await;
        let payload = serde_json::to_value(&providers).expect("json");
        let provider = &payload[0];
        for field in [
            "id",
            "kind",
            "endpoint",
            "modalities",
            "credential",
            "shadowed_file",
            "label",
            "summary",
            "env_var",
            "base_url_env",
            "has_catalogue",
        ] {
            assert!(
                provider.get(field).is_some(),
                "the provider contract is missing `{field}`: {provider}"
            );
        }
        assert_eq!(provider["label"], "OpenRouter");
        assert_eq!(
            provider["env_var"], "OPENROUTER_API_KEY",
            "the environment variable is derived from the id when the manifest does not say"
        );
        assert_eq!(provider["has_catalogue"], true);
        let _ = std::fs::remove_dir_all(&directory);
    }

    /// The models screen renders a provider card per provider alongside the
    /// use-case rows, and needs to know which providers are configured.
    #[tokio::test]
    async fn the_models_contract_carries_the_provider_cards() {
        let (state, directory) = state_for(PROVIDER_MANIFEST).await;
        let Json(response) = list_models(State(state.clone())).await;
        let payload = serde_json::to_value(&response).expect("json");
        assert!(
            payload.get("providers").is_some(),
            "the models contract is missing the provider list: {payload}"
        );
        let provider = &payload["providers"][0];
        for field in ["id", "label", "modalities", "has_catalogue", "configured"] {
            assert!(
                provider.get(field).is_some(),
                "the provider card is missing `{field}`: {provider}"
            );
        }
        assert_eq!(
            provider["configured"], false,
            "no credential was configured, so the card says so"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_run_cursor_prefers_last_event_id_over_the_query() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("last-event-id", "42".parse().unwrap());
        let query = EventCursor { after: Some(7) };
        assert_eq!(
            cursor_from(&headers, &query),
            42,
            "a browser reconnecting automatically must resume from where it dropped"
        );

        // No header: the explicit query parameter applies.
        let empty = axum::http::HeaderMap::new();
        assert_eq!(cursor_from(&empty, &query), 7);
        // Neither: a fresh client starts at the head of the log.
        assert_eq!(cursor_from(&empty, &EventCursor { after: None }), 0);
    }

    #[test]
    fn a_negative_cursor_is_clamped_rather_than_rejected() {
        let empty = axum::http::HeaderMap::new();
        assert_eq!(cursor_from(&empty, &EventCursor { after: Some(-5) }), 0);
    }
}
