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
        .join("target/wasm32-unknown-unknown/release/agent_plugin_echo.wasm");
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
    let dir = TempDir::new("agent-plugin-candidate");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");

    let number = host.registry().install(&manifest(&[Capability::Log]), &artifact).expect("install");
    assert!(
        host.registry().active("echo").expect("read").is_none(),
        "a registered candidate grants nothing"
    );
    assert!(matches!(
        host.validate("agent", "echo"),
        Err(Error::Plugin(PluginError::NoActiveGeneration { .. }))
    ));
    assert_eq!(number, 1);
}

#[test]
fn a_promoted_plugin_that_was_never_granted_anything_will_not_load() {
    let dir = TempDir::new("agent-plugin-ungranted");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    install_and_promote(&host, &[Capability::Log], &artifact);

    // The plugin imports `agent_host_log`, which is granted. With consent
    // withheld it is not linked, so the module cannot instantiate.
    assert!(
        matches!(host.validate("agent", "echo"), Err(Error::Plugin(PluginError::LoadFailed { .. }))),
        "an ungranted import means the plugin cannot instantiate at all"
    );
    assert!(host.effective("agent", "echo").expect("effective").is_empty());
}

#[test]
fn a_granted_plugin_loads_and_answers() {
    let dir = TempDir::new("agent-plugin-load");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    install_and_promote(&host, &[Capability::Log], &artifact);

    host.grant("agent", "echo", Capability::Log).expect("grant");

    let loaded = host.validate("agent", "echo").expect("validates");
    assert_eq!(loaded.generation, 1);
    assert_eq!(loaded.granted, vec![Capability::Log]);

    let response = host
        .execute("agent", "echo", "hello from the host")
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
    let dir = TempDir::new("agent-plugin-revoke");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    install_and_promote(&host, &[Capability::Log], &artifact);
    host.grant("agent", "echo", Capability::Log).expect("grant");

    host.execute("agent", "echo", "before").expect("works while granted");

    let revoked = host.revoke("agent", "echo");
    assert_eq!(revoked, vec![Capability::Log]);
    assert!(
        host.execute("agent", "echo", "after").is_err(),
        "a revoked grant stops the next call, not the next restart"
    );
    assert!(
        host.execute("agent", "echo", "after").is_err(),
        "and stays stopped"
    );
}

#[test]
fn a_grant_does_not_carry_to_another_agent() {
    let dir = TempDir::new("agent-plugin-agents");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    install_and_promote(&host, &[Capability::Log], &artifact);

    host.grant("agent", "echo", Capability::Log).expect("grant");
    assert!(host.execute("agent", "echo", "hi").is_ok());

    assert!(
        host.execute("code-reviewer", "echo", "hi").is_err(),
        "a sub-agent does not inherit its parent's grant by existing"
    );
}

#[test]
fn a_capability_the_host_cannot_serve_cannot_be_granted() {
    let dir = TempDir::new("agent-plugin-unserved");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    install_and_promote(&host, &[Capability::MemoryRead], &artifact);

    // The memory layer does not exist yet, so this capability is refused where a
    // person can see it, rather than granted and failing at the call.
    assert!(matches!(
        host.grant("agent", "echo", Capability::MemoryRead),
        Err(Error::Plugin(PluginError::NotGrantable(_)))
    ));
}

#[test]
fn granting_something_the_plugin_never_asked_for_is_refused() {
    let dir = TempDir::new("agent-plugin-unasked");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    install_and_promote(&host, &[Capability::Log], &artifact);

    let error = host
        .grant("agent", "echo", Capability::Shell)
        .expect_err("a grant for an unrequested capability is refused");
    assert!(error.to_string().contains("does not request"));
    assert!(
        !host.effective("agent", "echo").unwrap().contains(&Capability::Shell),
        "and it grants nothing"
    );
}

