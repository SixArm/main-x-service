# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [README.md](./README.md) — user-facing intro. This crate has no
> `spec/`, `AGENTS.md`, or `CLAUDE.md` — it is a small, single-file value-type
> crate; the rustdoc in `src/lib.rs` plus this file and the README are the
> complete documentation surface.

## [Unreleased]

> Reconstructed 2026-08-04 (DOC-5) from `git log` — this crate previously
> shipped with no `CHANGELOG.md` at all.

### Added

- Supply-chain scanning: `deny.toml` (RUSTSEC advisories, a permissive-
  license allow-list, warn-level bans/sources) and
  `.github/workflows/security.yml` (`cargo deny check` on push / PR /
  weekly cron), matching the family-wide SEC-I1 rollout. No behaviour
  change.

### Changed

- Bumped `uuid` `1.23` → `1.24`. No behaviour change.

## [0.1.0] - 2026-07-08

### Added

- **Inaugural release.** The shared cross-service-linking contract
  (`agents/share/cross-service-linking.md` §3, §9) — rollout step 1
  ("land the contracts; no behaviour yet"):
  - `EntityType` — the ten globally-unique entity discriminators
    (`person`, `worker`, `organization`, `case`, `place`, `thing`,
    `event`, `course`, `courseinstance`, `care_pathway`), with `as_str`,
    `from_token`, and the `service()` map (`course` and `courseinstance`
    both resolve to `course-service`, since a multi-entity service is
    why the ref encodes the *type*, not the service).
  - `EntityRef` — `{entity_type, id: Uuid}`, parsing / `Display` /
    serde as the single URN string `"<entity_type>:<uuid>"` (one
    indexable `TEXT` column), via `FromStr` and
    `TryFrom<String>`/`Into<String>`.
  - `ParseEntityRefError` — `Malformed` / `UnknownType` / `BadId`; no
    panics on any input.
  - `EdgeKind` — the closed v1 edge-kind registry (`same_identity`,
    `works_at`, `member_of`, `employed_by`, `subject_of`) with
    `is_symmetric`, `is_temporal`, `inverse`, `sensitivity`, and
    `permits(from, to)` endpoint-type validation.
  - `Sensitivity` — `Medium` / `High` (`subject_of` is the sole `High`
    kind, per §10's `case ↔ person` governance).
  - `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`,
    `#![warn(clippy::pedantic)]`; dependency-light (`serde`, `uuid`,
    `thiserror`). Unit tests plus a doctest cover URN round-tripping,
    every `EntityType` token, the `course`/`courseinstance` shared-
    service mapping, malformed/unknown-type/bad-UUID rejection, the
    serde wire form, and the registry's endpoint-permission rules.
