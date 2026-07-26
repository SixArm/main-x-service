//! Background workers (loco `BackgroundWorker`).

/// FHIR Bulk Data `$export` worker: materialises a queued job to the
/// artifact store (`spec/compliance` §12.3).
pub mod bulk_export;
pub mod downloader;
