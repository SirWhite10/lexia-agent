//! The Agent agent host.
//!
//! One crate, two distributions ([ADR 0004](../../docs/adr/0004-rust-client-all-platforms.md),
//! [ADR 0005](../../docs/adr/0005-rust-agent-host-and-wasm-plugins.md)): the `cdylib`
//! is the browser client, the binary is the agent host. Shared code lives here,
//! so a module written once is present in both.
//!
//! The layering, from the bottom:
//!
//! - [`state`] — where the host writes. One writer, always.
//! - [`config`] — `agent.toml`, the authored data that is not state.
//! - [`credentials`] — one key per provider, and which one is in force.
//! - [`models`] — the provider abstraction. Nothing above this names a vendor.
//! - [`runs`] — the durable run coordinator and its event log.
//! - [`http`] — the surface the clients are written against.
//!
//! Each layer knows only the ones below it. In particular [`models`] knows
//! nothing about HTTP, and nothing in the crate builds a vendor's request shape
//! by hand outside [`models`].

pub mod audio;
pub mod config;
pub mod credentials;
pub mod error;
pub mod http;
pub mod memory;
pub mod models;
pub mod plugins;
pub mod prompt;
pub mod runs;
pub mod skills;
pub mod state;
pub mod thought;
pub mod turn;

pub use config::AgentConfig;
pub use credentials::{CredentialStatus, Credentials, Source};
pub use error::{Error, Result, Secret};
pub use state::StateDir;

/// The default agent directory, when `AGENT_AGENT_DIR` is not set.
///
/// Resolved against the working directory so a checkout runs without
/// configuration, and overridable because a second agent on the same machine
/// needs its own.
pub const DEFAULT_AGENT_DIR: &str = "agents/default";

/// The default state directory, when `AGENT_STATE_DIR` is not set.
///
/// Deliberately *inside* the old `app-data/state` tree but in its own
/// subdirectory. The TypeScript host is still running and still owns
/// `app-data/state/agent.sqlite` until [WF-IMP-025] removes it; pointing both at
/// one directory would put two implementations in a position to write the same
/// file, which is the failure this whole migration is arranged to avoid.
pub const DEFAULT_STATE_SUBDIR: &str = "app-data/state/agent-host";

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
[agent]
name = "agent"
title = "Agent"
description = "The main agent."

[memory]
scope = "all"
facts = "memory/facts"

