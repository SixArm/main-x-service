# Content Management System — Specification

This directory is the **single source of truth** for the
cross-cutting Content Management System (CMS) specification, shared
by both editions. Each subproject's own `spec/` adds stack-specific
detail and links back here.

> ⚠️ **Demo software, not a production CMS.** This project models
> content-management practice for demonstration and integration
> purposes; it serves no real public site and holds no real
> personal data. See [regulatory.md](regulatory.md).

## What this project is

A **headless content management system**: authors create, manage,
and publish digital content without writing code, and the published
result is delivered as structured JSON to any channel — web app,
mobile app, kiosk, screen — rather than as server-rendered HTML.
Six modules:

1. **Content modelling & authoring** — operator-defined content
   types, structured block editing, references, revisions.
2. **Digital asset management** — an asset library with metadata,
   tags, renditions, and usage tracking.
3. **Editorial workflow & governance** — draft → review → approved
   → published → archived, scheduling, locks, roles as policy.
4. **Localization & translation** — locale variants, fallback
   chains, translation status and staleness.
5. **Delivery, presentation & SEO** — sites, templates as declared
   contracts, routing/slugs/redirects, the delivery API, SEO
   artifacts, request-context personalization.
6. **Content insights** — derived editorial and content-health
   views (never stored KPIs).

It is a **consumer application** (the case-folder / patient-flow /
workforce-planning-management / contact-relationship-management
shape): it registers no identities. An author, editor, or
translator is a
[worker-service](../../worker/worker-service-with-loco/) record; a
site's owning body is an
[organization-service](../../organization/organization-service-with-loco/)
record. CMS owns only **content and its editorial state** — types,
entries, revisions, assets, workflow, routes, locales — always
referencing identities by `EntityRef` URN, never duplicating them.

**It also registers no visitors.** There is no per-reader profile
store, no tracking identity, and no audience table
([design.md](design.md) CMS-D1, CMS-D11).

## Two editions

| Subproject                                                                                     | Role              | Stack                                   |
| ---------------------------------------------------------------------------------------------- | ----------------- | --------------------------------------- |
| [content-management-system-service-with-rust](../content-management-system-service-with-rust/) | Back-end JSON API | Rust, Loco (Axum + SeaORM), PostgreSQL  |
| [content-management-system-front-end-with-svelte](../content-management-system-front-end-with-svelte/) | Authoring & editorial UI | SvelteKit 2, Svelte 5 runes, TypeScript |

## Specification (topic files)

| File                                     | Covers                                                                     |
| ---------------------------------------- | -------------------------------------------------------------------------- |
| [purpose.md](purpose.md)                 | Problem statement, goals, the six modules                                  |
| [scope.md](scope.md)                     | In/out of scope; the boundary with the identity services                   |
| [domain-model.md](domain-model.md)       | Site, ContentType, Entry, Revision, Asset, Menu, Redirect, Translation, …  |
| [authoring.md](authoring.md)             | Module 1: content types, structured blocks, references, revisions          |
| [assets.md](assets.md)                   | Module 2: the asset library, renditions, usage, upload safety              |
| [workflow.md](workflow.md)               | Module 3: editorial lifecycle, scheduling, locks, approvals                |
| [localization.md](localization.md)       | Module 4: locale variants, fallback, translation status and staleness      |
| [delivery.md](delivery.md)               | Module 5: sites, templates, routing, the delivery API, SEO, personalization |
| [insights.md](insights.md)               | Module 6: derived editorial and content-health views                       |
| [integrations.md](integrations.md)       | Upstream family services; EntityRef URNs; the artifact store               |
| [auth.md](auth.md)                       | SSO, ABAC personas (author / editor / translator / admin / delivery), preview tokens |
| [audit.md](audit.md)                     | Audit trail, events, revision history as evidence                          |
| [architecture.md](architecture.md)       | Editions, layering, pure-core rules, persistence                           |
| [testing.md](testing.md)                 | Test strategy per edition                                                  |
| [regulatory.md](regulatory.md)           | Demo status; accessibility, GDPR, and publishing-record posture            |
| [roadmap.md](roadmap.md)                 | Beyond the v1 queue                                                        |
| [glossary.md](glossary.md)               | Headless, block document, rendition, fallback chain, slug, …               |

## Specification-driven delivery (SDD)

Three lock-step files drive delivery:

- [requirements.md](requirements.md) — numbered requirements
  (`CMS-R*`) with user stories and acceptance criteria.
- [design.md](design.md) — numbered design decisions (`CMS-D*`).
- [tasks.md](tasks.md) — **the live delivery checklist**
  (`CMS-T*`), phased; every task traces to design and requirement
  ids.

A change starts in `requirements.md`, is shaped in `design.md`, is
queued in `tasks.md`, and only then lands as code in a subproject.
**No code lands without the spec describing it.**

## References

- Sibling consumer apps (the shape this follows):
  [contact-relationship-management](../../contact-relationship-management/spec/index.md),
  [workforce-planning-management](../../workforce-planning-management/spec/index.md),
  [patient-flow](../../patient-flow/spec/index.md),
  [case-folder](../../case-folder/spec/index.md)
- Family contracts:
  [authentication-sessions](../../agents/share/authentication-sessions.md),
  [authorization-attributes](../../agents/share/authorization-attributes.md),
  [security](../../agents/share/security.md),
  [bulk-import-export](../../agents/share/bulk-import-export.md)
  (the `ArtifactStore` seam the asset library reuses),
  [locales](../../agents/share/locales.md) (the locale vocabulary)
