//! The agent manifest: `agent.toml`.
//!
//! Parsed with `deny_unknown_fields`, which is the same choice
//! `studio-platform` makes on its plugin descriptors. It costs nothing while the
//! format is young and saves everything later: a typo'd key is a startup error
//! naming the key, rather than a setting that silently never applies. The
//! alternative — ignoring what it does not recognise — produces an agent that
//! looks configured and is not.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, Error, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    pub agent: Identity,
    #[serde(default)]
    pub prompt: Prompt,
    pub memory: Memory,
    #[serde(default)]
    pub skills: Skills,
    #[serde(default)]
    pub capabilities: Capabilities,
    #[serde(default)]
    pub plugins: Plugins,
    #[serde(default)]
    pub subagents: Subagents,
    #[serde(default)]
    pub sandbox: Sandbox,
    /// Only the main agent carries these. A sub-agent manifest omits them
    /// entirely and inherits; see `agents/default/README.md`.
    #[serde(default, rename = "providers")]
    pub providers: Vec<Provider>,
    #[serde(default, rename = "use_cases")]
    pub use_cases: Vec<UseCase>,
    #[serde(default, rename = "model_overrides")]
    pub model_overrides: Vec<ModelOverride>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub name: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Prompt {
    pub instructions: Vec<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Memory {
    pub scope: String,
    pub facts: PathBuf,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Skills {
    #[serde(default)]
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub require: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Plugins {
    #[serde(default)]
    pub require: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Subagents {
    pub directory: PathBuf,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sandbox {
    pub workspace: PathBuf,
    pub profile: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    /// Anything that speaks the OpenAI chat-completions shape, which is most
    /// hosted inference including OpenRouter itself.
    OpenaiCompatible,
    Anthropic,
    /// A server on the network with no credential at all.
    Local,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: ProviderKind,
    pub endpoint: String,
    #[serde(default)]
    pub modalities: Vec<String>,
    pub key_file: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UseCase {
    pub id: String,
    pub title: String,
    pub description: String,
    /// `"<provider>/<model-id>"`, or a bare model id when the provider's
    /// catalogue does not cover the modality. OpenRouter's `/models` is
    /// text-only, so transcription rows have to be typed.
    pub model: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelOverride {
    pub model: String,
    pub context_window: u64,
}

impl AgentConfig {
    /// Read and validate a manifest, wrapping a failure with the path so an
    /// operator fixing one knows which file is wrong.
    pub fn load(manifest: &Path) -> Result<Self> {
        let path = manifest.display().to_string();
        let text = std::fs::read_to_string(manifest).map_err(|e| Error::Config {
            path: path.clone(),
            source: ConfigError::Read(e.to_string()),
        })?;
        Self::parse(&text).map_err(|source| Error::Config { path, source })
    }

    pub fn parse(text: &str) -> std::result::Result<Self, ConfigError> {
        let config: Self =
            toml::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> std::result::Result<(), ConfigError> {
        let mut provider_ids = BTreeSet::new();
        for provider in &self.providers {
            if !provider_ids.insert(provider.id.as_str()) {
                return Err(ConfigError::Invalid(format!(
                    "provider `{}` is declared more than once",
                    provider.id
                )));
            }
        }

        let mut use_case_ids = BTreeSet::new();
        for use_case in &self.use_cases {
            if !use_case_ids.insert(use_case.id.as_str()) {
                return Err(ConfigError::Invalid(format!(
                    "use-case `{}` is declared more than once",
                    use_case.id
                )));
            }
        }

        // `always` is the fallback every unlisted dispatch lands on, so its
        // absence would make a turn unroutable rather than merely unconfigured.
        if !self.use_cases.is_empty() && !use_case_ids.contains("always") {
            return Err(ConfigError::Invalid(
                "no `always` use-case: an unlisted dispatch has nowhere to fall back to".into(),
            ));
        }

        for use_case in &self.use_cases {
            if let Some((provider, _)) = use_case.model.split_once('/')
                && !provider_ids.contains(provider)
                && !provider_ids.is_empty()
            {
                return Err(ConfigError::Invalid(format!(
                    "use-case `{}` names provider `{provider}`, which is not configured",
                    use_case.id
                )));
            }
        }

        Ok(())
    }

    pub fn provider(&self, id: &str) -> Option<&Provider> {
        self.providers.iter().find(|p| p.id == id)
    }

    /// The context window the host must use verbatim for a model its provider
    /// does not list, if the manifest declares one.
    pub fn context_window(&self, model_id: &str) -> Option<u64> {
        self.model_overrides
            .iter()
            .find(|o| o.model == model_id)
            .map(|o| o.context_window)
    }

    /// Resolve the model for a use-case, falling back to `always` when the id is
    /// unknown.
    ///
    /// A row naming a provider that was never declared is rejected at load, not
    /// silently replaced — that is a typo in an authored file, and falling back
    /// would hide it. The other case, a provider that *is* declared but has no
    /// credential, is invisible here because this layer does not read the state
    /// directory. It is handled where the credential is resolved, so a missing
    /// transcription key degrades the row instead of breaking the turn.
    pub fn resolve(&self, use_case: &str) -> Option<&UseCase> {
        self.use_cases
            .iter()
            .find(|u| u.id == use_case)
            .or_else(|| self.use_cases.iter().find(|u| u.id == "always"))
    }
}

/// Split `"<provider>/<model-id>"`. A bare id yields `None` for the provider,
/// which is a legitimate typed entry rather than a malformed one.
pub fn split_model(model: &str) -> (Option<&str>, &str) {
    match model.split_once('/') {
        Some((provider, id)) if !provider.is_empty() && !id.is_empty() => (Some(provider), id),
        _ => (None, model),
    }
}