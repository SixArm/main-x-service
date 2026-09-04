# entity-ref

The one shared **contract** for cross-service entity linking in the Main X
Index family — `agents/share/cross-service-linking.md` §3 (the `EntityRef`
value type) and §9 (the v1 edge-kind registry). Rollout **step 1**: land
the contracts, no behaviour yet.

A record in another service is named by an opaque URN string
`"<entity_type>:<uuid>"`, e.g. `person:0c4f1e2a-…`. This crate owns the
parsing/validation and the static metadata; it is pure, panic-free, and
dependency-light (serde + uuid + thiserror). The rollout note below once
framed this as copyable per project until a second non-aggregator
consumer justified a shared dependency — that threshold has since been
crossed several times over; see "Who actually consumes this" below.

```rust
use entity_ref::{EdgeKind, EntityRef, EntityType, Sensitivity};

// Parse / display / (de)serialise as the single URN string.
let r: EntityRef = "person:0c4f1e2a-0000-4000-8000-000000000000".parse()?;
assert_eq!(r.entity_type, EntityType::Person);
assert_eq!(r.service(), "person-service");           // entity_type → owning service
assert_eq!(r.to_string(), "person:0c4f1e2a-0000-4000-8000-000000000000");

// The closed v1 edge-kind registry validates endpoint types.
assert!(EdgeKind::EmployedBy.permits(EntityType::Worker, EntityType::Organization));
assert_eq!(EdgeKind::EmployedBy.inverse(), Some("employs"));
assert_eq!(EdgeKind::SubjectOf.sensitivity(), Sensitivity::High); // case → person (§10)
```

## Types

| Item | Purpose |
|---|---|
| `EntityType` | The globally-unique entity discriminator (`person`, `worker`, …, `courseinstance`, `care_pathway`); `as_str`, `from_token`, and the `service()` map (course + courseinstance → `course-service`). |
| `EntityRef` | `{entity_type, id: Uuid}`; `FromStr`/`Display`/serde as the `"type:uuid"` URN (one indexable `TEXT` column). |
| `EdgeKind` | The closed v1 registry (`same_identity`, `works_at`, `member_of`, `employed_by`, `subject_of`) with `is_symmetric` / `is_temporal` / `inverse` / `sensitivity` / `permits(from, to)`. |
| `Sensitivity` | `Medium` (affiliation / identity) vs `High` (`case → person`, §10). |

## Who actually consumes this

As of 2026-08-04, eight crates depend on this one as a real Cargo `path`
dependency (`entity-ref = { path = "../../link/entity-ref-rust-crate" }`)
— not a copy-per-project, despite the framing above:

- **`link-graph-service-with-loco`** — the aggregator (the read model);
  by far the heaviest user, importing across `auth.rs`, `consumer.rs`,
  `controllers/graph.rs`, `events.rs`, `graph.rs`, `models/edges.rs`,
  `models/entity_presence.rs`, `probe.rs`, `reconcile.rs`, and
  `suggest/{job,mod}.rs`.
- **`person-service-with-loco`**, **`worker-service-with-loco`**,
  **`case-service-with-loco`** — the three entity services that
  originate edges (`entity_links` write-side + `linked`/`unlinked`
  events), as the design doc anticipated.
- **`contact-relationship-management-service-with-rust`**,
  **`content-management-system-service-with-rust`**,
  **`patient-flow-service-with-rust`**,
  **`workforce-planning-management-service-with-rust`** — the four
  consumer apps, which were *not* anticipated by the original rollout
  note. Each uses `EntityRef`/`EntityType` in its own `src/validation.rs`
  and `src/clients.rs` to validate and dereference cross-service refs
  (e.g. `person_ref`, `worker_ref`) rather than to originate edges.

Cross-service links are still **never** a matcher signal — see
`cross-service-linking.md` §7.

## Next tasks (recommended 2026-09-04)

> This crate has no `spec/`, so per the family's task-queue convention
> ([`AGENTS.md`](../../AGENTS.md) "Where planning lives") this section is
> the live work queue in place of a `spec/13-tasks.md`. Follow the
> family's three-part discipline (this README/CHANGELOG + code + tests)
> for any behavioural change.

