//! Binary entry point for the Thing Service.
//!
//! The substantive functionality of this crate lives in the library
//! ([`thing_service`](../thing_service/index.html)): models, matching,
//! validation, privacy, and metrics. This binary is a thin placeholder; the
//! production REST + gRPC server wiring is provided elsewhere in the
//! deployment. It exists so that `cargo run` has a target and so the crate
//! can be built as an executable.

// Use the high-performance MiMalloc allocator for MUSL static builds only.
// On glibc/macOS targets the system allocator is used, so this attribute is
// compiled out; it matters for the stripped, statically-linked release
// container image.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Process entry point. Currently a placeholder for the server bootstrap.
fn main() {
    println!("Hello, world!");
}
