//! `agent` — the agent host.
//!
//! A CLI alongside the service, because the agent is a product in its own right
//! and not only a server for the UI. An operator needs to see which credential
//! is actually in force and which model a use-case resolves to, without
//! starting a server and opening a screen to find out.
//!
//! ```sh
//! agent serve                 # run the HTTP surface
//! agent config                # show the resolved manifest
//! agent providers             # providers and where each credential comes from
//! agent models                # use-cases and what they resolve to
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use agent_host::config::split_model;
use agent_host::credentials::Credentials;
use agent_host::http::{AppState, ServerEvent, event_channel, router};
use agent_host::plugins::{Capability, PluginHost, Registry};
use agent_host::runs::Coordinator;
use agent_host::{AgentConfig, StateDir, DEFAULT_AGENT_DIR, DEFAULT_STATE_SUBDIR};
use tokio::sync::RwLock;

#[derive(Parser)]
#[command(name = "agent", version, about = "The Agent agent host")]
struct Cli {
    #[arg(long, env = "AGENT_AGENT_DIR", default_value = DEFAULT_AGENT_DIR)]
    agent_dir: PathBuf,

    #[arg(long, env = "AGENT_STATE_DIR", default_value = DEFAULT_STATE_SUBDIR)]
    state_dir: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum PluginsCommand {
    /// Every registered plugin, its generations, and its active one.
    List,
    /// What this agent may actually call, and what it merely asked for.
    #[command(name = "granted")]
    Granted { agent: String, plugin: String },
}

#[derive(Subcommand)]
enum Command {
    /// Serve the HTTP surface the clients are written against.
    Serve {
        #[arg(long, env = "AGENT_BIND", default_value = "127.0.0.1:8787")]
        bind: String,
    },
    /// Show the resolved manifest: what the host read, not what the file says.
    Config,
    /// Providers, and which credential is in force for each.
    Providers,
    /// Plugins: what is installed, which generation is live, what each agent may call.
    #[command(subcommand)]
    Plugins(PluginsCommand),
    /// Use-cases and the model each resolves to.
    Models,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // Both crates emit logs: the binary traces startup and shutdown,
                // the library traces configuration and provider failures. Naming
                // only one silently discards the other.
                "agent=info,agent_host=info,tower_http=info,warn".into()
            }),
        )
        .init();

    let cli = Cli::parse();

    // Loaded only where it is needed. `plugins` and `providers` answer from the
    // state directory, and an operator running one of them on a half-configured
    // install should get the answer, not a refusal to start.
    let manifest = cli.agent_dir.join("agent.toml");
    let load_config = || {
        AgentConfig::load(&manifest).with_context(|| format!("loading {}", manifest.display()))
    };

    match cli.command {
        Command::Serve { bind } => serve(load_config()?, cli.state_dir, bind).await,
        Command::Config => {
            println!("{}", toml::to_string_pretty(&load_config()?)?);
            Ok(())
        }
        Command::Providers => {
            let state = StateDir::ensure(&cli.state_dir)?;
            let credentials = Credentials::new(state);
            let config = load_config()?;
            if config.providers.is_empty() {
                println!("no providers are configured in {}", manifest.display());
                return Ok(());
            }
            for provider in &config.providers {
                let status = credentials.status(provider);
                let shadowed = if status.shadowed_file {
                    " (a stored key exists but the environment is winning)"
                } else {
                    ""
                };
                println!(
                    "{:<16} {:<18} {}{shadowed}",
                    provider.id,
                    format!("{:?}", status.source).to_lowercase(),
                    provider.endpoint,
                );
            }
            Ok(())
        }
        Command::Plugins(PluginsCommand::List) => {
            let registry = Registry::open(cli.state_dir.join("plugins"))?;
            let host = PluginHost::new(registry)?;
            if host.registry().plugins()?.is_empty() {
                println!("no plugins are registered under {}", cli.state_dir.join("plugins").display());
                return Ok(());
            }
            for name in host.registry().plugins()? {
                let generations = host.registry().generations(&name)?;
                let active = host.registry().active(&name)?;
                let marker = active
                    .as_ref()
                    .map(|g| format!("v{} ({}…)", g.number, &g.hash[..12]))
                    .unwrap_or_else(|| "no active generation".into());
                println!("{name:<16} {} generation(s), active {marker}", generations.len());
                for generation in &generations {
                    let live = active.as_ref().is_some_and(|g| g.number == generation.number);
                    println!(
                        "  {} v{}  {:<8}  {}",
                        if live { "●" } else { " " },
                        generation.number,
                        &generation.hash[..12],
                        if generation.capabilities.is_empty() {
                            "no capabilities requested".to_string()
                        } else {
                            generation
                                .capabilities
                                .iter()
                                .map(|c| c.name())
                                .collect::<Vec<_>>()
                                .join(", ")
                        }
                    );
                }
            }
            Ok(())
        }
        Command::Plugins(PluginsCommand::Granted { agent, plugin }) => {
            let registry = Registry::open(cli.state_dir.join("plugins"))?;
            let host = PluginHost::new(registry)?;
            let requested = host
                .registry()
                .active(&plugin)?
                .map(|generation| generation.capabilities)
                .unwrap_or_default();
            let granted = host.effective(&agent, &plugin)?;
            println!("{plugin} for {agent}");
            for capability in Capability::ALL {
                let asked = requested.contains(&capability);
                let has = granted.contains(&capability);
                let state = match (asked, has) {
                    (true, true) => "granted",
                    (true, false) => "requested, not granted",
                    (false, true) => "granted, not requested",
                    (false, false) => "not requested",
                };
                println!("  {capability:<16} {state}");
            }
            Ok(())
        }
        Command::Models => {
            let config = load_config()?;
            if config.use_cases.is_empty() {
                println!("no use-cases are configured in {}", manifest.display());
                return Ok(());
            }
            for use_case in &config.use_cases {
                let (provider, model) = split_model(&use_case.model);
                let notes: Vec<String> = [
                    provider
                        .filter(|p| config.provider(p).is_none())
                        .map(|p| format!("provider `{p}` is not configured; falls back to always")),
                    config
                        .context_window(model)
                        .map(|w| format!("context {w} from an override")),
                    provider.is_none().then(|| "typed id, not from a catalogue".into()),
                ]
                .into_iter()
                .flatten()
                .collect();
                let note = if notes.is_empty() {
                    String::new()
                } else {
                    format!("  [{}]", notes.join("; "))
                };
                println!("{:<16} {}{note}", use_case.id, use_case.model);
            }
            Ok(())
        }
    }
}

