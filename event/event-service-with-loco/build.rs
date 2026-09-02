//! Build script — compiles `proto/event.proto` into
//! `crate::api::grpc::proto` via `tonic-build` (PRO-H11, following
//! person-service's and worker-service's reference implementations).
//!
//! `tonic_build::compile_protos` shells out to a real `protoc` binary;
//! it does **not** bundle one itself. Person-service's and
//! worker-service's own `build.rs`, on their first landing, each
//! claimed otherwise and each broke CI (whose runner has no `protoc`
//! installed) while passing locally (where a developer's machine
//! already had `protoc` on `PATH`) — fixed there, and not repeated
//! here: `protoc-bin-vendored` supplies a `protoc` binary for the
//! platforms this family builds on, and `PROTOC` is pointed at it
//! before compiling, so no system `protoc` install is required
//! anywhere. `prost-build` does **not** emit `cargo:rerun-if-changed`
//! on its own (verified in person-service's own `build.rs`, not
//! assumed here), so this script declares it explicitly — without it,
//! editing the `.proto` after a first successful build would silently
//! keep using the stale `$OUT_DIR` codegen. The generated code is
//! included by `tonic::include_proto!("event")` in
//! `src/api/grpc/mod.rs` — never checked in.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/event.proto");
    // SAFETY (of the *deprecation*, not memory): `std::env::set_var` is
    // unsafe as of Rust 2024 because mutating the environment is not
    // thread-safe in a multi-threaded process. `build.rs` runs
    // single-threaded, before any of this crate's own code exists, so
    // there is no concurrent reader to race.
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    }
    tonic_build::compile_protos("proto/event.proto")?;
    Ok(())
}
