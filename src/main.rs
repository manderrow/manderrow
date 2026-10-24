#![windows_subsystem = "windows"]
#![deny(unused_must_use)]

// Fetching a mod index makes millions of short-lived allocations while decoding hundreds of
// megabytes of JSON. glibc's allocator keeps most of that memory afterwards, fragmented across its
// per-thread arenas, while mimalloc returns it to the OS.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// `mi_option_purge_delay`, which libmimalloc-sys has no constant for.
const MI_OPTION_PURGE_DELAY: libmimalloc_sys::mi_option_t = 15;

fn main() -> anyhow::Result<()> {
    // mimalloc returns freed memory to the OS a second after it's freed, and then only once it
    // next allocates or frees, which for the async runtime's threads, once idle, may be never.
    // Without the delay, it is returned as it's freed, which costs no measurable time.
    // SAFETY: setting an option takes no pointers
    unsafe { libmimalloc_sys::mi_option_set(MI_OPTION_PURGE_DELAY, 0) };

    manderrow_lib::main()
}
