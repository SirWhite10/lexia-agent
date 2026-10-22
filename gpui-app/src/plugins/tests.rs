//! The plugin host, proven against a real WebAssembly artifact.
//!
//! `guests/echo` is compiled to `wasm32-unknown-unknown` and loaded here for
//! real. Nothing in these tests is a stand-in: the artifact is a genuine wasm
//! module with one declared host import, and every assertion is about what the
//! runtime did with it.
//!
//! What these tests cannot show, and what the ticket keeps open: that the ABI is
//! language-neutral. The guest happens to be Rust. `clang` carries a `wasm32-wasi`
//! target on this host but no `wasm-ld` to link it, so a C guest cannot be built
//! here yet. Until one can, "language-neutral" is a design intent and not a
//! verified claim, and it is recorded as such on WF-IMP-021.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;
use crate::error::Error;

fn manifest(granted_asked: &[Capability]) -> Manifest {
    Manifest {
        name: "echo".into(),
        version: "0.1.0".into(),
        capabilities: granted_asked.to_vec(),
        secrets: Vec::new(),
        resources: Resources {
            memory_mb: 16,
            fuel: 50_000_000,
            timeout_ms: 2_000,
        },
    }
}

/// The guest's compiled artifact, built on demand so the test is self-contained
/// rather than depending on someone having run a build first.
fn guest_artifact() -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("guests/echo");
    let artifact = crate_dir
        .join("target/wasm32-unknown-unknown/release/lexosa_plugin_echo.wasm");
    if !artifact.is_file() {
        let status = Command::new("cargo")
            .args(["+nightly-2026-10-04", "build", "--release", "--target", "wasm32-unknown-unknown"])
            .current_dir(&crate_dir)
            .status()
            .expect("cargo is available to build the guest");
        assert!(status.success(), "the guest failed to build");
    }
    assert!(
        artifact.is_file(),
        "guest artifact missing at {}",
        artifact.display()
    );
    artifact
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("temp dir");
        Self(path)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn host(dir: &TempDir) -> PluginHost {
    PluginHost::new(Registry::open(dir.path().join("plugins")).expect("registry")).expect("host")
}

fn install_and_promote(host: &PluginHost, asked: &[Capability], artifact: &[u8]) -> u32 {
    let number = host.registry().install(&manifest(asked), artifact).expect("install");
    // Promotion is the deliberate act; validation would have failed before it.
    host.registry().promote("echo", number).expect("promote");
    number
}

#[test]
fn a_candidate_cannot_be_used_before_it_is_promoted() {
    let dir = TempDir::new("lexosa-plugin-candidate");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");

    let number = host.registry().install(&manifest(&[Capability::Log]), &artifact).expect("install");
    assert!(
        host.registry().active("echo").expect("read").is_none(),
        "a registered candidate grants nothing"
    );
    assert!(matches!(
        host.validate("lexosa", "echo"),
        Err(Error::Plugin(PluginError::NoActiveGeneration { .. }))
    ));
    assert_eq!(number, 1);
}

#[test]
fn a_promoted_plugin_that_was_never_granted_anything_will_not_load() {
    let dir = TempDir::new("lexosa-plugin-ungranted");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    install_and_promote(&host, &[Capability::Log], &artifact);

    // The plugin imports `lexosa_host_log`, which is granted. With consent
    // withheld it is not linked, so the module cannot instantiate.
    assert!(
        matches!(host.validate("lexosa", "echo"), Err(Error::Plugin(PluginError::LoadFailed { .. }))),
        "an ungranted import means the plugin cannot instantiate at all"
    );
    assert!(host.effective("lexosa", "echo").expect("effective").is_empty());
}

#[test]
fn a_granted_plugin_loads_and_answers() {
    let dir = TempDir::new("lexosa-plugin-load");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    install_and_promote(&host, &[Capability::Log], &artifact);

    host.grant("lexosa", "echo", Capability::Log).expect("grant");

    let loaded = host.validate("lexosa", "echo").expect("validates");
    assert_eq!(loaded.generation, 1);
    assert_eq!(loaded.granted, vec![Capability::Log]);

    let response = host
        .execute("lexosa", "echo", "hello from the host")
        .expect("executes");
    assert!(
        response.contains("hello from the host"),
        "the request crossed the boundary and came back: {response}"
    );
    assert!(
        response.contains("\"length\":19"),
        "and the guest really received 19 bytes: {response}"
    );
}

