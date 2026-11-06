//! `care-pathway-service` binary entrypoint — boots the axa CLI
//! (`care-pathway-service`), wiring the `App` hooks and the database
//! migrator. See the crate library docs for the service overview.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use axa::cli;
use care_pathway_service::app::App;

/// Parse the command line and run it.
///
/// # Errors
///
/// Returns the failure from [`App::run`].
#[tokio::main]
async fn main() -> axa::Result<()> {
    App::run(cli::parse(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION"),
    ))
    .await
}
