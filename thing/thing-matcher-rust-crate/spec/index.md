# Thing matcher — specification

**Crate:** `thing-matcher` &nbsp;·&nbsp; **Version targeted:** `0.6.1` &nbsp;·&nbsp; **Status:** authoritative

This document is the living, single source of truth (SSOT) for the `thing-matcher` Rust crate. Every other document in the repository (`README.md`, `index.md`, `AGENTS.md`, `AGENTS/*.md`, `CHANGELOG.md`) summarises or quotes this file — none contradicts or extends it. When prose elsewhere disagrees with this file, this file wins; when this file disagrees with the code, see §9.

The keywords **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are to be interpreted in the sense of RFC 2119 / RFC 8174.

---

## Table of contents

1. [Scope](01-scope.md)
2. [Terminology](02-terminology.md)
3. [Data model](03-data-model.md)
4. [Normalisation](04-normalisation.md)
5. [Matching engine](05-matching-engine.md)
6. [Per-field scoring algorithms](06-per-field-scoring-algorithms.md)
7. [Quality attributes and tuning](07-quality-attributes-and-tuning.md)
8. [Public API surface](08-public-api-surface.md)
9. [Specification / code drift discipline](09-specification-code-drift-discipline.md)
10. [Open questions](10-open-questions.md)
11. [Worked examples](11-worked-examples.md)
12. [Glossary cross-reference](12-glossary-cross-reference.md)
13. [References](13-references.md)
