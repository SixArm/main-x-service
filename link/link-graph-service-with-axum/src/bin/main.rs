//! `link-graph-service` binary entrypoint — boots the axa CLI,
//! wiring the [`App`] hooks and the database migrator. See the crate
//! library docs for the service overview.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use axa::cli;
use link_graph_service::app::App;

/// Parse the command line and run it.
///
/// # Errors
///
/// Returns the failure from [`App::run`].
#[tokio::main]
async fn main() -> axa::Result<()> {
    // One crypto backend, explicitly: ring. Only a `fluvio` build has a second
    // one (aws-lc-rs, forced by fluvio-future) compiled in to choose between.
    #[cfg(feature = "fluvio")]
    let _ = rustls::crypto::ring::default_provider().install_default();
    App::run(cli::parse(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION"),
    ))
    .await
}
