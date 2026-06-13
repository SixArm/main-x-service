# Case Tracking (Svelte edition) — Specification

Living specification for the SvelteKit **front-end** that consumes the
[Loco JSON API sibling](../../case-folder-service-with-rust/spec/index.md).
Single source of truth for spec-driven development of this subproject.

> ⚠️ **Demo software.** Not a regulated medical record. The
> [regulatory considerations](regulatory.md) (same as the API sibling)
> apply before any live use.

This subproject is a **browser client**. It owns no data; every page
fetches from `/api/*` on the back-end via `src/lib/api/client.ts`,
hydrates a rune-reactive cache (`src/lib/store/cache.svelte.ts`), and
renders. Mutations round-trip through the same client.

The cross-cutting domain (entities, NHS Number rules, invariants, use
cases, regulatory frame) lives in the
[root specification](../../spec/index.md). This subproject mirrors that
domain and adds **front-end-specific** detail: the load+cache wiring,
the cache API, UI conventions, theming/locale, and accessibility.

## Specification (topic files)

| File                                 | Covers                                                       |
| ------------------------------------ | ------------------------------------------------------------ |
| [purpose.md](purpose.md)             | Why this client exists                                       |
| [scope.md](scope.md)                 | In / out of scope for the UI                                 |
| [stack.md](stack.md)                 | SvelteKit, Lily, SVAR, why CSR-only                          |
| [domain-model.md](domain-model.md)   | The camelCase client types                                   |
| [auth.md](auth.md)                   | Magic-link login UI, session cookie, dev proxy, guard        |
| [nhs-number.md](nhs-number.md)       | Modulus 11 client-side pre-flight                            |
| [architecture.md](architecture.md)   | Layered diagram, load+cache wiring, error policy, file layout |
| [routes.md](routes.md)               | Use-case → route mapping, negative cases, route+API table    |
| [cache-api.md](cache-api.md)         | The reactive cache singleton + mutation contracts            |
| [ui-conventions.md](ui-conventions.md) | Badges, forms, theming, locale                             |
| [accessibility.md](accessibility.md) | WCAG 2.2 AA checklist                                        |
| [examples.md](examples.md)           | Loader, reactive read, mutation, add-a-route recipes         |
| [testing.md](testing.md)             | svelte-check + Playwright e2e inventory                      |
| [regulatory.md](regulatory.md)       | Regulatory + client-side security gates                      |
| [roadmap.md](roadmap.md)             | Svelte-edition roadmap                                       |
| [glossary.md](glossary.md)           | Svelte / SvelteKit / Lily terms                              |

## Specification-driven delivery (SDD)

| File                                 | Role                                                         |
| ------------------------------------ | ------------------------------------------------------------ |
| [requirements.md](requirements.md)   | UI requirements + acceptance criteria (trace to root FR/NFR) |
| [design.md](design.md)               | Svelte-specific design decisions                             |
| [tasks.md](tasks.md)                 | UI delivery checklist                                        |

## References

- [Svelte 5 docs](https://svelte.dev/docs) · [SvelteKit 2 docs](https://svelte.dev/docs/kit)
- [Lily Design System](https://lilydesignsystem.io) · [SVAR Svelte](https://svar.dev/svelte/)
- [Root specification](../../spec/index.md) · [Loco JSON API sibling](../../case-folder-service-with-rust/spec/index.md)
- [UK NHS Number](https://en.wikipedia.org/wiki/NHS_number) · [GOV.UK Design System](https://design-system.service.gov.uk/)
