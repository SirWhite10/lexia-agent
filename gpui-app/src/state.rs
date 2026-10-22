//! The state directory: where everything the host owns lives.
//!
//! This process is the only writer. That is the whole point of moving state out
//! of the TypeScript host — one store, one writer, so there is no window in
//! which two implementations can disagree about a run.
//!
//! Layout:
//!
//! ```text
//! <state>/
//! ├── credentials/<provider>.key   one per provider, owner-readable only
//! ├── plugins/versions/            immutable, content-addressed artifacts
//! ├── plugins/active/              the generation pointers actually loaded
//! ├── memory/                      the vector index; facts live in the agent dir
//! ├── workspaces/                  sandbox roots
//! ├── cache/
//! └── logs/
//! ```
//!
//! The agent *data* — manifest, instructions, facts, skills — is not here. It is
//! in `lexosa-agent/`, authored and tracked, and the host watches it. This
//! directory is only what the host writes.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct StateDir {
    root: PathBuf,
}

impl StateDir {
    /// Build the directory and everything under it. Safe to call repeatedly:
    /// existing directories are left alone, so a restart does not disturb state.
    pub fn ensure(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let dir = Self { root };

        for sub in [
            "",
            "credentials",
            "plugins",
            "plugins/versions",
            "plugins/active",
            "plugins/registry",
            "memory",
            "workspaces",
            "cache",
            "logs",
        ] {
            let path = dir.join(sub);
            fs::create_dir_all(&path).map_err(|e| {
                Error::State(format!("could not create {}: {e}", path.display()))
            })?;
        }

        // Credentials are secrets. Create the directory 0700 before anything is
        // written into it, rather than relying on the file mode alone.
        harden(&dir.credentials())?;

        Ok(dir)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The run store, beside the credential directory rather than inside the
    /// old `app-data/state` tree — the TypeScript host still owns
    /// `lexia.sqlite` until WF-IMP-025 removes it, and two writers on one
    /// database is the failure this migration exists to prevent.
    /// The plugin generation store: content-addressed artifacts, the registry,
    /// and the active pointers.
    pub fn plugins_directory(&self) -> PathBuf {
        self.join("plugins")
    }

    pub fn runs_database(&self) -> PathBuf {
        self.join("runs.sqlite")
    }

    pub fn credentials(&self) -> PathBuf {
        self.join("credentials")
    }

    pub fn key_file(&self, provider: &str) -> PathBuf {
        self.credentials().join(format!("{provider}.key"))
    }

    pub fn plugins_versions(&self) -> PathBuf {
        self.join("plugins/versions")
    }

    pub fn plugins_active(&self) -> PathBuf {
        self.join("plugins/active")
    }

    pub fn plugins_registry(&self) -> PathBuf {
        self.join("plugins/registry")
    }

    pub fn memory(&self) -> PathBuf {
        self.join("memory")
    }

    pub fn workspaces(&self) -> PathBuf {
        self.join("workspaces")
    }

    pub fn logs(&self) -> PathBuf {
        self.join("logs")
    }

    fn join(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }
}

/// Owner-only permissions, applied to whatever exists at `path`.
///
/// Unix-only. On other platforms there is no equivalent bit to set, and this is
/// a no-op rather than an error: refusing to start because a platform cannot
/// express a permission bit would be worse than running with the default.
#[cfg(unix)]
fn harden(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if !path.exists() {
        return Ok(());
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|e| Error::State(format!("could not restrict {}: {e}", path.display())))
}

#[cfg(not(unix))]
fn harden(_path: &Path) -> Result<()> {
    Ok(())
}