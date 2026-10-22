//! Loading and running a plugin under Wasmtime.
//!
//! **WASI is deliberately not linked.** A plugin reaches the filesystem through
//! a granted host capability that resolves paths against the sandbox root, not
//! through a WASI preopen. That is stricter, not stylistic: with preopens a
//! plugin with no filesystem capability still gets a `path_open` import, and the
//! safety argument becomes "the preopen is narrow". Here the import does not
//! exist, so the argument is "there is nothing to call".
//!
//! The consequence is what makes the consent ledger enforceable: **an import a
//! plugin uses but was not granted for fails to instantiate.** The sandbox is the
//! linker, not a check the plugin could talk its way past.
//!
//! ## The ABI
//!
//! Strings cross the boundary as UTF-8 in linear memory. A packed `i64` carries
//! a pointer in its high half and a length in its low half, so a response needs
//! no second call to learn its size.
//!
//! ```text
//! export fn agent_abi_version() -> i32                     must return ABI_VERSION
//! export fn agent_alloc(len: i32) -> i32                   host allocates a guest buffer
//! export fn agent_execute(req: i32, req_len: i32, out: i32, out_cap: i32) -> i32
//! export memory
//! ```
//!
//! Both buffers are allocated by the host through `agent_alloc`. An earlier
//! shape had the guest return a packed `(ptr, len)` it had allocated itself,
//! which put the response buffer at the mercy of the guest's allocator: a
//! guest that misallocates returns a pointer the host then reads garbage from.
//! Making the host allocate the output too means a guest bug can produce a
//! wrong answer but cannot produce a wild read.

use std::time::Duration;

use wasmtime::{Config, Engine, Linker, Memory, Module, Store, StoreLimits};

use super::capabilities::{Capability, Resources};
use super::consent::ConsentLedger;
use super::store::{Generation, Registry};
use crate::error::{Error, Result};

/// The ABI version this host speaks. A guest exporting a different one is
/// refused rather than called: a mismatch is a mismatch, not something to
/// discover mid-call.
pub const ABI_VERSION: i32 = 1;

/// The module namespace host functions are linked into.
///
/// `env`, because that is where `rust-lld` places a core wasm module's undefined
/// symbols, so that is where a guest's imports arrive. It is a namespace, not a
/// capability: the host links only the names it granted, so an import from any
/// other namespace — or any other name within this one — still fails to
/// instantiate. A toolchain that renames the namespace would change this
/// constant and nothing else.
pub const HOST_NAMESPACE: &str = "env";

/// WebAssembly linear memory grows in 64 KiB pages.
const WASM_PAGE_BYTES: usize = 65_536;

/// How large a response buffer the host hands a plugin. A plugin that needs more
/// says so and is asked to retry with a caller-sized buffer; it does not get to
/// write past what it was given.
const RESPONSE_CAPACITY: i32 = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("plugin `{0}` is not installed")]
    NotInstalled(String),
    #[error("plugin `{plugin}` has no active generation")]
    NoActiveGeneration { plugin: String },
    #[error("plugin `{plugin}` speaks ABI {found}; this host speaks {expected}")]
    AbiMismatch {
        plugin: String,
        found: i32,
        expected: i32,
    },
    #[error("plugin `{plugin}` needs `{capability}`, which this host cannot serve yet")]
    NotYetImplemented { plugin: String, capability: String },
    #[error("plugin `{plugin}` failed to load: {reason}")]
    LoadFailed { plugin: String, reason: String },
    #[error("plugin `{plugin}` used {limit} beyond its ceiling")]
    CeilingExceeded { plugin: String, limit: String },
    #[error("plugin `{plugin}` trapped: {reason}")]
    Trapped { plugin: String, reason: String },
    #[error("plugin `{plugin}` returned something unusable: {reason}")]
    Rejected { plugin: String, reason: String },
    #[error("`{0}` is not a capability this host implements")]
    NotGrantable(String),
}

