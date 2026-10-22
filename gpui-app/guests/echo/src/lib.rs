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

#![no_std]

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

/// The ABI version the host expects. A mismatch is refused at load rather than
/// discovered mid-call.
#[unsafe(no_mangle)]
pub extern "C" fn agent_abi_version() -> i32 {
    1
}

/// Allocate for the host to write into. The host writes `len` bytes here
/// immediately afterwards, so the memory must already cover them.
#[unsafe(no_mangle)]
pub extern "C" fn agent_alloc(len: i32) -> i32 {
    let len = len.max(0) as usize;
    let layout = Layout::from_size_align(len.max(1), 1).expect("a non-zero layout");
    unsafe { ALLOCATOR.alloc(layout) as i32 }
}

/// One call. Writes the response into the buffer the host allocated and returns
/// how many bytes it wrote, or -1.
///
/// # Safety
///
/// The host guarantees `req`/`req_len` are readable and `out`/`out_cap` are a
/// writable range in this memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agent_execute(
    req: i32,
    req_len: i32,
    out: i32,
    out_cap: i32,
) -> i32 {
    let request = read(req, req_len);

    // A deliberately small transformation: this plugin proves the boundary, it
    // is not where capability comes from.
    let mut response = String::with_capacity(request.len() + 32);
    response.push_str("{\"echo\":");
    push_json_string(&mut response, &request);
    response.push_str(",\"length\":");
    response.push_str(&request.len().to_string());
    response.push('}');

    let bytes = response.as_bytes();
    if bytes.len() > out_cap.max(0) as usize {
        // The host sized this buffer; a response that does not fit is reported
        // rather than truncated, because a truncated JSON document is worse than
        // an honest refusal.
        return -1;
    }
    log(&format!("echo received {} bytes", request.len()));
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, bytes.len());
    bytes.len() as i32
}

// Granted by the host because the manifest asked for `log` and only `log`.
extern "C" {
    fn agent_host_log(ptr: i32, len: i32) -> i32;
}

fn log(message: &str) {
    unsafe {
        let len = message.len() as i32;
        let ptr = agent_alloc(len);
        core::ptr::copy_nonoverlapping(message.as_ptr(), ptr as *mut u8, len as usize);
        agent_host_log(ptr, len);
    }
}

unsafe fn read(ptr: i32, len: i32) -> String {
    let bytes = core::slice::from_raw_parts(ptr as *const u8, len.max(0) as usize);
    String::from_utf8_lossy(bytes).into_owned()
}

/// Enough JSON for a string field. A plugin returning a number, an array or a
/// nested object would need more here, and that is the plugin's problem rather
/// than the ABI's.
fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}