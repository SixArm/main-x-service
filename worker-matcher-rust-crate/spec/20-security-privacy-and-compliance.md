## 20. Security, Privacy, and Compliance

No IO (library reads no files, makes no network calls, opens no sockets). No logging of PII (no logging in library code at all). No global state (no thread-locals, no `static mut`, no lazy_statics carrying worker data). Memory hygiene — input strings are caller-owned; the library borrows; no zeroing because the library does not hold PII beyond a single call. GDPR — the library is a pure function; consumer applications carry GDPR responsibility for records they pass in. Safety — no algorithm is perfect (§5); consumers MUST treat probabilistic matches as recommendations, not decisions. Full guidance in [`AGENTS/security-and-privacy.md`](../AGENTS/security-and-privacy.md).

---