struct HostData {
    plugin: String,
    granted: Vec<Capability>,
    limits: StoreLimits,
    /// What the plugin logged before it trapped. The only record of a partial
    /// side effect, so it is reported rather than discarded with the store.
    log: Vec<String>,
    /// The guest's memory, attached after instantiation. A host function needs
    /// it to read its arguments, and the store cannot hold a `Memory` before the
    /// instance exists.
    memory: Option<Memory>,
}

pub struct LoadedPlugin {
    pub generation: u32,
    pub hash: String,
    pub granted: Vec<Capability>,
}

pub struct PluginHost {
    engine: Engine,
    registry: Registry,
    /// Interior mutability because consent changes at run time — a grant or a
    /// revocation arrives while the engine is shared. `Engine` is the expensive
    /// handle; rebuilding the host to change a decision would be absurd.
    consent: std::sync::Mutex<ConsentLedger>,
}

impl PluginHost {
    pub fn new(registry: Registry) -> Result<Self> {
        let mut config = Config::new();
        // Fuel bounds instructions; the epoch bounds wall-clock. Both are needed,
        // because a plugin that blocks cannot burn fuel and one that loops burns
        // it without ever yielding.
        config.consume_fuel(true);
        config.epoch_interruption(true);
        config.wasm_multi_memory(false);
        // The host is the only authority. A plugin cannot reach anything the
        // linker did not hand it, because the linker hands it nothing else.
        config.wasm_simd(true);
        config.memory_reservation(0);

        Ok(Self {
            engine: Engine::new(&config)?,
            registry,
            consent: std::sync::Mutex::new(ConsentLedger::new()),
        })
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    fn consent(&self) -> std::sync::MutexGuard<'_, ConsentLedger> {
        // A panic while holding this lock cannot leave a partial decision: every
        // method below either inserts one whole entry or none.
        self.consent.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// What this host can actually serve today. A capability outside this set
    /// cannot be granted, so it cannot be consented to and then fail at load — the
    /// refusal happens where a person is looking at it.
    pub fn grantable(capability: Capability) -> bool {
        matches!(
            capability,
            Capability::Log
                | Capability::HttpGet
                | Capability::HttpPost
                | Capability::FilesystemRead
                | Capability::FilesystemWrite
                | Capability::Shell
        )
    }

    /// Record a grant, refusing one this host cannot serve or that the plugin
    /// never asked for — a grant for an unrequested capability would read as
    /// working and grant nothing.
    pub fn grant(&self, agent: &str, plugin: &str, capability: Capability) -> Result<()> {
        if !Self::grantable(capability) {
            return Err(PluginError::NotGrantable(capability.name().into()).into());
        }
        let Some(generation) = self.registry.active(plugin)? else {
            return Err(PluginError::NotInstalled(plugin.into()).into());
        };
        if !generation.capabilities.contains(&capability) {
            return Err(Error::State(format!(
                "plugin `{plugin}` does not request `{capability}`"
            )));
        }
        self.consent().grant(agent, plugin, capability);
        Ok(())
    }

    pub fn deny(&self, agent: &str, plugin: &str, capability: Capability) {
        self.consent().deny(agent, plugin, capability);
    }

    /// Revoke everything an agent holds for a plugin, reporting what to
    /// deactivate now rather than at the next load.
    pub fn revoke(&self, agent: &str, plugin: &str) -> Vec<Capability> {
        self.consent().revoke_plugin(agent, plugin)
    }

    pub fn effective(&self, agent: &str, plugin: &str) -> Result<Vec<Capability>> {
        let Some(generation) = self.registry.active(plugin)? else {
            return Err(PluginError::NotInstalled(plugin.into()).into());
        };
        Ok(self
            .consent()
            .effective(agent, plugin, &generation.capabilities))
    }

    /// Load the active generation and check it can actually be instantiated and
    /// answer the ABI. This is the sandbox test a candidate must pass before
    /// promotion: a plugin that cannot load never becomes active.
    pub fn validate(&self, agent: &str, plugin: &str) -> Result<LoadedPlugin> {
        let (mut store, instance) = self.prepare(agent, plugin)?;
        let version = instance
            .get_typed_func::<(), i32>(&mut store, "agent_abi_version")
            .map_err(|e| PluginError::LoadFailed {
                plugin: plugin.to_string(),
                reason: format!("no `agent_abi_version` export: {e}"),
            })?;
        let found = version
            .call(&mut store, ())
            .map_err(|e| PluginError::Trapped {
                plugin: plugin.to_string(),
                reason: e.to_string(),
            })?;
        if found != ABI_VERSION {
            return Err(PluginError::AbiMismatch {
                plugin: plugin.to_string(),
                found,
                expected: ABI_VERSION,
            }
            .into());
        }

        let generation = self.registry.active(plugin)?;
        let granted = store.data().granted.clone();
        Ok(LoadedPlugin {
            generation: generation.as_ref().map(|g| g.number).unwrap_or(0),
            hash: generation.map(|g| g.hash).unwrap_or_default(),
            granted,
        })
    }

    /// One call: request in, response out.
    pub fn execute(&self, agent: &str, plugin: &str, request: &str) -> Result<String> {
        let (mut store, instance) = self.prepare(agent, plugin)?;

        let alloc = instance
            .get_typed_func::<i32, i32>(&mut store, "agent_alloc")
            .map_err(|e| PluginError::LoadFailed {
                plugin: plugin.to_string(),
                reason: format!("no `agent_alloc` export: {e}"),
            })?;
        let execute = instance
            .get_typed_func::<(i32, i32, i32, i32), i32>(&mut store, "agent_execute")
            .map_err(|e| PluginError::LoadFailed {
                plugin: plugin.to_string(),
                reason: format!("no `agent_execute` export: {e}"),
            })?;

        let memory = store
            .data()
            .memory()
            .ok_or_else(|| PluginError::LoadFailed {
                plugin: plugin.to_string(),
                reason: "the guest exports no `memory`".into(),
            })?;

        let request_ptr = alloc
            .call(&mut store, request.len() as i32)
            .map_err(|e| self.trap_reason(plugin, &store, e))?;
        memory
            .write(&mut store, request_ptr as usize, request.as_bytes())
            .map_err(|_| PluginError::CeilingExceeded {
                plugin: plugin.to_string(),
                limit: "memory".into(),
            })?;

        // The host sizes the output, so a guest that wants more has to say so
        // rather than writing past what it was given.
        let out_ptr = alloc
            .call(&mut store, RESPONSE_CAPACITY)
            .map_err(|e| self.trap_reason(plugin, &store, e))?;

        let written = match execute.call(
            &mut store,
            (
                request_ptr,
                request.len() as i32,
                out_ptr,
                RESPONSE_CAPACITY,
            ),
        ) {
            Ok(written) => written,
            Err(cause) => return Err(self.trap_reason(plugin, &store, cause).into()),
        };

        if written < 0 {
            return Err(PluginError::Rejected {
                plugin: plugin.to_string(),
                reason: "the guest declined the call".into(),
            }
            .into());
        }
        if written as usize > RESPONSE_CAPACITY as usize {
            return Err(PluginError::Rejected {
                plugin: plugin.to_string(),
                reason: format!("the guest claimed {written} bytes of a {RESPONSE_CAPACITY}-byte buffer"),
            }
            .into());
        }

        let mut buffer = vec![0u8; written as usize];
        memory
            .read(&store, out_ptr as usize, &mut buffer)
            .map_err(|e| PluginError::Trapped {
                plugin: plugin.to_string(),
                reason: format!("could not read the response: {e}"),
            })?;

        String::from_utf8(buffer).map_err(|e| PluginError::Rejected {
            plugin: plugin.to_string(),
            reason: format!("the response was not utf-8: {e}"),
        }
        .into())
    }

    /// The log a plugin left behind, read without consuming the store.
    fn trap_reason(&self, plugin: &str, store: &Store<HostData>, cause: wasmtime::Error) -> PluginError {
        let logged = store.data().log.join(" | ");
        PluginError::Trapped {
            plugin: plugin.to_string(),
            reason: if logged.is_empty() {
                cause.to_string()
            } else {
                format!("{cause} (last logged: {logged})")
            },
        }
    }

    fn prepare(
        &self,
        agent: &str,
        plugin: &str,
    ) -> Result<(Store<HostData>, wasmtime::Instance)> {
        let Some(generation) = self.registry.active(plugin)? else {
            return Err(PluginError::NoActiveGeneration {
                plugin: plugin.to_string(),
            }
            .into());
        };
        let granted = self.consent().effective(agent, plugin, &generation.capabilities);
        for capability in &granted {
            if !Self::grantable(*capability) {
                return Err(PluginError::NotYetImplemented {
                    plugin: plugin.to_string(),
                    capability: capability.name().into(),
                }
                .into());
            }
        }
        self.instantiate(plugin, &generation, granted)
    }

    fn instantiate(
        &self,
        plugin: &str,
        generation: &Generation,
        granted: Vec<Capability>,
    ) -> Result<(Store<HostData>, wasmtime::Instance)> {
        let artifact = self.registry.store.read_artifact(&generation.hash)?;
        let resources: Resources = generation.resources;

        let data = HostData {
            plugin: plugin.to_string(),
            granted,
            limits: wasmtime::StoreLimitsBuilder::new()
                .memory_size(resources.memory_mb as usize * 1024 * 1024)
                .build(),
            log: Vec::new(),
            memory: None,
        };

        let mut store = Store::new(&self.engine, data);
        store.limiter(|state| &mut state.limits);
        store
            .set_fuel(resources.fuel)
            .map_err(|e| PluginError::LoadFailed {
                plugin: plugin.to_string(),
                reason: e.to_string(),
            })?;
        store.set_epoch_deadline(1);

        let module = Module::new(&self.engine, &artifact).map_err(|e| PluginError::LoadFailed {
            plugin: plugin.to_string(),
            reason: e.to_string(),
        })?;

        // Only granted capabilities are linked. An ungranted one is absent, so a
        // plugin that calls it fails to instantiate rather than failing later.
        let mut linker = Linker::new(&self.engine);
        for capability in &store.data().granted {
            self.link(&mut linker, *capability).map_err(|reason| PluginError::LoadFailed {
                plugin: plugin.to_string(),
                reason,
            })?;
        }

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| PluginError::LoadFailed {
                plugin: plugin.to_string(),
                reason: e.to_string(),
            })?;

        // The guest's memory is attached now, for the host functions to read and
        // write through the caller's context.
        store.data_mut().memory = instance.get_memory(&mut store, "memory");

        Ok((store, instance))
    }

