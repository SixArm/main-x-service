## 16. Open Questions

Inherited from the subproject specs where noted; tracked here because
each one changes the **integration contract**, not just a crate's
internals.

- **OQ-1 — Refresh tokens vs. short-lived access tokens only?**
  (Service spec §16.) Today: access tokens only, default 1 h. Refresh
  tokens improve UX for long operator sessions but add server-side
  state and a second revocation surface. Millions of citizen users
  may tip the balance — re-authenticating via email every hour is
  hostile at that scale.
- **OQ-2 — Should revocation propagate to peers?** (Service spec §16.)
  Today signout is local; peers honour cached tokens until `exp`. A
  short-TTL deny-list endpoint (or pushed revocation events) would
  close the window at the cost of reintroducing a runtime dependency
  on the auth service — the thing offline verification exists to
  avoid. Governmental compliance review may force a decision.
- **OQ-3 — Audience model when peers need distinct audiences.**
  (Service spec §16.) Today every token carries the single audience
  `main-x-service`. Per-service audiences (`aud: person-service`)
  would scope tokens but complicate the front-end story (one token
  per service vs. one federation token).
- **OQ-4 — Where should the verifier live long-term?** It is currently
  a sibling crate inside this entity. If it is published to crates.io
  (Cargo.toml is package-ready and `cargo package --list` passes
  since §13 T-1), does the entity spec remain its contract authority, or does
  the crate grow its own §1–§25 library-style spec like the matcher
  crates?
- **OQ-5 — `localStorage` vs in-memory token storage in front-ends.**
  (Front-end spec §16.) XSS posture vs. refresh/UX. Whatever is
  decided here becomes the convention every sibling front-end copies.

Open questions resolve into §13 tasks or §5–§9 amendments when
decisions are made.
