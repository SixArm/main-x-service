//! The process-default TLS provider, installed before any HTTP client is built.
//!
//! `reqwest`'s `rustls-no-provider` feature brings no crypto backend and
//! panics when a client is built with none installed; `ring` is the family's
//! one backend. Every client in this crate is built through here.

/// Install `ring` as the process default. Only the first call does anything;
/// an `Err` means a provider is already installed, which is fine.
pub fn install_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// `reqwest::Client::new()` with the provider installed first.
#[must_use]
pub fn client() -> reqwest::Client {
    install_provider();
    reqwest::Client::new()
}

/// `reqwest::Client::builder()` with the provider installed first.
pub fn client_builder() -> reqwest::ClientBuilder {
    install_provider();
    reqwest::Client::builder()
}
