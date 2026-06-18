//! Loco CLI entrypoint for the authentication-service single sign-on provider.
//!
//! Boots the loco `App` (routes, workers, migrations) via `cli::main`.
//! See the crate library docs and `spec/index.md` for the service shape.
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use authentication_service::app::App;
use authentication_service::migration::Migrator;
use loco_rs::cli;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
