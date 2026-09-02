//! Build script — compiles `proto/worker.proto` into
//! `crate::api::grpc::proto` via `tonic-build` (PRO-H11, following
//! person-service's reference implementation).
//!
//! `tonic_build::compile_protos` shells out to a bundled `protoc`, so no
//! system `protoc` install is required. `prost-build` does **not** emit
//! `cargo:rerun-if-changed` on its own (verified in person-service's own
//! `build.rs`, not assumed here), so this script declares it explicitly
//! — without it, editing the `.proto` after a first successful build
//! would silently keep using the stale `$OUT_DIR` codegen. The
//! generated code is included by `tonic::include_proto!("worker")` in
//! `src/api/grpc/mod.rs` — never checked in.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/worker.proto");
    tonic_build::compile_protos("proto/worker.proto")?;
    Ok(())
}
