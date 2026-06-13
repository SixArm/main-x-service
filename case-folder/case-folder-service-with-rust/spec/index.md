# Case Tracking (Loco edition) — Specification

Living specification for the **Loco / Axum / Postgres** JSON API
implementation of Case Tracking. Single source of truth for
spec-driven development of this subproject.

> ⚠️ **Demo software.** Not a regulated medical record. The
> [regulatory considerations](regulatory.md) (same as the Svelte
> sibling) apply before any live use.

This subproject is **back-end only**. There is no built-in user
interface — front-ends (web, mobile, internal tooling, other services)
consume the API over HTTP/JSON. The
[Svelte sibling](../../case-folder-front-end-with-svelte/spec/index.md) provides
a reference UI.

The cross-cutting domain (entities, NHS Number rules, invariants, use
cases, regulatory frame) lives in the
[root specification](../../spec/index.md). This subproject **mirrors**
that domain and adds **stack-specific** detail: Loco hooks, route
shapes, JSON contracts, upstream-service client interfaces.

## Specification (topic files)

| File                               | Covers                                                          |
| ---------------------------------- | --------------------------------------------------------------- |
| [purpose.md](purpose.md)           | Why this back-end exists                                        |
| [scope.md](scope.md)               | In / out of scope for the API                                   |
| [stack.md](stack.md)               | Frameworks, versions, pins, the Loco-version caveat             |
| [domain-model.md](domain-model.md) | Five-service split + every upstream Client trait                |
| [auth.md](auth.md)                 | Magic-link endpoints, JWT, mailer, session guard               |
| [nhs-number.md](nhs-number.md)     | Modulus 11 in Rust                                              |
| [architecture.md](architecture.md) | Layered diagram, Loco lifecycle, file layout                    |
| [routes.md](routes.md)             | Use-case → route mapping + the full route table                 |
| [api-contract.md](api-contract.md) | JSON conventions, envelopes, error shapes, soft-fail policy     |
| [database.md](database.md)         | Postgres (vestigial), migrations, stub mode, seed task          |
| [examples.md](examples.md)         | `curl` + Rust recipes                                           |
| [testing.md](testing.md)           | Unit + request test inventory, CI gates                         |
| [regulatory.md](regulatory.md)     | Regulatory considerations + production security gates           |
| [roadmap.md](roadmap.md)           | Loco-edition roadmap                                            |
| [glossary.md](glossary.md)         | Loco / Rust terms                                               |

## Specification-driven delivery (SDD)

| File                               | Role                                                         |
| ---------------------------------- | ------------------------------------------------------------ |
| [requirements.md](requirements.md) | API requirements + acceptance criteria (trace to root FR/NFR) |
| [design.md](design.md)             | Loco-specific design decisions                               |
| [tasks.md](tasks.md)               | API delivery checklist                                       |

## References

- [Loco docs](https://loco.rs/docs) · [Loco GitHub](https://github.com/loco-rs/loco)
- [SeaORM book](https://www.sea-ql.org/SeaORM/docs/)
- [Root specification](../../spec/index.md) · [Svelte sibling](../../case-folder-front-end-with-svelte/spec/index.md)
- [UK NHS Number](https://en.wikipedia.org/wiki/NHS_number) · [NHS Synthetic Data](https://digital.nhs.uk/services/synthetic-data)