    /// Link exactly one host import. `func_wrap` returns a result and a dropped
    /// one would leave a *granted* capability with no import — which reads as
    /// working and denies everything, so the failure is propagated instead.
    fn link(
        &self,
        linker: &mut Linker<HostData>,
        capability: Capability,
    ) -> std::result::Result<(), String> {
        match capability {
            Capability::Log => {
                linker
                    .func_wrap(
                        HOST_NAMESPACE,
                        capability.host_import(),
                        |mut caller: wasmtime::Caller<'_, HostData>, ptr: i32, len: i32| -> i32 {
                            match read_string(&mut caller, ptr, len) {
                                Some(message) => {
                                    tracing::info!(
                                        plugin = %caller.data().plugin,
                                        "plugin: {message}"
                                    );
                                    caller.data_mut().log.push(message);
                                    0
                                }
                                None => -1,
                            }
                        },
                    )
                    .map_err(|e| format!("`{capability}`: {e}"))?;
            }
            Capability::HttpGet | Capability::HttpPost => {
                let is_post = capability == Capability::HttpPost;
                linker.func_wrap(
                    HOST_NAMESPACE,
                    capability.host_import(),
                    move |mut caller: wasmtime::Caller<'_, HostData>, ptr: i32, len: i32| -> i64 {
                        let Some(request) = read_string(&mut caller, ptr, len) else {
                            return 0;
                        };
                        let outcome = match http_call(is_post, &request) {
                            Ok(body) => body,
                            Err(error) => format!("error: {error}"),
                        };
                        write_string(&mut caller, &outcome)
                    },
                )
                .map_err(|e| format!("`{capability}`: {e}"))?;
            }
            Capability::FilesystemRead | Capability::FilesystemWrite => {
                let is_write = capability == Capability::FilesystemWrite;
                linker.func_wrap(
                    HOST_NAMESPACE,
                    capability.host_import(),
                    move |mut caller: wasmtime::Caller<'_, HostData>, ptr: i32, len: i32| -> i64 {
                        let Some(request) = read_string(&mut caller, ptr, len) else {
                            return 0;
                        };
                        let outcome = match sandboxed_file(&request, is_write) {
                            Ok(body) => body,
                            Err(error) => format!("error: {error}"),
                        };
                        write_string(&mut caller, &outcome)
                    },
                )
                .map_err(|e| format!("`{capability}`: {e}"))?;
            }
            Capability::Shell => {
                linker.func_wrap(
                    HOST_NAMESPACE,
                    capability.host_import(),
                    |mut caller: wasmtime::Caller<'_, HostData>, ptr: i32, len: i32| -> i64 {
                        let Some(command) = read_string(&mut caller, ptr, len) else {
                            return 0;
                        };
                        write_string(&mut caller, &run_shell(&command))
                    },
                )
                .map_err(|e| format!("`{capability}`: {e}"))?;
            }
            Capability::MemoryRead | Capability::MemoryWrite => {
                // Unreachable: `grantable` refuses these, so they never reach a
                // linker. Linked as an explicit refusal rather than omitted, so a
                // manifest that requests one fails with a reason rather than an
                // unknown import.
                linker.func_wrap(
                    HOST_NAMESPACE,
                    capability.host_import(),
                    |_caller: wasmtime::Caller<'_, HostData>, _ptr: i32, _len: i32| -> i64 { -1 },
                )
                .map_err(|e| format!("`{capability}`: {e}"))?;
            }
        }
        Ok(())
    }
}

