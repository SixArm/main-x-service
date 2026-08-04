## 1. Purpose and Vision

### 1.1 Purpose

Provide an operator-facing web UI for the Place Service that exercises the full duplicate-handling workflow: search, create with real-time duplicate detection, score-based match check, manual merge, and per-record audit review.

### 1.2 Vision

A web interface that:

- Surfaces every score-bearing decision (match quality, per-component breakdown) so operators can audit a merge before committing.
- Mirrors the service's REST surface 1:1 — no hidden business logic on the client.
- Stays terse and direct: a single primary action per page, no modals stacked on modals.
- Scales to the four sibling entities (Worker / Place / Thing / Event) by copy-adapt of this scaffold.

### 1.3 Non-goals

- **Not** a public-facing portal — assumes authenticated operator users. Auth landed as T-22 (BFF + httpOnly session cookie + server-side PASETO exchange; see `AGENTS.md` "Authentication — the BFF pattern"); enforcement is still governed by the service's own `PLACE_REQUIRE_AUTH` gate (default-off — see [`../../../agents/share/security.md`](../../../agents/share/security.md) §4).
- **Not** a substitute for direct API access — power users use Swagger UI / curl.
- **Not** a FHIR client — FHIR routes are out of scope (the service exposes them; this UI doesn't render them).

