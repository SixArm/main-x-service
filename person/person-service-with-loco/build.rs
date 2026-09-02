//! Build script — compiles `proto/person.proto` into
//! `crate::api::grpc::proto` via `tonic-build` (PRO-H11).
//!
//! `tonic_build::compile_protos` shells out to a bundled `protoc`
//! (through the `protoc-bin-vendored`-style dependency `tonic-build`
//! pulls in), so no system `protoc` install is required. `prost-build`
//! does **not** emit `cargo:rerun-if-changed` on its own (a `TODO` in
//! its own source says so — verified, not assumed), so this script
//! declares it explicitly; without it, editing the `.proto` after a
//! first successful build would silently keep using the stale
//! `$OUT_DIR` codegen until an unrelated source change forced a
//! rebuild. The generated code is included by
//! `tonic::include_proto!("person")` in `src/api/grpc/mod.rs` — never
//! checked in, exactly like every other prost/tonic codegen setup.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/person.proto");
    tonic_build::compile_protos("proto/person.proto")?;
    Ok(())
}