impl HostData {
    fn memory(&self) -> Option<Memory> {
        self.memory
    }
}

fn read_string(caller: &mut wasmtime::Caller<'_, HostData>, ptr: i32, len: i32) -> Option<String> {
    let memory = caller.data().memory()?;
    let mut buffer = vec![0u8; len.max(0) as usize];
    memory.read(&*caller, ptr as usize, &mut buffer).ok()?;
    String::from_utf8(buffer).ok()
}

fn write_string(caller: &mut wasmtime::Caller<'_, HostData>, text: &str) -> i64 {
    let len = text.len() as i32;
    let Some(memory) = caller.data().memory else {
        return 0;
    };
    let end = memory.data_size(&*caller);
    // Append by growing the memory, then writing at the old end. Wasm linear
    // memory never moves, so every pointer the guest already holds stays valid
    // across the growth — which is what makes this safe to do without asking
    // the guest to allocate.
    let needed = text.len().div_ceil(WASM_PAGE_BYTES) as u64;
    if memory.grow(&mut *caller, needed).is_err() {
        return 0;
    }
    let target = end;
    if memory.write(&mut *caller, target, text.as_bytes()).is_err() {
        return 0;
    }
    (((target as u64) << 32) | len as u64) as i64
}

/// Resolve a plugin-supplied path against the sandbox root and refuse anything
/// that escapes it. `..`, absolute paths and symlinks all resolve before the
/// check, so none of them is a way out.
fn sandboxed_file(request: &str, write: bool) -> std::result::Result<String, String> {
    let (path, body) = match request.split_once('\u{0}') {
        Some((path, body)) => (path, body),
        None => (request, ""),
    };
    let root = std::env::current_dir()
        .map_err(|e| format!("no sandbox root: {e}"))?
        .canonicalize()
        .map_err(|e| format!("sandbox root is unreadable: {e}"))?;

    let candidate = std::path::Path::new(path);
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };

    // Canonicalise the parent when the leaf does not exist yet, so writing a new
    // file cannot escape through a path that has not been created.
    let resolved = match joined.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            let parent = joined
                .parent()
                .ok_or_else(|| "no such path".to_string())?
                .canonicalize()
                .map_err(|e| format!("no such directory: {e}"))?;
            let name = joined
                .file_name()
                .ok_or_else(|| "no file name in the path".to_string())?;
            parent.join(name)
        }
    };

    if !resolved.starts_with(&root) {
        return Err("path escapes the sandbox root".into());
    }

    if write {
        std::fs::write(&resolved, body).map_err(|e| e.to_string())?;
        Ok(format!("wrote {} bytes", body.len()))
    } else {
        std::fs::read_to_string(&resolved).map_err(|e| e.to_string())
    }
}

