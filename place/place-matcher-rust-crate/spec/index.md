# Place matcher — specification

**Crate:** `place-matcher` &nbsp;·&nbsp; **Version targeted:** `0.7.0` &nbsp;·&nbsp; **Status:** authoritative

This document is the living, single source of truth (SSOT) for the `place-matcher` Rust crate. Every other document in the repository (`README.md`, `index.md`, `AGENTS.md`, `agents/*.md`, `CHANGELOG.md`) summarises or quotes this file — none contradicts or extends it. When prose elsewhere disagrees with this file, this file wins; when this file disagrees with the code, see §9.

The keywords **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are to be interpreted in the sense of RFC 2119 / RFC 8174.

---

## Table of contents

1. [Scope](01-scope.md)
2. [Terminology](02-terminology.md)
3. [Data model](03-data-model.md)
4. [Normalisation](04-normalisation.md)
5. [Matching pipeline](05-matching-pipeline.md)
6. [Per-field scoring algorithms](06-per-field-scoring-algorithms.md)
7. [Configuration](07-configuration.md)
8. [Determinism and safety](08-determinism-and-safety.md)
9. [Public API contract (SemVer)](09-public-api-contract-semver.md)
10. [Open questions](10-open-questions.md)
11. [Worked examples](11-worked-examples.md)
12. [Glossary cross-reference](12-glossary-cross-reference.md)
13. [References](13-references.md)
