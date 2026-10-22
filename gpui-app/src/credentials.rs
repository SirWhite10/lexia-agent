//! Credentials: one per provider, and where it came from.
//!
//! The precedence rule is not this codebase's invention — it is carried over
//! unchanged from the TypeScript host because it is product behaviour, not
//! implementation detail, and it is written down in `OPERATIONS.md`:
//!
//! > `<PROVIDER>_API_KEY` in the host environment outranks the stored file for
//! > the life of the process, so a shell configuration is never silently
//! > overridden from a browser.
//!
//! Two consequences the UI depends on. The screen can say *which* credential is
//! in force rather than showing a masked file that is not being used. And an
//! environment credential cannot be cleared from the browser — removing the
//! stored key leaves the environment one still winning, and the removal must
//! say so rather than appear to have worked.

use std::fs;
use std::path::Path;

use crate::config::Provider;
use crate::error::{Error, Result, Secret};
use crate::state::StateDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    /// `<PROVIDER>_API_KEY` in the process environment.
    Environment,
    /// `<state>/credentials/<provider>.key`.
    File,
    /// Neither. The provider is declared but cannot be called.
    Absent,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CredentialStatus {
    pub provider: String,
    pub source: Source,
    /// True when the environment credential is winning over a stored file, which
    /// the settings screen must be able to say plainly.
    pub shadowed_file: bool,
}

/// Reading the environment through a function rather than `std::env::var`, so
/// precedence can be tested without mutating the test process's own environment.
type EnvLookup = Box<dyn Fn(&str) -> Option<String> + Send + Sync>;

pub struct Credentials {
    state: StateDir,
    env: EnvLookup,
}

impl Credentials {
    pub fn new(state: StateDir) -> Self {
        Self {
            state,
            env: Box::new(|key| std::env::var(key).ok()),
        }
    }

    /// Test seam for `EnvLookup`.
    pub fn with_env(
        state: StateDir,
        env: impl Fn(&str) -> Option<String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            state,
            env: Box::new(env),
        }
    }

    pub fn status(&self, provider: &Provider) -> CredentialStatus {
        let from_env = (self.env)(&env_var(&provider.id)).is_some();
        let from_file = self.key_file(&provider.id).exists();

        CredentialStatus {
            provider: provider.id.clone(),
            source: if from_env {
                Source::Environment
            } else if from_file {
                Source::File
            } else {
                Source::Absent
            },
            shadowed_file: from_env && from_file,
        }
    }

    /// The credential to actually use, or why there isn't one.
    pub fn resolve(&self, provider: &Provider) -> Result<Secret> {
        if let Some(value) = (self.env)(&env_var(&provider.id)) {
            return Ok(Secret::new(value));
        }
        let path = self.key_file(&provider.id);
        if let Ok(value) = fs::read_to_string(&path)
            && !value.trim().is_empty()
        {
            return Ok(Secret::new(value.trim().to_string()));
        }
        Err(Error::MissingCredential {
            provider: provider.id.clone(),
        })
    }

    pub fn key_file(&self, provider: &str) -> std::path::PathBuf {
        self.state.key_file(provider)
    }

    /// Store a credential. An empty value removes the key rather than storing an
    /// empty one, which would be indistinguishable from a truncated paste.
    pub fn store(&self, provider: &str, value: &str) -> Result<CredentialStatus> {
        let trimmed = value.trim();
        let path = self.key_file(provider);

        if trimmed.is_empty() {
            remove_file(&path)?;
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            write_owner_only(&path, trimmed)?;
        }

        // Report what actually holds now, not what was asked for: the
        // environment may still outrank a key that was just written.
        let status = CredentialStatus {
            provider: provider.to_string(),
            source: if (self.env)(&env_var(provider)).is_some() {
                Source::Environment
            } else if path.is_file() {
                Source::File
            } else {
                Source::Absent
            },
            shadowed_file: (self.env)(&env_var(provider)).is_some() && path.is_file(),
        };
        Ok(status)
    }

    pub fn remove(&self, provider: &str) -> Result<()> {
        remove_file(&self.key_file(provider))
    }
}

fn env_var(provider: &str) -> String {
    format!(
        "{}_API_KEY",
        provider
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
            .collect::<String>()
    )
}

/// Removing a key that is not there is success, not an error. The operation the
/// caller asked for has been achieved either way.
fn remove_file(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(unix)]
fn write_owner_only(path: &Path, value: &str) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(value.as_bytes())?;
    // Written without a trailing newline on purpose: the value is used verbatim
    // as a bearer token, and a trailing newline in an Authorization header is a
    // confusing way to fail authentication.
    file.flush()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_owner_only(path: &Path, value: &str) -> Result<()> {
    fs::write(path, value)?;
    Ok(())
}