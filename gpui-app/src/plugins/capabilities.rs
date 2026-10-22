//! What a plugin may ask for, and what it may actually have.
//!
//! A **closed enum**, not free-form strings, following `studio-platform`'s
//! `DeclaredCapability`. The difference is what happens to a typo: a string
//! either resolves to nothing and the plugin silently runs crippled, or it fails
//! to parse and the name is in the error. For a system where the agent writes its
//! own plugins, "silently runs with less than you asked for" is a bug class worth
//! removing at the type level.
//!
//! Two records, deliberately separate:
//!
//! - **Requested** — what a plugin's manifest declares. A request, never a grant.
//! - **Granted** — what the operator consented to. See [`super::consent`].

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

/// The host calls a plugin may make. Adding one is a deliberate act: every
/// variant is a new thing agent-authored code can reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// `host.log` — write to the host's log. The cheapest capability, and the
    /// one a plugin cannot do without.
    Log,
    /// `host.http.get` — issue an outbound GET.
    HttpGet,
    /// `host.http.post` — issue an outbound request with a body.
    HttpPost,
    /// Read files, restricted to the sandbox root.
    FilesystemRead,
    /// Write files, restricted to the sandbox root.
    FilesystemWrite,
    /// Run a command. The most dangerous grant there is.
    Shell,
    /// Read the agent's memory through `host.memory.search`.
    MemoryRead,
    /// Write a durable fact through `host.memory.write`.
    MemoryWrite,
}

impl Capability {
    pub const ALL: [Capability; 8] = [
        Capability::Log,
        Capability::HttpGet,
        Capability::HttpPost,
        Capability::FilesystemRead,
        Capability::FilesystemWrite,
        Capability::Shell,
        Capability::MemoryRead,
        Capability::MemoryWrite,
    ];

    /// The stable wire name, used in manifests, in the consent ledger and in
    /// error messages. Never derived from `Debug`, so renaming the Rust variant
    /// cannot silently rename the stored grant.
    pub fn name(self) -> &'static str {
        match self {
            Capability::Log => "log",
            Capability::HttpGet => "http_get",
            Capability::HttpPost => "http_post",
            Capability::FilesystemRead => "filesystem_read",
            Capability::FilesystemWrite => "filesystem_write",
            Capability::Shell => "shell",
            Capability::MemoryRead => "memory_read",
            Capability::MemoryWrite => "memory_write",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Capability::ALL.into_iter().find(|c| c.name() == value)
    }

    /// The module export this capability unlocks. A grant that does not map to
    /// exactly one import is a bug rather than a configuration, because a grant
    /// with no effect would read as a grant that works.
    pub fn host_import(self) -> &'static str {
        match self {
            Capability::Log => "agent_host_log",
            Capability::HttpGet => "agent_host_http_get",
            Capability::HttpPost => "agent_host_http_post",
            Capability::FilesystemRead => "agent_host_fs_read",
            Capability::FilesystemWrite => "agent_host_fs_write",
            Capability::Shell => "agent_host_shell",
            Capability::MemoryRead => "agent_host_memory_search",
            Capability::MemoryWrite => "agent_host_memory_write",
        }
    }

    /// Secrets a plugin may name are granted per name, never in bulk.
    pub fn takes_secrets(self) -> bool {
        matches!(self, Capability::HttpGet | Capability::HttpPost)
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Resource ceilings. A plugin runs under limits whether or not it asked for
/// them; these are the ceiling, not the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Resources {
    /// Maximum linear memory, in mebibytes.
    pub memory_mb: u32,
    /// Execution budget, in Wasmtime fuel. One unit is roughly one instruction,
    /// so this is a wall-clock bound that cannot be exceeded by a tight loop.
    pub fuel: u64,
    /// Wall-clock ceiling, in milliseconds. Fuel alone does not bound a plugin
    /// that blocks on a host call, so this is separate.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    5_000
}

impl Default for Resources {
    /// Deliberately tight. A plugin that needs more asks, and a raise is a
    /// visible act rather than a default nobody chose.
    fn default() -> Self {
        Self {
            memory_mb: 64,
            fuel: 100_000_000,
            timeout_ms: 5_000,
        }
    }
}

impl Resources {
    /// Clamp a manifest's request to what the host will ever give. A manifest
    /// cannot raise its own ceiling by writing a bigger number.
    pub fn clamp(self) -> Self {
        let ceiling = Resources::default();
        Self {
            memory_mb: self.memory_mb.min(ceiling.memory_mb),
            fuel: self.fuel.min(ceiling.fuel),
            timeout_ms: self.timeout_ms.min(ceiling.timeout_ms),
        }
    }
}

/// A plugin's descriptor: what it asks for.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    /// Named secrets the plugin may ask the host to use. Meaningless without the
    /// matching capability, and checked for that below.
    #[serde(default)]
    pub secrets: Vec<String>,
    #[serde(default)]
    pub resources: Resources,
}

impl Manifest {
    pub fn parse(text: &str) -> std::result::Result<Self, ConfigError> {
        let manifest: Manifest =
            toml::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> std::result::Result<(), ConfigError> {
        let mut seen: Vec<Capability> = Vec::new();
        for capability in &self.capabilities {
            if seen.contains(capability) {
                return Err(ConfigError::Invalid(format!(
                    "plugin `{}` declares `{capability}` more than once",
                    self.name
                )));
            }
            seen.push(*capability);
        }

        // A secret without the capability that would use it is either a typo or
        // an attempt to have the secret handed over without the reach. Either
        // way the manifest is wrong and says so.
        if !self.secrets.is_empty()
            && !self
                .capabilities
                .iter()
                .any(|c| c.takes_secrets())
        {
            return Err(ConfigError::Invalid(format!(
                "plugin `{}` names secrets but requests no capability that uses them",
                self.name
            )));
        }

        if self.name.is_empty() || self.name.contains('/') {
            return Err(ConfigError::Invalid(format!(
                "plugin name `{}` must be non-empty and contain no `/`",
                self.name
            )));
        }
        Ok(())
    }

    pub fn requests(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }
}