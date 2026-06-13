# Spec-Driven Development — Authentication Entity

This entity practises **spec-driven development** with a two-level
authority model. Read this before editing anything under
`authentication/`.

## Authority model

- Each subproject's own `spec/` is the single source of truth **for
  that subproject's internals** — the
  [service spec](../authentication-service-rust-crate/spec/index.md)
  and the
  [front-end spec](../authentication-front-end-with-svelte/spec/index.md).
- The entity-level spec ([`../spec/index.md`](../spec/index.md)) is
  the source of truth for the **cross-subproject contract**: the
  magic-link protocol, the JWT claim set, the JWKS shape, the
  verifier-library API, and entity-wide goals.
- Disagreement about **crate internals** → the crate spec wins.
  Disagreement about the **integration contract** → the entity spec
  wins. Either way, open a task (entity spec §13 or the crate's §13)
  to reconcile — never silently rewrite.
- The verifier crate has its own spec
  ([spec/index.md](../authentication-verifier-rust-crate/spec/index.md)):
  authoritative for crate internals; the entity spec stays
  authoritative for the integration contract.

## Three-part PRs

A behavioural change is one PR: **spec edit + code edit + test edit.**
A contract change (claims, JWKS, endpoints, verifier API) usually
touches *two* specs — the entity spec and the owning crate's spec —
plus code and tests on **both sides** of the contract (service signs,
verifier verifies, front-end consumes).

## When to update which entity-spec section

| You're changing… | Update entity spec section… |
|---|---|
| JWT claim set, token lifetime, `kid` derivation | §5.3–§5.5, §4 |
| JWKS document shape or path | §5.4, §9 |
| Magic-link request / redeem behaviour | §6.1–§6.3 |
| Session / revocation semantics | §6.4, §5.2 |
| Verifier public API or error taxonomy | §6.5, §9.2 |
| Front-end flow or storage keys | §6.6, §10.4 |
| Availability / key-rotation / rate-limit targets | §7 |
| Issuance / verification flow or deployment shape | §8 |
| Database schema | §10 |
| Compliance scope | §12 |
| Adding work | §13 (new `T-N` entry) |
| Completing work | tick §13 + the relevant CHANGELOG |
| Open-question resolution | move from §16 into the relevant section |

## Anti-patterns (entity-specific)

- Changing `Claims` in the service without the mirror edit in the
  verifier (they MUST stay byte-compatible) — this is exactly the
  drift the entity spec exists to prevent.
- Reintroducing HS256 or a shared secret "just for tests".
- Logging a magic-link token or a JWT in production code.
- Breaking the always-`200` anti-enumeration shape while adding rate
  limiting.
- Giving the verifier a dependency it doesn't strictly need — peers
  embed it; its footprint is a contract feature (entity spec NFR-12).

## Document hierarchy

```
authentication/
├── spec/                 ← entity contract + goals (this level)
├── AGENTS/               ← entity reference docs (you are here)
├── authentication-service-rust-crate/
│   ├── spec/             ← service internals (authoritative for the crate)
│   └── AGENTS.md, README.md, CHANGELOG.md
├── authentication-verifier-rust-crate/
│   ├── spec/             ← verifier internals (authoritative for the crate)
│   └── AGENTS.md, README.md, CHANGELOG.md
└── authentication-front-end-with-svelte/
    ├── spec/             ← front-end internals (authoritative)
    └── AGENTS.md, README.md, CHANGELOG.md
```

There is intentionally no `plan.md` and no `tasks.md` at any level:
plan content lives in spec §8–§12, tasks in §13, status / roadmap in
§14–§15, open questions in §16.