[sandbox]
workspace = "workspace"
profile = "standard"
"#;

    #[test]
    fn parses_a_minimal_manifest() {
        let config = AgentConfig::parse(MINIMAL).expect("valid");
        assert_eq!(config.agent.name, "agent");
        assert_eq!(config.sandbox.profile, "standard");
        assert!(config.providers.is_empty());
    }

    /// A manifest that says nothing about waking still has to wake to
    /// something, so the platform default applies. This is the path a user takes
    /// when they skip the setup step that picks a name.
    #[test]
    fn a_manifest_without_wake_phrases_gets_the_platform_default() {
        let config = AgentConfig::parse(MINIMAL).expect("valid");
        assert_eq!(config.wake.len(), 1);
        assert_eq!(config.wake[0].phrase, "AGENT");
        assert_eq!(config.wake_command("AGENT"), Some("chat.reply"));
    }

    #[test]
    fn one_agent_holds_several_phrases_each_mapped_to_its_own_command() {
        let text = format!(
            "{MINIMAL}{}",
            r#"
[[wake]]
phrase = "agent"
command = "chat.reply"

[[wake]]
phrase = "hey agent"
command = "note.write"
"#
        );
        let config = AgentConfig::parse(&text).expect("valid");
        // The phrase is stored uppercase, because that is what the detector
        // reports.
        assert_eq!(config.wake[0].phrase, "AGENT");
        assert_eq!(config.wake_command("AGENT"), Some("chat.reply"));
        assert_eq!(config.wake_command("HEY AGENT"), Some("note.write"));
        assert_eq!(config.wake_command("SOMETHING ELSE"), None);
    }

    #[test]
    fn a_phrase_that_cannot_be_spelled_is_refused() {
        let text = format!(
            "{MINIMAL}{}",
            r#"
[[wake]]
phrase = "H!ENROY"
command = "chat.reply"
"#
        );
        let error = AgentConfig::parse(&text).expect_err("an unspellable phrase must not parse");
        assert!(error.to_string().contains("wake phrase"), "{error}");
    }

    #[test]
    fn a_wake_command_must_be_a_command_name_and_not_a_shell_line() {
        let text = format!(
            "{MINIMAL}{}",
            r#"
[[wake]]
phrase = "AGENT"
command = "rm -rf /"
"#
        );
        let error = AgentConfig::parse(&text).expect_err("a shell line must not parse");
        assert!(error.to_string().contains("wake command"), "{error}");
    }

    #[test]
    fn the_same_phrase_cannot_be_declared_twice() {
        let text = format!(
            "{MINIMAL}{}",
            r#"
[[wake]]
phrase = "AGENT"
command = "chat.reply"

[[wake]]
phrase = "agent"
command = "note.write"
"#
        );
        let error = AgentConfig::parse(&text).expect_err("a duplicate phrase must not parse");
        assert!(error.to_string().contains("more than once"), "{error}");
    }

    /// The whole reason for `deny_unknown_fields`: a typo'd key must fail loudly
    /// rather than leave a setting that silently never applies.
    #[test]
    fn rejects_an_unknown_key() {
        let typo = MINIMAL.replace("profile = \"standard\"", "profiel = \"standard\"");
        let error = AgentConfig::parse(&typo).expect_err("typo must not parse");
        assert!(matches!(error, crate::error::ConfigError::Parse(_)));
    }

    #[test]
    fn rejects_a_duplicate_provider() {
        let text = format!(
            "{MINIMAL}
[[providers]]
id = \"openrouter\"
type = \"openai-compatible\"
endpoint = \"https://openrouter.ai/api/v1\"
key_file = \"openrouter.key\"

[[providers]]
id = \"openrouter\"
type = \"openai-compatible\"
endpoint = \"https://example.invalid\"
key_file = \"other.key\"
"
        );
        let error = AgentConfig::parse(&text).expect_err("duplicate must not parse");
        assert!(error.to_string().contains("declared more than once"));
    }

    #[test]
    fn requires_an_always_use_case_when_any_are_declared() {
        let text = format!(
            "{MINIMAL}
[[use_cases]]
id = \"heavy\"
title = \"Heavy\"
description = \"\"
model = \"openrouter/some-model\"

[[providers]]
id = \"openrouter\"
type = \"openai-compatible\"
endpoint = \"https://openrouter.ai/api/v1\"
key_file = \"openrouter.key\"
"
        );
        let error = AgentConfig::parse(&text).expect_err("missing always must not parse");
        assert!(error.to_string().contains("always"));
    }

    /// Two different failures that look alike and must not be conflated. A row
    /// naming a provider that was never *declared* is a typo in an authored
    /// file, so it is rejected where the operator will see it. A provider that is
    /// declared but has no *credential* is invisible to this layer and degrades
    /// at dispatch instead — that is covered where the key is resolved.
    #[test]
    fn an_undeclared_provider_is_rejected_rather_than_silently_replaced() {
        let text = format!(
            "{MINIMAL}{}",
            r#"
[[use_cases]]
id = "always"
title = "Always"
description = ""
model = "openrouter/present-model"

[[use_cases]]
id = "transcription"
title = "Transcription"
description = ""
model = "absent/absent-model"

[[providers]]
id = "openrouter"
type = "openai-compatible"
endpoint = "https://openrouter.ai/api/v1"
key_file = "openrouter.key"
"#
        );
        let error = AgentConfig::parse(&text).expect_err("must not parse");
        let message = error.to_string();
        assert!(message.contains("transcription"), "names the row: {message}");
        assert!(message.contains("absent"), "names the provider: {message}");
    }

    #[test]
    fn an_unknown_use_case_falls_back_to_always() {
        let text = format!(
            "{MINIMAL}{}",
            r#"
[[use_cases]]
id = "always"
title = "Always"
description = ""
model = "openrouter/present-model"

[[providers]]
id = "openrouter"
type = "openai-compatible"
endpoint = "https://openrouter.ai/api/v1"
key_file = "openrouter.key"
"#
        );
        let config = AgentConfig::parse(&text).expect("valid");
        assert_eq!(
            config.resolve("transcription").expect("always exists").id,
            "always",
            "an unlisted use-case falls back rather than failing"
        );
    }

    #[test]
    fn a_bare_model_id_is_typed_not_malformed() {
        let (provider, model) = config::split_model("whisper-large-v3");
        assert_eq!(provider, None);
        assert_eq!(model, "whisper-large-v3");

        let (provider, model) = config::split_model("openrouter/ling-3.0");
        assert_eq!(provider, Some("openrouter"));
        assert_eq!(model, "ling-3.0");
    }

    #[test]
    fn credentials_resolve_from_the_environment_first() {
        let state = StateDir::ensure(temp_state("creds-env")).expect("state dir");
        let credentials = Credentials::with_env(state, |key| {
            (key == "OPENROUTER_API_KEY").then(|| "from-env".to_string())
        });
        let provider = config::Provider {
            id: "openrouter".into(),
            kind: config::ProviderKind::OpenaiCompatible,
            endpoint: "https://openrouter.ai/api/v1".into(),
            modalities: vec!["text".into()],
            key_file: "openrouter.key".into(),
            label: None,
            summary: String::new(),
            env_var: None,
            base_url_env: None,
            has_catalogue: true,
        };

        // Nothing stored yet: the environment is the only source.
        let status = credentials.status(&provider);
        assert_eq!(status.source, crate::credentials::Source::Environment);
        assert_eq!(
            credentials.resolve(&provider).expect("resolves").expose(),
            "from-env"
        );

        // Store one. The environment still wins, and the shadow is reported.
        credentials.store("openrouter", "from-file").expect("stores");
        let status = credentials.status(&provider);
        assert_eq!(status.source, crate::credentials::Source::Environment);
        assert!(status.shadowed_file, "the file is shadowed, not gone");

        // Removing the stored key cannot remove the environment credential, and
        // saying otherwise would be a lie the settings screen would show.
        credentials.remove("openrouter").expect("removes");
        assert!(!credentials.key_file("openrouter").is_file());
        assert_eq!(
            credentials.status(&provider).source,
            crate::credentials::Source::Environment
        );
    }

    #[test]
    fn an_empty_submission_removes_rather_than_stores_an_empty_key() {
        let state = StateDir::ensure(temp_state("creds-empty")).expect("state dir");
        let credentials = Credentials::with_env(state, |_| None);

        credentials.store("openrouter", "a-real-key").expect("stores");
        assert!(credentials.key_file("openrouter").is_file());

        credentials.store("openrouter", "   ").expect("removes");
        assert!(
            !credentials.key_file("openrouter").is_file(),
            "an empty submission must remove the key, not store whitespace"
        );
    }

    #[test]
    fn a_secret_does_not_print_its_value() {
        let secret = Secret::new("super-secret-key".into());
        let rendered = format!("{secret:?}");
        assert!(!rendered.contains("super-secret"));
        assert!(rendered.contains("16"), "length is diagnostic and safe");
    }

    fn temp_state(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("agent-test-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }
}