//! The Lexosa agent host.
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

pub mod config;
pub mod credentials;
pub mod error;
pub mod http;
pub mod models;
pub mod plugins;
pub mod runs;
pub mod state;

pub use config::AgentConfig;
pub use credentials::{CredentialStatus, Credentials, Source};
pub use error::{Error, Result, Secret};
pub use state::StateDir;

/// The default agent directory, when `LEXOSA_AGENT_DIR` is not set.
///
/// Resolved against the working directory so a checkout runs without
/// configuration, and overridable because a second agent on the same machine
/// needs its own.
pub const DEFAULT_AGENT_DIR: &str = "lexosa-agent";

/// The default state directory, when `LEXOSA_STATE_DIR` is not set.
///
/// Deliberately *inside* the old `app-data/state` tree but in its own
/// subdirectory. The TypeScript host is still running and still owns
/// `app-data/state/lexia.sqlite` until [WF-IMP-025] removes it; pointing both at
/// one directory would put two implementations in a position to write the same
/// file, which is the failure this whole migration is arranged to avoid.
pub const DEFAULT_STATE_SUBDIR: &str = "app-data/state/agent-host";

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
[agent]
name = "lexosa"
title = "Lexosa"
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
        assert_eq!(config.agent.name, "lexosa");
        assert_eq!(config.sandbox.profile, "standard");
        assert!(config.providers.is_empty());
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
        let dir = std::env::temp_dir().join(format!("lexosa-test-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }
}