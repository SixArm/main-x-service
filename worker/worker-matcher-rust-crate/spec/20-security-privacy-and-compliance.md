## 20. Security, Privacy, and Compliance

No IO (library reads no files, makes no network calls, opens no sockets). No logging of PII (no logging in library code at all). No global state (no thread-locals, no `static mut`, no lazy_statics carrying worker data). Memory hygiene — input strings are caller-owned; the library borrows; no zeroing because the library does not hold PII beyond a single call. GDPR — the library is a pure function; consumer applications carry GDPR responsibility for records they pass in. Safety — no algorithm is perfect (§5); consumers MUST treat probabilistic matches as recommendations, not decisions. Full guidance in [`agents/security-and-privacy.md`](../agents/security-and-privacy.md).

**No spurious identity (SEC-M2 / SEC-M3).** A deterministic short-circuit
to `1.0` requires both sides to carry a non-empty, well-formed value —
never a shared blank/sentinel. `passport_books_share_pair` skips a pair
whose `country` or `number` is blank; the demographic-tuple fallback
requires both names to have a non-empty normalised form; the format-only
national-identifier parsers (`parse_ie_ihi`, `parse_es_tsi`,
`parse_dk_cpr`) reject all-zeros sentinels. This is the crate's instance
of the family-wide no-spurious-identity invariant
(`agents/share/security.md` §3, invariant 4).

**Supply chain.** `deny.toml` gates dependency advisories and licences
via `cargo-deny` (SEC-I1); `#![forbid(unsafe_code)]` in `lib.rs` (SEC-I3,
per `agents/security-and-privacy.md` above). Fuzz coverage for untrusted
input is §18.6 (SEC-I2).

---

