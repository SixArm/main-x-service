## 20. Security, Privacy, and Compliance

No IO (no file / network / socket). No logging of PII (no logging in library code at all). No global state (no thread-locals, no `static mut`, no lazy_statics holding person data). Memory hygiene: input strings are caller-owned; the library borrows them and holds no PII beyond a single call (no zeroing required). GDPR: the library is a pure function; consumers carry GDPR responsibility for the records they pass in. Safety: per §5, no algorithm is perfect — consumers MUST treat probabilistic matches as recommendations, not decisions.

---