#[test]
fn revocation_stops_the_call_immediately() {
    let dir = TempDir::new("lexosa-plugin-revoke");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    install_and_promote(&host, &[Capability::Log], &artifact);
    host.grant("lexosa", "echo", Capability::Log).expect("grant");

    host.execute("lexosa", "echo", "before").expect("works while granted");

    let revoked = host.revoke("lexosa", "echo");
    assert_eq!(revoked, vec![Capability::Log]);
    assert!(
        host.execute("lexosa", "echo", "after").is_err(),
        "a revoked grant stops the next call, not the next restart"
    );
    assert!(
        host.execute("lexosa", "echo", "after").is_err(),
        "and stays stopped"
    );
}

#[test]
fn a_grant_does_not_carry_to_another_agent() {
    let dir = TempDir::new("lexosa-plugin-agents");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    install_and_promote(&host, &[Capability::Log], &artifact);

    host.grant("lexosa", "echo", Capability::Log).expect("grant");
    assert!(host.execute("lexosa", "echo", "hi").is_ok());

    assert!(
        host.execute("code-reviewer", "echo", "hi").is_err(),
        "a sub-agent does not inherit its parent's grant by existing"
    );
}

#[test]
fn a_capability_the_host_cannot_serve_cannot_be_granted() {
    let dir = TempDir::new("lexosa-plugin-unserved");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    install_and_promote(&host, &[Capability::MemoryRead], &artifact);

    // The memory layer does not exist yet, so this capability is refused where a
    // person can see it, rather than granted and failing at the call.
    assert!(matches!(
        host.grant("lexosa", "echo", Capability::MemoryRead),
        Err(Error::Plugin(PluginError::NotGrantable(_)))
    ));
}

#[test]
fn granting_something_the_plugin_never_asked_for_is_refused() {
    let dir = TempDir::new("lexosa-plugin-unasked");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    install_and_promote(&host, &[Capability::Log], &artifact);

    let error = host
        .grant("lexosa", "echo", Capability::Shell)
        .expect_err("a grant for an unrequested capability is refused");
    assert!(error.to_string().contains("does not request"));
    assert!(
        !host.effective("lexosa", "echo").unwrap().contains(&Capability::Shell),
        "and it grants nothing"
    );
}

#[test]
fn a_failed_candidate_leaves_the_active_generation_running() {
    let dir = TempDir::new("lexosa-plugin-rollback");
    let host = host(&dir);
    let good = std::fs::read(guest_artifact()).expect("artifact");

    let first = host.registry().install(&manifest(&[Capability::Log]), &good).expect("v1");
    host.registry().promote("echo", first).expect("promote v1");
    host.grant("lexosa", "echo", Capability::Log).expect("grant");
    host.execute("lexosa", "echo", "v1 works").expect("v1 runs");

    // A candidate that is not a module at all.
    let broken = host
        .registry()
        .install(&manifest(&[Capability::Log]), b"this is not wasm")
        .expect("install broken");
    assert!(
        host.registry().promote("echo", broken).is_ok(),
        "promotion is a pointer move; it does not load"
    );
    assert!(
        host.execute("lexosa", "echo", "after a bad build").is_err(),
        "and the broken generation fails loudly"
    );

    let back = host.registry().rollback("echo").expect("rollback");
    assert_eq!(back.number, first);
    assert_eq!(host.registry().active("echo").unwrap().unwrap().number, first);
    assert!(
        host.execute("lexosa", "echo", "restored").is_ok(),
        "rollback needs no rebuild, because the bytes were never mutated"
    );
}

#[test]
fn declaring_an_import_the_plugin_never_requested_is_an_error() {
    let dir = TempDir::new("lexosa-plugin-bare");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    // The manifest asks for nothing, yet the guest still declares
    // `lexosa_host_log`. Because no capability was granted, no such import was
    // linked, so instantiation fails. Declaring an import you were not granted
    // is an error rather than a silent no-op.
    install_and_promote(&host, &[], &artifact);
    assert!(matches!(
        host.validate("lexosa", "echo"),
        Err(Error::Plugin(PluginError::LoadFailed { .. }))
    ));
}
