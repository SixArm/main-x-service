# integrity-mac

Keyed integrity MACs (HMAC-SHA256, FIPS 198-1) with production-grade key
handling, shared by every Main X Index service that carries a
tamper-evidence tier.

> This crate has no `spec/`, `AGENTS.md`, `CLAUDE.md`, or `index.md` — a
> deliberate, documented waiver, not an oversight. See the
> [`CHANGELOG.md`](./CHANGELOG.md) header for why: the design rationale
> below and in `src/lib.rs`'s rustdoc is already the complete
> documentation surface.

## Why it exists

The services store three integrity values per row: SHA-256, SHA-3, and a
MAC. The two digests are **unkeyed**, and their pre-image formats are
published in each service's `spec/12-compliance.md` §12.4z — so an
adversary who can write SQL edits a row and recomputes them. The MAC is
the only one they cannot forge, because its key lives in the service's
environment and is never written to the database.

That makes this the code whose correctness the whole control rests on,
and the code that should not have one divergent copy per service. It was
copied four times before this crate existed. The trigger for extracting
it: a latent defect in the sibling `soup.rs` — a test matching the
substring `timestamp` rather than the JSON field — survived in three
copies and surfaced only when a fourth crate happened to use the word in
prose. A key-handling defect that survived that way would not announce
itself at all. It would make MACs forgeable while every test stayed
green.

## What it provides

- **HKDF-SHA256 domain separation.** The configured value is a *root*
  key that never MACs anything; each purpose derives its own subkey under
  `mxi/<service>/<domain>/d1`. A tag cannot transfer between purposes, or
  between two services sharing one cluster-wide key, even if their
  pre-images are byte-identical.
- **Key sourcing.** A mounted file takes precedence over an inline
  environment variable, and an unreadable file disables MACs rather than
  falling back — a deployment that mounted a secret and mistyped the path
  should see MACs stop, not continue under a key it believed it replaced.
- **Root-key zeroization** once the subkeys are derived, so the key that
  compromises every domain does not survive in core dumps or swap.
- **Placeholder refusal.** A length floor alone accepts 32 zero bytes and
  `0101…`; keys with fewer than 8 distinct bytes are rejected.
- **Key generation** from the OS CSPRNG, and owner-only key files created
  at mode 0600 that refuse to clobber.
- **A verdict vocabulary that distinguishes "I cannot check this" from
  "this is wrong"** — an unknown key or scheme is never reported as
  tampering.

## What it does not defend against

An adversary holding **both** the database and the service environment
has the key and can forge freely. Nothing stored beside the data could
prevent that. This is defence against database-only compromise — a
stolen backup, a replica, SQL injection, a DBA without application-server
access — which is the common case and worth having.

## Usage

Each service defines its own `Domain` enum (the sets differ) and holds
one `KeySet` for the process. See `src/lib.rs` for the full example.

## Compatibility

The derivation is pinned by golden vectors cross-checked against an
independent HKDF-SHA256 implementation. Changing the info string, the
scheme tag, or the output length is a **migration**, not a refactor: the
`d1` scheme tag in every stored MAC is what makes a deliberate change
survivable, and a stored value naming an unknown scheme is reported
unverifiable rather than invalid.

## Next tasks (recommended 2026-09-04)

> This crate has no `spec/`, so per the family's task-queue convention
> ([`AGENTS.md`](../../AGENTS.md) "Where planning lives") this section is
> the live work queue in place of a `spec/13-tasks.md`. Follow the
> family's three-part discipline (this README/CHANGELOG + code + tests)
> for any behavioural change.

