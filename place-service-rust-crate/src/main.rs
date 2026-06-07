//! Binary entry point for the place-service.
//!
//! The substantive functionality of this crate lives in the library
//! (`place_service`); this binary is a thin shell. In production builds the
//! REST/gRPC servers, database pool, search index, and event publisher are
//! wired up here, but the current stub merely confirms the binary links and
//! runs. See the library crate root ([`place_service`](../place_service/index.html))
//! for the domain model, matching, validation, and privacy modules.

/// Use the high-performance [MiMalloc](https://github.com/microsoft/mimalloc)
/// allocator for statically-linked MUSL builds (the production container
/// target), where the default system allocator is comparatively slow.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Process entry point.
///
/// Currently a placeholder; real deployments boot the Axum server, SeaORM
/// pool, Tantivy index, and event stream from the library crate.
fn main() {
    println!("Hello, world!");
}
