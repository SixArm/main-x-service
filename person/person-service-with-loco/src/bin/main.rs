//! Person Service binary entry point.
//!
//! Boots the loco.rs application (Axum REST API, `SeaORM` persistence,
//! matching, search, streaming, observability) via the loco CLI. See
//! [`person_service::app::App`] for the wiring and `lib.rs` for the
//! supported library surface.

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

// When we build for MUSL static, use faster memory allocator.
/// Process-wide global allocator for MUSL static builds.
///
/// MUSL's bundled allocator is slow under the service's concurrent
/// alloc/free pattern, so MUSL targets swap in `mimalloc`. Gated on
/// `target_env = "musl"` so glibc/dev builds keep the system allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use axa::cli::Cli;
use clap::Parser;
use person_service::app::App;

/// Parse the command line and run it.
///
/// # Errors
///
/// Returns the failure from [`App::run`].
#[tokio::main]
async fn main() -> axa::Result<()> {
    App::run(Cli::parse()).await
}