async fn serve(config: AgentConfig, state_dir: PathBuf, bind: String) -> Result<()> {
    let state = StateDir::ensure(&state_dir)
        .with_context(|| format!("preparing state directory {}", state_dir.display()))?;
    let credentials = Arc::new(Credentials::new(state.clone()));
    let events = event_channel();
    let agent_name = config.agent.name.clone();

    // Startup recovery, before anything can claim a lease (ADR 0002). An action
    // whose lease outlived the process that held it goes to
    // `needs_reconciliation` — never straight back to the queue, because the
    // side effect may already have happened.
    let runs = Arc::new(Coordinator::open(&state.runs_database())?);
    let recovered = runs.recover_expired_leases()?;
    if !recovered.is_empty() {
        tracing::warn!(
            count = recovered.len(),
            "recovered expired leases; those actions need an explicit decision"
        );
    }

    let plugins = Arc::new(PluginHost::new(Registry::open(state.plugins_directory())?)?);

    let app_state = AppState {
        config: Arc::new(RwLock::new(config)),
        plugins,
        runs,
        credentials,
        state_dir: state,
        events: events.clone(),
        version: env!("CARGO_PKG_VERSION"),
    };

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("binding {bind}"))?;

    let local = listener.local_addr().unwrap_or_else(|_| bind.parse().unwrap());
    tracing::info!(agent = %agent_name, state = %state_dir.display(), "state directory ready");
    tracing::info!(%local, "serving");

    // Sent after the listener is bound rather than before, so a client that
    // connects immediately cannot miss it.
    let _ = events.send(ServerEvent::Started {
        agent: agent_name,
        version: env!("CARGO_PKG_VERSION").to_string(),
    });

    axum::serve(listener, router(app_state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server stopped")?;

    tracing::info!("stopped");
    Ok(())
}

/// Stop on Ctrl-C or SIGTERM. A supervisor restarting the host sends SIGTERM,
/// and exiting on it is what makes a restart a restart rather than a kill.
async fn shutdown_signal() {
    let interrupt = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(e) => tracing::warn!(%e, "could not install SIGTERM handler"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = interrupt => tracing::info!("interrupted"),
        _ = terminate => tracing::info!("terminated"),
    }
}