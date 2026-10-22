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
    /// Wake phrases: what the user says to wake this agent, and what that
    /// wakes. Absent means the platform default, which is what an agent
    /// created without a chosen name gets.
    #[serde(default = "default_wake_phrases", rename = "wake")]
    pub wake: Vec<WakePhrase>,
}

/// The phrase an agent answers to when the user skipped choosing one.
pub const DEFAULT_WAKE_PHRASE: &str = "AGENT";

/// The command the default phrase runs.
pub const DEFAULT_WAKE_COMMAND: &str = "chat.reply";

fn default_wake_phrases() -> Vec<WakePhrase> {
    vec![WakePhrase {
        phrase: DEFAULT_WAKE_PHRASE.into(),
        command: DEFAULT_WAKE_COMMAND.into(),
    }]
}

/// A phrase is capitals, digits, spaces, hyphens and apostrophes. The detector
/// matches a pre-spelled keyword, so the phrase is compared uppercased.
fn is_wake_phrase(phrase: &str) -> bool {
    let bytes = phrase.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    if !bytes[0].is_ascii_uppercase() && !bytes[0].is_ascii_digit() {
        return false;
    }
    bytes[1..].iter().all(|b| {
        b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b' ' || *b == b'-' || *b == b'\''
    })
}

/// A command is a plain name, so it cannot carry a shell line.
fn is_wake_command(command: &str) -> bool {
    let bytes = command.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    if !bytes[0].is_ascii_lowercase() {
        return false;
    }
    bytes[1..].iter().all(|b| {
        b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'.' || *b == b'_' || *b == b'-'
    })
}

/// One wake phrase and the command it runs.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WakePhrase {
    /// The phrase, uppercased. It is spelled the way the detector's own
    /// vocabulary spells it, so a phrase the model cannot say is refused.
    pub phrase: String,
    /// The command this phrase runs. A command name, not a free-form string.
    pub command: String,
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
        let mut config: Self =
            toml::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))?;
        config.validate()?;
        // A phrase is compared against a detected keyword, which is uppercased.
        // Normalising here means the manifest may be written in any case and
        // the stored value is the one that is compared.
        for wake in &mut config.wake {
            wake.phrase = wake.phrase.trim().to_uppercase();
        }
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

        // A wake phrase is uppercase, one to sixty-four characters, and a
        // command is a plain command name. Both are checked here so a manifest
        // that cannot work is refused at startup rather than at the moment
        // somebody says the wrong word.
        let mut phrases: BTreeSet<String> = BTreeSet::new();
        for wake in &self.wake {
            // Compared in the normalised form, because that is the form the
            // detector reports and the form two declarations would collide in.
            let phrase = wake.phrase.trim().to_uppercase();
            if !is_wake_phrase(&phrase) {
                return Err(ConfigError::Invalid(format!(
                    "wake phrase `{}` must be 1-64 letters, digits, spaces, hyphens or apostrophes, in capitals",
                    wake.phrase
                )));
            }
            if !is_wake_command(&wake.command) {
                return Err(ConfigError::Invalid(format!(
                    "wake command `{}` must be lowercase letters, digits, dots, underscores or hyphens",
                    wake.command
                )));
            }
            if !phrases.insert(phrase.clone()) {
                return Err(ConfigError::Invalid(format!(
                    "wake phrase `{}` is declared more than once",
                    phrase
                )));
            }
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

    /// The command a detected phrase runs, or `None` when the phrase is not one
    /// this agent answers to.
    pub fn wake_command(&self, phrase: &str) -> Option<&str> {
        let wanted = phrase.trim().to_uppercase();
        self.wake
            .iter()
            .find(|w| w.phrase == wanted)
            .map(|w| w.command.as_str())
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