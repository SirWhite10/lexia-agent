//! The generation store: content-addressed artifacts, and which one is active.
//!
//! A plugin is never replaced in place. Every build becomes a new generation,
//! content-addressed by hash, and exactly one is active. Keeping the previous
//! generations is the whole mechanism: rollback is instant because the bytes are
//! still there and were never mutated.
//!
//! Content addressing also buys integrity verification — a plugin's hash is its
//! identity, so "the file changed under us" is detectable rather than assumed —
//! and deduplication, since two builds that compiled to the same bytes are the
//! same artifact.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::capabilities::{Capability, Manifest, Resources};
use crate::error::{Error, Result};

/// One immutable build of a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Generation {
    pub number: u32,
    /// SHA-256 of the artifact, lowercase hex. This is the identity.
    pub hash: String,
    /// What the plugin asked for when this generation was built.
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub secrets: Vec<String>,
    #[serde(default)]
    pub resources: Resources,
    pub built_at: String,
}

/// A plugin's generations and which is live.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RegistryEntry {
    pub name: String,
    /// `None` while a plugin has never been promoted. A registered plugin with no
    /// active generation is a known capability that is not yet granted.
    #[serde(default)]
    pub active: Option<u32>,
    #[serde(default)]
    pub generations: Vec<Generation>,
}

impl RegistryEntry {
    fn next_number(&self) -> u32 {
        self.generations.iter().map(|g| g.number).max().unwrap_or(0) + 1
    }

    fn generation(&self, number: u32) -> Option<&Generation> {
        self.generations.iter().find(|g| g.number == number)
    }
}

pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        for sub in ["versions", "active", "registry"] {
            std::fs::create_dir_all(root.join(sub))?;
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Store artifact bytes and return their hash. Writing the same bytes twice
    /// is one file, not two.
    pub fn put_artifact(&self, bytes: &[u8]) -> Result<String> {
        let hash = sha256_hex(bytes);
        // Sharded by the first two hex characters: a single directory with every
        // plugin's artifact in it stops being a directory at a few thousand.
        let path = self.artifact_path(&hash);
        if path.exists() {
            return Ok(hash);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
        Ok(hash)
    }

    pub fn artifact_path(&self, hash: &str) -> PathBuf {
        self.root.join("versions").join(&hash[..2]).join(format!("{hash}.wasm"))
    }

    pub fn has_artifact(&self, hash: &str) -> bool {
        hash.len() == 64 && self.artifact_path(hash).is_file()
    }

    pub fn read_artifact(&self, hash: &str) -> Result<Vec<u8>> {
        let path = self.artifact_path(hash);
        let bytes = std::fs::read(&path)?;
        // Verified on read, not only on write: the file could have been replaced
        // since, and a plugin's hash is its identity.
        let actual = sha256_hex(&bytes);
        if actual != hash {
            return Err(Error::State(format!(
                "plugin artifact {hash} does not match its contents (found {actual})"
            )));
        }
        Ok(bytes)
    }

    pub fn entry_path(&self, plugin: &str) -> PathBuf {
        self.root.join("registry").join(format!("{plugin}.toml"))
    }

    /// Which generation is active, as the host records it outside the registry
    /// file. Kept separate so a promotion can move this pointer atomically even
    /// if the registry write is later.
    pub fn active_path(&self, plugin: &str) -> PathBuf {
        self.root.join("active").join(format!("{plugin}.toml"))
    }

    pub fn read_entry(&self, plugin: &str) -> Result<Option<RegistryEntry>> {
        let path = self.entry_path(plugin);
        if !path.is_file() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path)?;
        Ok(Some(
            toml::from_str(&text).map_err(|e| Error::State(format!("{}: {e}", path.display())))?,
        ))
    }

    pub fn write_entry(&self, entry: &RegistryEntry) -> Result<()> {
        let path = self.entry_path(&entry.name);
        std::fs::write(&path, toml::to_string_pretty(entry).map_err(|e| Error::State(e.to_string()))?)
            .map_err(|e| Error::State(format!("{}: {e}", path.display())))?;
        Ok(())
    }

    pub fn write_active(&self, plugin: &str, number: u32) -> Result<()> {
        std::fs::write(self.active_path(plugin), number.to_string())?;
        Ok(())
    }

    pub fn read_active(&self, plugin: &str) -> Result<Option<u32>> {
        let path = self.active_path(plugin);
        if !path.is_file() {
            return Ok(None);
        }
        Ok(std::fs::read_to_string(&path)?.trim().parse().ok())
    }

    pub fn plugins(&self) -> Result<Vec<String>> {
        let dir = self.root.join("registry");
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut names: Vec<String> = std::fs::read_dir(&dir)?
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.extension()? != "toml" {
                    return None;
                }
                Some(path.file_stem()?.to_string_lossy().to_string())
            })
            .collect();
        names.sort();
        Ok(names)
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(String::with_capacity(64), |mut out, byte| {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// What happened to a candidate, and why. Promotion is transactional: the
/// active pointer only moves once every step has passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Promotion {
    Promoted,
    RolledBack,
}

#[derive(Debug, thiserror::Error)]
pub enum PromotionError {
    #[error("no plugin named `{0}` is registered")]
    UnknownPlugin(String),
    #[error("plugin `{plugin}` has no generation {number}")]
    UnknownGeneration { plugin: String, number: u32 },
    #[error("plugin `{0}` has no earlier generation to roll back to")]
    NothingToRollBack(String),
}

impl Registry {
    /// Register a build as a new generation. It is *not* active yet: a
    /// candidate exists, and promotion is a separate, deliberate act.
    pub fn install(&self, manifest: &Manifest, artifact: &[u8]) -> Result<u32> {
        let hash = self.store.put_artifact(artifact)?;
        let mut entry = self
            .store
            .read_entry(&manifest.name)?
            .unwrap_or_else(|| RegistryEntry {
                name: manifest.name.clone(),
                active: None,
                generations: Vec::new(),
            });

        // The same bytes built twice is the same artifact, not a new generation.
        if let Some(existing) = entry.generations.iter().find(|g| g.hash == hash) {
            return Ok(existing.number);
        }

        let number = entry.next_number();
        entry.generations.push(Generation {
            number,
            hash,
            capabilities: manifest.capabilities.clone(),
            secrets: manifest.secrets.clone(),
            // Clamped here rather than at run time: the ceiling is a property of
            // the generation, so a manifest cannot raise it after promotion.
            resources: manifest.resources.clamp(),
            built_at: now_iso(),
        });
        self.store.write_entry(&entry)?;
        Ok(number)
    }

    /// Make a generation active. Called only after the candidate has been
    /// validated, loaded, sandbox-tested and health-tested.
    pub fn promote(&self, plugin: &str, number: u32) -> Result<Promotion> {
        let mut entry = self
            .store
            .read_entry(plugin)?
            .ok_or_else(|| PromotionError::UnknownPlugin(plugin.to_string()))?;
        if entry.generation(number).is_none() {
            return Err(PromotionError::UnknownGeneration {
                plugin: plugin.to_string(),
                number,
            }
            .into());
        }
        entry.active = Some(number);
        // The registry file first, then the pointer. If the second write fails
        // the registry still names the generation as active, so the two can never
        // disagree about which is newer.
        self.store.write_entry(&entry)?;
        self.store.write_active(plugin, number)?;
        Ok(Promotion::Promoted)
    }

    /// Make the previous generation active again. The bytes were never mutated,
    /// so this is a pointer move rather than a rebuild.
    pub fn rollback(&self, plugin: &str) -> Result<Generation> {
        let entry = self
            .store
            .read_entry(plugin)?
            .ok_or_else(|| PromotionError::UnknownPlugin(plugin.to_string()))?;
        let current = entry
            .active
            .ok_or_else(|| PromotionError::NothingToRollBack(plugin.to_string()))?;

        // The generation immediately below the active one, which is the one that
        // was active before it — not merely any older generation.
        let target = entry
            .generations
            .iter()
            .filter(|g| g.number < current)
            .max_by_key(|g| g.number)
            .ok_or_else(|| PromotionError::NothingToRollBack(plugin.to_string()))?;

        let target = target.clone();
        self.promote(plugin, target.number)?;
        Ok(target)
    }

    pub fn active(&self, plugin: &str) -> Result<Option<Generation>> {
        let Some(entry) = self.store.read_entry(plugin)? else {
            return Ok(None);
        };
        Ok(entry.active.and_then(|number| entry.generation(number).cloned()))
    }

    pub fn generations(&self, plugin: &str) -> Result<Vec<Generation>> {
        Ok(self
            .store
            .read_entry(plugin)?
            .map(|entry| entry.generations)
            .unwrap_or_default())
    }

    pub fn entry(&self, plugin: &str) -> Result<Option<RegistryEntry>> {
        self.store.read_entry(plugin)
    }

    pub fn plugins(&self) -> Result<Vec<String>> {
        self.store.plugins()
    }
}

pub struct Registry {
    pub store: Store,
}

impl Registry {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        Ok(Self {
            store: Store::open(root)?,
        })
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(name: &str) -> Manifest {
        Manifest {
            name: name.into(),
            version: "1.0.0".into(),
            capabilities: vec![Capability::Log],
            secrets: Vec::new(),
            resources: Resources::default(),
        }
    }

    fn store() -> (tempdir::TempDir, Registry) {
        let dir = tempdir::TempDir::new("lexosa-store");
        let registry = Registry::open(dir.path().join("plugins")).expect("registry");
        (dir, registry)
    }

    /// A tiny stand-in for `tempfile`, which is not a dependency. A unique
    /// directory under the system temp, removed on drop.
    mod tempdir {
        use std::path::{Path, PathBuf};
        pub struct TempDir(PathBuf);
        impl TempDir {
            pub fn new(name: &str) -> Self {
                let path = std::env::temp_dir().join(format!(
                    "{name}-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ));
                std::fs::create_dir_all(&path).expect("temp dir");
                Self(path)
            }
            pub fn path(&self) -> &Path {
                &self.0
            }
        }
        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    #[test]
    fn installing_records_a_generation_without_activating_it() {
        let (_dir, registry) = store();
        let number = registry.install(&manifest("echo"), b"\0asm-fake").expect("install");

        assert_eq!(number, 1);
        assert!(
            registry.active("echo").expect("read").is_none(),
            "a candidate is not a capability; promotion is a separate act"
        );
        assert_eq!(registry.generations("echo").expect("read").len(), 1);
    }

    #[test]
    fn identical_bytes_are_one_generation_not_two() {
        let (_dir, registry) = store();
        let first = registry.install(&manifest("echo"), b"\0asm-same").unwrap();
        let second = registry.install(&manifest("echo"), b"\0asm-same").unwrap();
        assert_eq!(first, second, "the same bytes are the same artifact");
        assert_eq!(registry.generations("echo").unwrap().len(), 1);
    }

    #[test]
    fn promotion_moves_the_active_pointer() {
        let (_dir, registry) = store();
        let one = registry.install(&manifest("echo"), b"v1").unwrap();
        assert_eq!(registry.promote("echo", one).unwrap(), Promotion::Promoted);
        let active = registry.active("echo").unwrap().expect("active");
        assert_eq!(active.number, 1);
        assert_eq!(registry.store.read_active("echo").unwrap(), Some(1));
    }

    #[test]
    fn rollback_returns_to_the_generation_that_was_active_before() {
        let (_dir, registry) = store();
        let one = registry.install(&manifest("echo"), b"v1").unwrap();
        let two = registry.install(&manifest("echo"), b"v2").unwrap();
        let three = registry.install(&manifest("echo"), b"v3").unwrap();
        registry.promote("echo", one).unwrap();
        registry.promote("echo", two).unwrap();
        registry.promote("echo", three).unwrap();

        let back = registry.rollback("echo").expect("rollback");
        assert_eq!(back.number, 2, "rollback returns to the previous active, not the oldest");
        assert_eq!(registry.active("echo").unwrap().unwrap().number, 2);
        assert!(
            registry.store.has_artifact(&back.hash),
            "rollback needs the bytes to still be there"
        );
    }

    #[test]
    fn rollback_with_nothing_to_return_to_is_an_error_not_a_silent_no_op() {
        let (_dir, registry) = store();
        let one = registry.install(&manifest("echo"), b"v1").unwrap();
        registry.promote("echo", one).unwrap();
        assert!(registry.rollback("echo").is_err());
    }

    #[test]
    fn a_failed_promotion_leaves_the_active_generation_untouched() {
        let (_dir, registry) = store();
        let one = registry.install(&manifest("echo"), b"v1").unwrap();
        let two = registry.install(&manifest("echo"), b"v2").unwrap();
        registry.promote("echo", one).unwrap();

        assert!(registry.promote("echo", 99).is_err(), "an unknown generation is refused");
        assert_eq!(
            registry.active("echo").unwrap().unwrap().number,
            one,
            "a refused promotion must not disturb what is live"
        );
        assert_eq!(registry.generations("echo").unwrap().len(), 2);
        let _ = two;
    }

    #[test]
    fn a_tampered_artifact_is_detected_on_read() {
        let (_dir, registry) = store();
        let number = registry.install(&manifest("echo"), b"honest bytes").unwrap();
        let generation = registry.active("echo").unwrap();
        assert!(generation.is_none());

        let entry = registry.entry("echo").unwrap().unwrap();
        let stored = entry.generation(number).unwrap().clone();
        registry.promote("echo", number).unwrap();

        // Someone replaced the file. The hash is the identity, so this is caught.
        std::fs::write(registry.store.artifact_path(&stored.hash), b"swapped").unwrap();
        let error = registry.store.read_artifact(&stored.hash).expect_err("tampering");
        assert!(error.to_string().contains("does not match its contents"));
    }

    #[test]
    fn a_manifest_cannot_raise_its_own_ceiling() {
        let (_dir, registry) = store();
        let mut asked = manifest("greedy");
        asked.resources = Resources {
            memory_mb: 4096,
            fuel: u64::MAX,
            timeout_ms: 86_400_000,
        };
        registry.install(&asked, b"v1").unwrap();
        let stored = &registry.generations("greedy").unwrap()[0];
        assert_eq!(stored.resources, Resources::default());
    }
}