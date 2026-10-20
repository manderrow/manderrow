#![windows_subsystem = "windows"]
#![deny(unused_must_use)]

// Fetching a mod index makes millions of short-lived allocations while decoding hundreds of
// megabytes of JSON. glibc's allocator keeps most of that memory afterwards, fragmented across its
// per-thread arenas, while mimalloc returns it to the OS.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> anyhow::Result<()> {
    manderrow_lib::main()
}