#[test]
fn a_failed_candidate_leaves_the_active_generation_running() {
    let dir = TempDir::new("agent-plugin-rollback");
    let host = host(&dir);
    let good = std::fs::read(guest_artifact()).expect("artifact");

    let first = host.registry().install(&manifest(&[Capability::Log]), &good).expect("v1");
    host.registry().promote("echo", first).expect("promote v1");
    host.grant("agent", "echo", Capability::Log).expect("grant");
    host.execute("agent", "echo", "v1 works").expect("v1 runs");

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
        host.execute("agent", "echo", "after a bad build").is_err(),
        "and the broken generation fails loudly"
    );

    let back = host.registry().rollback("echo").expect("rollback");
    assert_eq!(back.number, first);
    assert_eq!(host.registry().active("echo").unwrap().unwrap().number, first);
    assert!(
        host.execute("agent", "echo", "restored").is_ok(),
        "rollback needs no rebuild, because the bytes were never mutated"
    );
}

#[test]
fn declaring_an_import_the_plugin_never_requested_is_an_error() {
    let dir = TempDir::new("agent-plugin-bare");
    let host = host(&dir);
    let artifact = std::fs::read(guest_artifact()).expect("artifact");
    // The manifest asks for nothing, yet the guest still declares
    // `agent_host_log`. Because no capability was granted, no such import was
    // linked, so instantiation fails. Declaring an import you were not granted
    // is an error rather than a silent no-op.
    install_and_promote(&host, &[], &artifact);
    assert!(matches!(
        host.validate("agent", "echo"),
        Err(Error::Plugin(PluginError::LoadFailed { .. }))
    ));
}

/// The audio guest's compiled artifact, built on demand like the echo guest's.
fn audio_guest_artifact() -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("guests/audio");
    let artifact = crate_dir
        .join("target/wasm32-unknown-unknown/release/agent_plugin_audio.wasm");
    if !artifact.is_file() {
        let status = Command::new("cargo")
            .args([
                "+nightly-2026-10-04",
                "build",
                "--release",
                "--target",
                "wasm32-unknown-unknown",
            ])
            .current_dir(&crate_dir)
            .status()
            .expect("cargo is available to build the guest");
        assert!(status.success(), "the audio guest failed to build");
    }
    assert!(
        artifact.is_file(),
        "audio guest artifact missing at {}",
        artifact.display()
    );
    artifact
}

/// A capture source that hands over a fixed tone, so the microphone path can be
/// proven on a machine with no microphone — and so the test is deterministic.
struct FakeCapture {
    samples: Vec<f32>,
    rate: u32,
}

impl crate::audio::CaptureSource for FakeCapture {
    fn start(
        &mut self,
        sink: std::sync::Arc<std::sync::Mutex<crate::audio::CaptureBuffer>>,
    ) -> std::result::Result<u32, crate::audio::AudioError> {
        let mut buffer = sink.lock().expect("the capture buffer is not poisoned");
        buffer.push_mono(&self.samples, self.rate);
        Ok(self.rate)
    }

    fn stop(&mut self) {}
}

fn host_with_capture(dir: &TempDir, samples: Vec<f32>, rate: u32) -> PluginHost {
    let capture = std::sync::Arc::new(crate::audio::AudioCapture::with_source(Box::new(
        FakeCapture { samples, rate },
    )));
    PluginHost::with_capture(
        Registry::open(dir.path().join("plugins")).expect("registry"),
        capture,
    )
    .expect("host")
}

fn manifest_named(name: &str, asked: &[Capability]) -> Manifest {
    Manifest {
        name: name.into(),
        ..manifest(asked)
    }
}

fn install_and_promote_named(
    host: &PluginHost,
    name: &str,
    asked: &[Capability],
    artifact: &[u8],
) -> u32 {
    let number = host
        .registry()
        .install(&manifest_named(name, asked), artifact)
        .expect("install");
    host.registry().promote(name, number).expect("promote");
    number
}

