# Worker matcher — Living Specification

> **Status:** Living document. Canonical SDD specification for the `worker-matcher` Rust crate — single source of truth; consolidates what would otherwise live in `spec.md` + `plan.md` + `tasks.md`. Delivered tasks archived in [`AGENTS/delivered-tasks.md`](../AGENTS/delivered-tasks.md) + [`AGENTS/delivered-tasks-2.md`](../AGENTS/delivered-tasks-2.md); research-spike outcomes in [`AGENTS/roadmap-research.md`](../AGENTS/roadmap-research.md).
>
> **Version:** 0.3.0 · **Maintainer:** Joel Parker Henderson — `joel@joelparkerhenderson.com` · **Crate:** `worker-matcher` (Cargo) · **Edition:** Rust 2024 · **Licence:** MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause · **Repository:** https://github.com/sixarm/worker-matcher-rust-crate
>
> See also: [index.md](../index.md), [AGENTS.md](../AGENTS.md), [README.md](../README.md), [CHANGELOG.md](../CHANGELOG.md).

---

## Table of contents

1. [Purpose and Vision](01-purpose-and-vision.md)
2. [Scope](02-scope.md)
3. [Stakeholders and Users](03-stakeholders-and-users.md)
4. [Glossary](04-glossary.md)
5. [Research Basis](05-research-basis.md)
6. [Functional Requirements](06-functional-requirements.md)
7. [Non-Functional Requirements](07-non-functional-requirements.md)
8. [Domain Model](08-domain-model.md)
9. [Architecture](09-architecture.md)
10. [Component Specifications](10-component-specifications.md)
11. [Public API Specification](11-public-api-specification.md)
12. [Algorithm Specifications](12-algorithm-specifications.md)
13. [Configuration Specification](13-configuration-specification.md)
14. [Normalization Specification](14-normalization-specification.md)
15. [Error Model](15-error-model.md)
16. [Serialization Contract](16-serialization-contract.md)
17. [Quality Attributes](17-quality-attributes.md)
18. [Testing Strategy](18-testing-strategy.md)
19. [Build, Tooling, and Release](19-build-tooling-and-release.md)
20. [Security, Privacy, and Compliance](20-security-privacy-and-compliance.md)
21. [Roadmap and Future Work](21-roadmap-and-future-work.md)
22. [Open Questions and Risks](22-open-questions-and-risks.md)
23. [Tasks and Acceptance Criteria](23-tasks-and-acceptance-criteria.md)
24. [Change Control](24-change-control.md)
25. [References](25-references.md)
