//! `care-pathway-service` binary entrypoint — boots the loco.rs CLI
//! (`care-pathway-service`), wiring the `App` hooks and the database
//! migrator. See the crate library docs for the service overview.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use care_pathway_service::app::App;
use loco_rs::cli;
use migration::Migrator;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