- [ ] **ER-1 (S) "Who actually consumes this" is stale — a ninth consumer
      exists and isn't listed.** *(verified:
      `grep -rl "entity-ref" --include=Cargo.toml .` from the repo root,
      excluding this crate's own manifest and its `fuzz/`, finds NINE
      consumers today: the eight this README's "Who actually consumes
      this" section names (2026-08-04) plus
      `care-pathway/care-pathway-service-with-loco`, which is not
      mentioned anywhere in this file.
      `care-pathway-service-with-loco/Cargo.toml:49` confirms
      `entity-ref = { path = "../../link/entity-ref-rust-crate" }`, and
      `agents/share/cross-service-linking.md` §11 records why: care-pathway
      landed the `continues_as` write-side 2026-08-24 — the **fifth**
      edge-originating service, not just a fourth consumer app)*. Update
      the "Who actually consumes this" list and its "eight crates"
      count to nine, adding care-pathway alongside person/worker/case as
      an edge-originating service (it uses `EntityRef`/`EdgeKind` in its
      `entity_links` write-side, not just to validate/dereference refs
      like the four consumer apps).
      **Acceptance:** the count and the list match a fresh
      `grep -rl "entity-ref" --include=Cargo.toml .` run.

- [ ] **ER-2 (S) Cut a release for the accumulated `[Unreleased]`
      changes, and fix the stale MSRV entry inside it.**
      *(verified: `CHANGELOG.md`'s `[Unreleased]` section lists the
      cargo-fuzz harness, the Criterion benches, AND a "declared MSRV
      (Rust 1.95)" entry — but `Cargo.toml` today reads
      `rust-version = "1.96"`, the current N-2 policy value, not 1.95;
      the unreleased changelog text was written for an earlier N-3/1.95
      bump and was never updated when the crate moved to 1.96 alongside
      the rest of the family, per `ci/msrv.txt`. `Cargo.toml`'s
      `version` field still reads `0.2.0`, the same version the
      `## [0.2.0] - 2026-08-05` heading below describes — i.e. three
      real changes have landed with no version bump since)*. Bump to
      `0.3.0` (fuzzing + benches are new capability, not a patch), move
      the `[Unreleased]` entries under a dated heading, and correct the
      MSRV entry to say 1.96 (N-2) rather than 1.95 (N-3) — or add a
      second dated bullet recording the later 1.95→1.96 move, whichever
      reads more honestly against `git log` for this crate's
      `Cargo.toml`. Per the family's "cargo publish authorized"
      convention for an already-published, verified-green crate,
      publish the release.
      **Acceptance:** `cargo test`, `cargo clippy --all-targets -- -D
      warnings`, `cargo fmt --check` green; `CHANGELOG.md` has no stale
      `[Unreleased]` content and no MSRV/version mismatch against
      `Cargo.toml`; `cargo publish --dry-run` succeeds.

- [ ] **ER-3 (S) The "## Types" table omits `continues_as` (and doesn't
      name the two entity types it needs).** *(verified: this README's
      "## Types" table lists the `EdgeKind` registry as "(`same_identity`,
      `works_at`, `member_of`, `employed_by`, `subject_of`)" — five
      kinds — but `src/lib.rs`'s `EdgeKind` enum and its `ALL` const
      carry a sixth, `ContinuesAs` (landed 2026-08-24, tested by
      `continues_as_names_a_journey_and_only_a_journey`); the
      `EntityType` row's `"… courseinstance, care_pathway"` elision
      likewise predates and doesn't name the two entity types
      `ContinuesAs` needs, `CarePathwayInstance` and `PatientFlowStay`,
      both present in `EntityType::ALL`)*. Add `continues_as` to the
      `EdgeKind` row and name `care_pathway_instance` /
      `patient_flow_stay` explicitly in the `EntityType` row (or drop
      the lossy `…` elision entirely and list all twelve), so the
      README's public-surface table matches `src/lib.rs`'s `ALL`
      constants without a reader having to cross-check the source.
      **Acceptance:** every `EdgeKind::ALL` / `EntityType::ALL` member
      is named somewhere in the "## Types" table (a doctest or a simple
      script assertion is a reasonable way to pin this so it can't
      silently drift again).

- [ ] **ER-4 (S) Add `readme = "README.md"` to `Cargo.toml`.**
      *(verified: `Cargo.toml` declares `description`/`license`/
      `repository`/`keywords`/`categories` but no `readme` field, so
      the published crates.io page for `entity-ref` 0.2.0 shows only
      the one-line `description`, not this file's worked example and
      "Who actually consumes this" narrative)*. A one-line manifest
      addition, worth landing alongside ER-2's release so the next
      published version renders the README on crates.io.
      **Acceptance:** `cargo package --list` includes `README.md`;
      the next `cargo publish --dry-run` (ER-2) shows the readme wired.
