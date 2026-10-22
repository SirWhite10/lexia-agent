//! A guest that asks for the microphone.
//!
//! It exists to prove one thing: that `audio_capture` is a real capability and
//! not a name in an enum. It imports `agent_host_audio_capture`, which the host
//! links only when an operator granted that capability to the agent. Without
//! the grant the import does not exist, so the module fails to *instantiate* —
//! the sandbox is the absence of the import, not a check inside it.
//!
//! `no_std` and `alloc` only, like every guest: nothing here is shared with
//! the host, and every value crosses the boundary as bytes in linear memory.

#![no_std]

//! A real plugin, built to `wasm32-unknown-unknown`.
//!
//! Nothing here is shared with the host. There is no generated binding, no
//! common crate, no agreed `Result` layout — every value crosses the boundary as
//! UTF-8 in linear memory with a pointer and a length. That is the point of
//! writing it this way: if the ABI could only be spoken by Rust, this file would
//! need something the host provided.
//!
//! `no_std` and `alloc` only, so the artifact carries no runtime. The allocator
//! is a bump over `memory.grow`, which is all the ABI needs and keeps the
//! artifact small. The host builds a fresh store per call, so nothing is
//! reclaimed within an instantiation.
//!
//! It imports exactly one host function, `agent_host_log`, because the manifest
//! requests `log` and nothing else. An import for an unrequested capability is
//! not linked, so calling one would fail to *instantiate* — the sandbox is the
//! absence of the import, not a check inside it.

extern crate alloc;

use alloc::alloc::handle_alloc_error;
use alloc::format;
use alloc::string::{String, ToString};
use core::alloc::{GlobalAlloc, Layout};

/// Bump allocator. Each allocation grows linear memory to cover the next aligned
/// slot; nothing is reused, because an instantiation is short-lived.
struct Bump;

/// WebAssembly linear memory grows in 64 KiB pages.
const PAGE_SIZE: usize = 65_536;

unsafe impl GlobalAlloc for Bump {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // `memory_size` counts *pages*; the cursor is a byte offset. Treating the
        // page count as bytes puts every allocation within a few bytes of the
        // start of memory, and they overwrite one another.
        let current = core::arch::wasm32::memory_size(0) * PAGE_SIZE;
        let aligned = current.next_multiple_of(layout.align());
        let end = aligned + layout.size();
        let pages = end.div_ceil(PAGE_SIZE) - current / PAGE_SIZE;
        if core::arch::wasm32::memory_grow(0, pages) == usize::MAX {
            handle_alloc_error(layout);
        }
        aligned as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Nothing to do: memory is discarded with the store.
    }
}

#[global_allocator]
static ALLOCATOR: Bump = Bump;

/// A panicking plugin traps. That is the correct outcome — the host learns the
/// call failed and the run is marked accordingly, rather than the plugin
/// continuing in a state nobody can describe.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}

/// The ABI version the host expects.
#[unsafe(no_mangle)]
pub extern "C" fn agent_abi_version() -> i32 {
    1
}

/// Allocate for the host to write into.
#[unsafe(no_mangle)]
pub extern "C" fn agent_alloc(len: i32) -> i32 {
    let len = len.max(0) as usize;
    let layout = Layout::from_size_align(len.max(1), 1).expect("a non-zero layout");
    unsafe { ALLOCATOR.alloc(layout) as i32 }
}

// Granted by the host because the manifest asked for `audio_capture`. It is not
// linked otherwise, so naming it here is itself the request.
extern "C" {
    fn agent_host_audio_capture(ptr: i32, len: i32) -> i32;
}

/// How many samples to ask for. Small enough to fit a page, large enough to
/// carry a few frames of speech.
const SAMPLE_COUNT: usize = 512;

/// One call: ask the host for audio and report what came back.
///
/// # Safety
///
/// The host guarantees `out`/`out_cap` are a writable range in this memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agent_execute(
    _req: i32,
    _req_len: i32,
    out: i32,
    out_cap: i32,
) -> i32 {
    let mut buffer = [0.0_f32; SAMPLE_COUNT];
    // The host writes mono f32 at the capture rate into this buffer and returns
    // the number of bytes it wrote. A negative value is a refusal, not silence.
    let written = agent_host_audio_capture(buffer.as_mut_ptr() as i32, (SAMPLE_COUNT * 4) as i32);
    if written < 0 {
        return -1;
    }

    let samples = (written as usize) / 4;
    let peak = buffer[..samples.min(SAMPLE_COUNT)]
        .iter()
        .fold(0.0_f32, |acc, value| acc.max(value.abs()));

    let mut response = String::new();
    response.push_str("{\"bytes\":");
    response.push_str(&written.to_string());
    response.push_str(",\"samples\":");
    response.push_str(&samples.to_string());
    response.push_str(",\"peak\":");
    response.push_str(&format!("{peak:.6}"));
    response.push('}');

    let bytes = response.as_bytes();
    if bytes.len() > out_cap.max(0) as usize {
        return -1;
    }
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, bytes.len());
    bytes.len() as i32
}
