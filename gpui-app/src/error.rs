//! The error type the whole host speaks.
//!
//! One enum rather than a `Box<dyn Error>` per layer, because the failure modes
//! that matter here are few and specific: a credential that is missing, a
//! manifest that is wrong, a provider that said no. Anything more precise than
//! that is a provider's own error text, which travels as data.

use std::fmt;

pub type Result<T, E = Error> = std::result::Result<T, E>;


#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The manifest could not be read or parsed. Carries the path, because an
    /// operator fixing a manifest needs to know which one.
    #[error("{path}: {source}")]
    Config {
        path: String,
        #[source]
        source: ConfigError,
    },

    /// No credential for this provider, from neither the environment nor the
    /// stored file.
    #[error("no credential for provider `{provider}`: set {provider}_API_KEY or store one")]
    MissingCredential { provider: String },

    /// The provider answered, and the answer was not usable.
    #[error("provider `{provider}` returned {status}: {body}")]
    ProviderStatus {
        provider: String,
        status: u16,
        body: String,
    },

    /// The provider could not be reached at all.
    #[error("provider `{provider}` is unreachable: {reason}")]
    ProviderUnreachable { provider: String, reason: String },

    /// A use-case named a provider that is not configured.
    #[error("use-case `{use_case}` names provider `{provider}`, which is not configured")]
    UnknownProvider { use_case: String, provider: String },

    /// The run was asked for and does not exist. Distinct from a host failure
    /// so the HTTP layer can answer 404 rather than 500: a client that asks for a
    /// run it has lost the id of should be told so.
    #[error("no run `{id}`")]
    UnknownRun { id: String },

    /// The HTTP client could not be built. Separate from an unreachable
    /// provider: nothing was contacted, and the caller is not looking at a
    /// provider id at that point.
    #[error("could not build an HTTP client: {0}")]
    HttpClient(String),

    /// A plugin failed in a way the host reports as such. Carried as its own
    /// variant so the caller can tell "the plugin refused" from "the host broke",
    /// which are different things to act on.
    #[error(transparent)]
    Plugin(#[from] crate::plugins::PluginError),

    #[error(transparent)]
    Promotion(#[from] crate::plugins::store::PromotionError),

    /// The WebAssembly runtime could not compile or instantiate something.
    #[error("wasm runtime: {0}")]
    Wasm(String),

    #[error("state directory is not usable: {0}")]
    State(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<wasmtime::Error> for Error {
    fn from(error: wasmtime::Error) -> Self {
        Self::Wasm(error.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not be read: {0}")]
    Read(String),
    #[error("is not valid TOML: {0}")]
    Parse(String),
    #[error("{0}")]
    Invalid(String),
}

/// A credential that will not print itself.
///
/// `Debug` is what leaks: an accidental `dbg!(credential)` or a `{:?}` in an
/// error chain would put the key in a log. This type makes that impossible to do
/// by accident, so a `Secret` can be passed around freely without anyone having
/// to remember not to log it.
pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Length is safe to show and useful for diagnosing a truncated paste;
        // the value is not.
        write!(f, "Secret(<{} bytes>)", self.0.len())
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        // Best effort: overwrite before the allocation goes away, so a core dump
        // is less likely to carry the key. Not guaranteed — the allocator may
        // have copied the bytes — but it costs nothing.
        let bytes = unsafe { self.0.as_bytes_mut() };
        bytes.fill(0);
    }
}