- [ ] **IM-1 (M) Cut the first crates.io release.**
      *(verified: `Cargo.toml` already carries every field
      `link/entity-ref-rust-crate`'s **published** (`entity-ref` 0.2.0,
      crates.io) manifest carries — `description`, `license`,
      `repository`, `keywords`, `categories` — so this crate clears the
      same publish bar entity-ref already cleared; `agents/share/
      overview.md` and the root `AGENTS.md` library-crates table both
      still say "Not published to crates.io — … no release has been
      cut", and `grep -rl "integrity-mac" --include=Cargo.toml .`
      (excluding this crate's own manifest and its `fuzz/`) finds 12
      real in-tree consumers, all on a `path` dependency)*. Publish
      `integrity-mac` 0.2.0 to crates.io (mirroring entity-ref's
      publish), per the family's "cargo publish authorized" convention
      for an already-verified-green, already-published-shape crate.
      Update `agents/share/overview.md` and the root `AGENTS.md` Library
      Crates table to drop the "not yet published" framing (leaving the
      accurate note, matching entity-ref's own history, that every
      in-tree consumer still takes the `path` dependency rather than the
      crates.io release — publishing changes what's *possible*, not
      what the family actually does today).
      **Acceptance:** `cargo publish --dry-run` succeeds; the crate is
      live on crates.io at 0.2.0; the two family docs above no longer
      claim it is unpublished.

- [ ] **IM-2 (L) Evaluate extracting the duplicated audit hash-chain +
      checkpoint logic, the same way this crate's own MAC logic was
      extracted.** *(verified: `person/person-service-with-loco/src/
      compliance/audit_chain.rs` and `worker/worker-service-with-loco/
      src/compliance/audit_chain.rs` are 656 lines each, and `diff`
      between them shows only the entity name and test-vector hash
      differ — the same near-byte-identical-copy shape this crate's own
      README (`## Why it exists`) describes as the trigger for
      extracting the MAC code: "It was copied four times before this
      crate existed … A key-handling defect that survived that way
      would not announce itself at all." `case-service-with-loco` and
      `care-pathway-service-with-loco` carry their own
      `src/compliance/audit_chain.rs` + `checkpoint.rs` too — four
      copies total, matching this crate's own origin story exactly.
      `agents/share/runbooks/integrity-activation.md` additionally
      documents checkpoint **storage** as "undefined" family-wide (line
      32: "checkpoint storage | undefined | wholesale deletion of the
      trail is undetectable"), a second axis of duplicated-and-unsolved
      logic across the same four crates)*. Given the size, land this as
      a decision-then-migration: (a) confirm the four `audit_chain.rs` +
      `checkpoint.rs` pairs really are drift-free copies (a full diff,
      not just person/worker), (b) if so, design the shared API this
      crate would need (it currently ships tags/verdicts, not a chain
      abstraction) as a spec addition to this README, (c) migrate one
      crate first (person, as this family's usual reference) with the
      others following as separate follow-up tasks, each its own
      three-part change (README/CHANGELOG here + code + tests) plus a
      `CHANGELOG.md` entry in every migrated service crate.
      **Acceptance:** either the shared abstraction lands (starting with
      one migrated service, `cargo test --lib` green there and in this
      crate) or the decision to keep the copies is recorded here with
      the reasoning (e.g. "the four crates' compliance requirements
      have already diverged enough that a shared abstraction would
      re-introduce the multi-copy risk in a different shape") — either
      way, not left as an unexamined duplication.

- [ ] **IM-3 (S) Add `readme = "README.md"` to `Cargo.toml`.**
      *(verified: neither this crate's `Cargo.toml` nor the published
      `entity-ref` sibling's declares a `readme` field, so — same gap,
      inherited rather than unique — the crates.io page for either
      would show only the one-line `description`, not this file's much
      more informative "Why it exists" / "What it does not defend
      against" sections)*. A one-line manifest addition; do it here
      ahead of the IM-1 publish so the first release already renders
      the README on crates.io, and note the same fix for `entity-ref`
      as a follow-up in that crate's own docs (out of scope here).
      **Acceptance:** `cargo package --list` includes `README.md`;
      `cargo publish --dry-run` (from IM-1) shows the readme wired.
