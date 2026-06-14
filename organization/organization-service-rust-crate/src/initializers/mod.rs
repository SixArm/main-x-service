//! Loco initializers.
//!
//! Reserved for app-startup wiring (loco's `Initializer` trait). Empty
//! today: auth verification and the event publisher are process-wide
//! `OnceLock`s rather than loco-managed state, so nothing needs an
//! initializer yet (see `App::initializers` in `app.rs`).
