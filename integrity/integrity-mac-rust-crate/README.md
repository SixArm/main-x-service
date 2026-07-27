# integrity-mac

Keyed integrity MACs (HMAC-SHA256, FIPS 198-1) with production-grade key
handling, shared by every Main X Index service that carries a
tamper-evidence tier.

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