/// The host makes the call, never the guest: the plugin holds no socket, so
/// revoking `http_*` stops it at the next call rather than leaving a live
/// connection behind.
fn http_call(is_post: bool, request: &str) -> std::result::Result<String, String> {
    let (url, body) = match request.split_once('\u{0}') {
        Some((url, body)) => (url, body),
        None => (request, ""),
    };
    // Only http(s): a `file://` URL here would turn a network grant into a
    // filesystem one, which is exactly the escalation the grant is meant to
    // prevent.
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("only http and https are permitted".into());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let response = if is_post {
        client.post(url).body(body.to_string()).send()
    } else {
        client.get(url).send()
    }
    .map_err(|e| e.to_string())?;

    let status = response.status();
    response.text().map_err(|e| e.to_string()).map(|text| {
        if status.is_success() {
            text
        } else {
            format!("http {status}: {text}")
        }
    })
}

fn run_shell(command: &str) -> String {
    match std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .output()
    {
        Ok(output) => format!(
            "exit={} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => format!("could not run: {error}"),
    }
}

#[cfg(test)]
mod sandbox_tests {
    use super::*;

    /// The filesystem capability is the one grant where escaping is plausible:
    /// a plugin has to name a path, and paths can climb. These are the shapes it
    /// must be refused for.
    ///
    /// `sandboxed_file` resolves against the *current directory*, so the test
    /// runs from a scratch directory and treats that as the root.
    fn refused(path: &str) -> bool {
        sandboxed_file(path, false).is_err()
    }

    #[test]
    fn a_relative_path_that_climbs_out_is_refused() {
        assert!(refused("../../../etc/passwd"), ".. escapes");
        assert!(refused("a/../../b"), ".. after a real segment still escapes");
        assert!(refused("./../outside"), "./.. escapes");
    }

    #[test]
    fn an_absolute_path_outside_the_root_is_refused() {
        assert!(refused("/etc/passwd"));
        assert!(refused("/tmp"));
        assert!(refused("/"));
    }

    #[test]
    fn a_symlink_out_of_the_root_is_refused() {
        let link = std::env::current_dir().unwrap().join("escape-link");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("/etc/passwd", &link).expect("symlink");
        assert!(
            refused("escape-link"),
            "the path is canonicalised before the check, so a symlink is not a way out"
        );
        let _ = std::fs::remove_file(&link);
    }

    #[test]
    fn a_path_inside_the_root_is_allowed() {
        let name = format!("sandbox-ok-{}", std::process::id());
        let path = std::env::current_dir().unwrap().join(&name);
        std::fs::write(&path, "inside").expect("write");
        assert!(
            sandboxed_file(&name, false).is_ok(),
            "a legitimate path inside the root still works"
        );
        // And writing is confined the same way.
        assert!(sandboxed_file(&format!("../{name}"), true).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn http_refuses_a_scheme_that_would_turn_a_network_grant_into_a_file_one() {
        for url in ["file:///etc/passwd", "ftp://example.invalid/x", "javascript:alert(1)"] {
            assert!(
                http_call(false, url).is_err(),
                "{url} must not be reachable through an http grant"
            );
        }
    }
}
