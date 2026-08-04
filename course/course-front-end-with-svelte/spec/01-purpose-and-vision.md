## 1. Purpose and Vision

### 1.1 Purpose

Provide an operator-facing web UI for the Course Service that exercises the full duplicate-handling workflow: search, create with real-time duplicate detection, score-based match check, manual merge, and per-record audit review.

### 1.2 Vision

A web interface that:

- Surfaces every score-bearing decision (match quality, per-component breakdown) so operators can audit a merge before committing.
- Mirrors the service's REST surface 1:1 — no hidden business logic on the client.
- Stays terse and direct: a single primary action per page, no modals stacked on modals.
- Scales to the four sibling entities (Worker / Place / Course / Event) by copy-adapt of this scaffold.

### 1.3 Non-goals

- **Not** a public-facing portal — assumes authenticated operator users. BFF magic-link sign-in (`/signin`, `/verify`) landed 2026-06-18; no route currently redirects an unauthenticated visitor away (§13 T-24).
- **Not** a substitute for direct API access — power users use Swagger UI / curl.
- **Not** a FHIR client — the Course Service exposes no FHIR surface (no FHIR resource fits Course cleanly; service spec §2.2).

