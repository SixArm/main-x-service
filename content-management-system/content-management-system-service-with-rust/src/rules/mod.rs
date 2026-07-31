//! The **pure core** (CMS-D4, CMS-D13): DB-free, exhaustively
//! unit-tested logic that controllers wire but never re-implement.
//!
//! Landed so far (Phase 1):
//!
//! - [`tokens`] — the closed string vocabularies.
//! - [`locale`] — locale-code shape, the site's declared locale set,
//!   and fallback-chain validity (CMS-R1, CMS-R14).
//! - [`schema`] — content-type field-schema validation and the
//!   `additive | tightening | breaking` compatibility classifier
//!   (CMS-R2).
//! - [`template`] — the declared region contracts a channel lays out
//!   (CMS-R1, CMS-D6).
//! - [`block`] — block-document validation, and the sanitization pass
//!   that runs before storage (CMS-R4).
//! - [`sanitize`] — the HTML allow-list behind it (CMS-D5).
//! - [`reference`] — the edges extracted from every saved revision,
//!   which make "where used" and delete-refusal possible (CMS-R5).
//! - [`diff`] — what one save changed, for the revision history
//!   (CMS-R3).
//! - [`media`] — upload typing: sniffing, the accepted-format
//!   allow-list, and intrinsic dimensions (CMS-R6).
//! - [`gate`] — what stands between a variant and publication
//!   (CMS-R11).
//! - [`lifecycle`] — the editorial transition table (CMS-R9).
//! - [`staleness`] — how far a translation has fallen behind its
//!   source (CMS-R15).
//! - [`path`] — path normalization and bounded, loop-free redirect
//!   resolution (CMS-R17).
//! - [`audience`] — personalization over an allow-listed request
//!   context, with the keys it consulted (CMS-R20).
//! - [`seo`] — canonical URLs, sitemaps, and `robots.txt`, derived
//!   from what is actually published (CMS-R19).
//! - [`insight`] — the arithmetic behind the dashboards, and the
//!   honesty rules that keep it defensible (CMS-R21).
//! - [`preview`] — minting, hashing, and honouring the one credential
//!   that shows unpublished content (CMS-R22).
//! - [`webhook`] — signing, URL policy, and retry/backoff for the only
//!   extension mechanism this service has (CMS-R23).
//!

pub mod audience;
pub mod block;
pub mod diff;
pub mod gate;
pub mod insight;
pub mod lifecycle;
pub mod locale;
pub mod media;
pub mod path;
pub mod preview;
pub mod reference;
pub mod sanitize;
pub mod schema;
pub mod seo;
pub mod staleness;
pub mod template;
pub mod tokens;
pub mod webhook;
