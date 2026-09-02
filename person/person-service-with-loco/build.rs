//! Build script — compiles `proto/person.proto` into
//! `crate::api::grpc::proto` via `tonic-build` (PRO-H11).
//!
//! `tonic_build::compile_protos` shells out to a real `protoc` binary;
//! it does **not** bundle one itself (an earlier version of this
//! comment claimed it did — wrong, and only CI caught it, since a
//! local `cargo build` succeeded here via a `protoc` already on
//! `PATH`). `protoc-bin-vendored` supplies one for the platforms this
//! family builds on, so `PROTOC` is set explicitly before compiling —
//! no system `protoc` install required anywhere, verified against a
//! real CI run rather than assumed from a local success. `prost-build`
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
    // SAFETY (of the *deprecation*, not memory): `std::env::set_var` is
    // unsafe as of Rust 2024 because mutating the environment is not
    // thread-safe in a multi-threaded process. `build.rs` runs
    // single-threaded, before any of this crate's own code exists, so
    // there is no concurrent reader to race.
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    }
    tonic_build::compile_protos("proto/person.proto")?;
    Ok(())
}
