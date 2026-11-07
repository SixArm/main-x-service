//! Date/time conversions across the persistence boundary.
//!
//! Domain models and the row structs in [`super::models`] both use
//! `chrono` (`DateTime<Utc>`), which is what sqlx maps `TIMESTAMPTZ` to, so
//! reads and writes need no conversion. Two places still speak the `time`
//! crate: the outbox's pure envelope→row mapping
//! ([`super::outbox::OutboxInsert`]) is stamped with a
//! `time::OffsetDateTime`, and the audit row's `timestamp` is serialized in
//! `time`'s form (see [`super::models::AuditLogRow`]). These helpers
//! translate between the two.

use chrono::{DateTime, Utc};
use time::OffsetDateTime;

/// `chrono::DateTime<Utc>` → `time::OffsetDateTime` (UTC).
///
/// Both types carry nanosecond UTC instants, so the conversion is
/// lossless.
///
/// # Panics
///
/// Panics if the `chrono` instant falls outside `time::OffsetDateTime`'s
/// representable range. In practice both ranges cover real-world event
/// timestamps, so this cannot fire for stored event data.
#[must_use]
pub fn ts_to_offset(ts: DateTime<Utc>) -> OffsetDateTime {
    let nanos =
        i128::from(ts.timestamp()) * 1_000_000_000 + i128::from(ts.timestamp_subsec_nanos());
    OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .expect("chrono DateTime within time::OffsetDateTime range")
}

/// `time::OffsetDateTime` → `chrono::DateTime<Utc>`.
///
/// # Panics
///
/// Panics if the `time` instant falls outside `chrono::DateTime`'s
/// representable range (see [`ts_to_offset`]); unreachable for stored
/// event data.
#[must_use]
pub fn offset_to_ts(odt: OffsetDateTime) -> DateTime<Utc> {
    DateTime::from_timestamp(odt.unix_timestamp(), odt.nanosecond())
        .expect("time::OffsetDateTime within chrono DateTime range")
}
