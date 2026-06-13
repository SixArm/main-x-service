# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added

- **Inaugural scaffold (v0.1.0).** SvelteKit 2 / Svelte 5 (runes) SPA
  for the Organization Service, copy-adapted from
  authentication-front-end-with-svelte (same loco raw-JSON client).
  - Routes: `/` (list), `/new` (create), `/[pid]` (detail + delete +
    check-duplicates), `/[pid]/edit` (edit).
  - Lean API client extended with `put`/`delete`; `OrganizationRepository`
    (list/get/create/update/remove/checkDuplicates).
  - `types.ts` mirrors `organization_matcher::Organization` (the service
    DTO), including `IdentifierScheme` and `PostalAddress`.
  - `OrganizationForm` editing scalars, comma-list fields, a postal
    address, and a simple identifiers editor (unit-variant schemes).
  - SPA mode (`+layout.ts`); dependency-light (no SVAR/Lily). `pnpm run
    check` clean (0/0); production build succeeds.

### Configuration

- `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).