#[test]
fn a_plugin_that_asked_for_the_microphone_cannot_load_until_it_is_granted() {
    let dir = TempDir::new("agent-plugin-audio-grant");
    let host = host(&dir);
    let artifact = std::fs::read(audio_guest_artifact()).expect("artifact");
    install_and_promote_named(&host, "audio", &[Capability::AudioCapture], &artifact);

    // The guest imports `agent_host_audio_capture`. Without the grant the host
    // does not link it, so the module cannot instantiate at all. This is the
    // whole enforcement story: the capability is the absence of the import.
    assert!(
        matches!(
            host.validate("agent", "audio"),
            Err(Error::Plugin(PluginError::LoadFailed { .. }))
        ),
        "an ungranted import means the plugin cannot instantiate"
    );
    assert!(host.effective("agent", "audio").expect("effective").is_empty());

    // With the grant, the same artifact loads.
    host.grant("agent", "audio", Capability::AudioCapture).expect("grant");
    let loaded = host.validate("agent", "audio").expect("the granted plugin loads");
    assert_eq!(loaded.granted, vec![Capability::AudioCapture]);
}

#[test]
fn a_granted_microphone_reaches_the_plugin() {
    let dir = TempDir::new("agent-plugin-audio-call");
    // One second of a full-scale tone at 48 kHz, which the host resamples to
    // the capture rate before the guest sees it.
    let host = host_with_capture(&dir, vec![1.0_f32; 48_000], 48_000);
    let artifact = std::fs::read(audio_guest_artifact()).expect("artifact");
    install_and_promote_named(&host, "audio", &[Capability::AudioCapture], &artifact);
    host.grant("agent", "audio", Capability::AudioCapture).expect("grant");

    let response = host
        .execute("agent", "audio", "capture")
        .expect("executes");
    assert!(
        response.contains("\"samples\":512"),
        "the guest received the samples it asked for: {response}"
    );
    assert!(
        response.contains("\"peak\":1.000000"),
        "and the audio really crossed the boundary: {response}"
    );
}

#[test]
fn a_microphone_grant_does_not_carry_to_another_agent() {
    let dir = TempDir::new("agent-plugin-audio-agents");
    let host = host_with_capture(&dir, vec![1.0_f32; 48_000], 48_000);
    let artifact = std::fs::read(audio_guest_artifact()).expect("artifact");
    install_and_promote_named(&host, "audio", &[Capability::AudioCapture], &artifact);

    host.grant("agent", "audio", Capability::AudioCapture).expect("grant");
    assert!(host.execute("agent", "audio", "capture").is_ok());

    assert!(
        host.execute("code-reviewer", "audio", "capture").is_err(),
        "a sub-agent does not inherit its parent's microphone by existing"
    );
}

#[test]
fn a_microphone_that_cannot_be_opened_is_reported_rather_than_silent() {
    let dir = TempDir::new("agent-plugin-audio-busy");
    struct BusyCapture;
    impl crate::audio::CaptureSource for BusyCapture {
        fn start(
            &mut self,
            _sink: std::sync::Arc<std::sync::Mutex<crate::audio::CaptureBuffer>>,
        ) -> std::result::Result<u32, crate::audio::AudioError> {
            Err(crate::audio::AudioError::DeviceBusy(
                "the wake service holds the device".into(),
            ))
        }
        fn stop(&mut self) {}
    }
    let capture = std::sync::Arc::new(crate::audio::AudioCapture::with_source(Box::new(
        BusyCapture,
    )));
    let host = PluginHost::with_capture(
        Registry::open(dir.path().join("plugins")).expect("registry"),
        capture,
    )
    .expect("host");
    let artifact = std::fs::read(audio_guest_artifact()).expect("artifact");
    install_and_promote_named(&host, "audio", &[Capability::AudioCapture], &artifact);
    host.grant("agent", "audio", Capability::AudioCapture).expect("grant");

    // The guest gets a refusal, not a run of zeros that would read as a quiet
    // room — and the refusal carries the reason the host refused.
    let error = host
        .execute("agent", "audio", "capture")
        .expect_err("a device that could not be opened did not deliver samples");
    let message = error.to_string();
    assert!(
        message.contains("audio capture failed"),
        "the reason the host refused reached the caller: {message}"
    );
    assert!(
        message.contains("in use"),
        "and it names the cause rather than saying the guest declined: {message}"
    );
}
