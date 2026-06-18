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

use loco_rs::cli;
use migration::Migrator;
use person_service::app::App;

/// Binary entry point: hand control to the loco CLI.
///
/// `cli::main::<App, Migrator>()` parses the loco subcommands
/// (`start`, `db migrate`, `task`, …) and dispatches against this
/// crate's [`App`] hooks and the migration crate's `Migrator`. Returns
/// loco's `Result` so a boot/CLI failure propagates as a non-zero exit.
#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
