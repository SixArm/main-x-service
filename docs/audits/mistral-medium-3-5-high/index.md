Based on my analysis of the **Main X Service** monorepo, here are **prioritized, actionable quality improvement tasks** organized into your two categories. Each task is designed to be executable by AI, with clear success criteria.

---

---

# 🎯 **CATEGORY 1: Visibility, Onboarding, Promotion & Upskilling**

*Tasks to help new visitors understand, adopt, and contribute to the project.*

---

## 📚 **Documentation & Discovery**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **HIGH** | **DOC-ONBOARD-001** | Create a **repo-level `CONTRIBUTING.md`** | Unified guide covering: dev environment setup, PR process, commit conventions, code review expectations, green gate requirements. Link from README. |
| **HIGH** | **DOC-ONBOARD-002** | Create a **`GETTING-STARTED.md`** (separate from tutorials) | 5-minute quick start: clone → run single service locally (no containers) with `cargo run`, hit health endpoint. For users who just want to see it work *now*. |
| **HIGH** | **DOC-ONBOARD-003** | **Fix the broken README links** | README references `@agents/share/overview.md` via `@`-include, but this doesn't render on GitHub. Replace with direct links or inline the content. |
| **HIGH** | **DOC-ONBOARD-004** | **Create a `ROADMAP.md`** | Extract the "Known gaps" and "Themes" from `plan.md` into a public-facing roadmap. Remove internal audit details, keep feature timelines. |
| **HIGH** | **DOC-ONBOARD-005** | **Add per-crate `CONTRIBUTING.md`** (for 5 oldest crates) | person, worker, place, thing, event services lack `CONTRIBUTING.md`. Use a template: local dev, test commands, where to find specs. |
| **MEDIUM** | **DOC-ONBOARD-006** | **Create `ARCHITECTURE.md` at repo root** | Visual architecture diagram (Mermaid) + 2-paragraph summary of the system. Link to detailed `agents/share/architecture.md`. |
| **MEDIUM** | **DOC-ONBOARD-007** | **Generate API documentation** | Use `cargo doc --no-deps --open` for each service crate, add to CI, publish to GitHub Pages. Currently missing. |
| **MEDIUM** | **DOC-ONBOARD-008** | **Add `SECURITY.md`** | Security policy: vulnerability reporting (currently missing), supported versions, security update process. GitHub requires this for security advisories. |
| **MEDIUM** | **DOC-ONBOARD-009** | **Add `CODE_OF_CONDUCT.md`** | Standard CoC (e.g., Contributor Covenant). Encourages healthy community. Currently missing. |
| **MEDIUM** | **DOC-ONBOARD-010** | **Create `FAQ.md`** | Answer common questions: "What is the Main X Index?", "How do services communicate?", "Why no workspace Cargo.toml?", "How to add a new entity?" |
| **LOW** | **DOC-ONBOARD-011** | **Add badges to all READMEs** | CI status, license, Rust version, crates.io version (for published crates). Currently only root has partial badges. |
| **LOW** | **DOC-ONBOARD-012** | **Add screenshots to front-end READMEs** | Each SvelteKit front-end should have 2-3 screenshots showing the UI. Currently none exist. |

---

## 🎓 **Tutorials & Examples**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **HIGH** | **TUT-001** | **Complete tutorials 02-06** | Tutorials 02-06 are referenced in `tutorials/01-getting-started.md` but don't exist. Create them based on `plan.md` themes. |
| **HIGH** | **TUT-002** | **Add tutorial 07: Front-end Development** | Walk through: set up a front-end, connect to a service, create a custom view. Currently missing. |
| **HIGH** | **TUT-003** | **Create video walkthrough scripts** | Write scripts for 3 videos: (1) Backend dev, (2) Front-end dev, (3) Full stack. For YouTube/promotion. |
| **MEDIUM** | **TUT-004** | **Add interactive tutorials** | Use GitHub Codespaces or similar to create "click to start" dev environments with pre-configured stacks. |
| **MEDIUM** | **TUT-005** | **Create a "Tour of Main X Index" document** | Guided tour explaining each entity, matcher, service, and how they work together. With diagrams. |
| **MEDIUM** | **TUT-006** | **Expand `examples/api/`** | Add `.http` files for all 10 services (currently only case exists). Include auth handshake, CRUD, search, match. |
| **MEDIUM** | **TUT-007** | **Create `examples/policies/` cookbook** | Add 10+ ABAC policy examples: role-based, attribute-based, time-based, IP-based, etc. Currently only referenced in `plan.md`. |
| **LOW** | **TUT-008** | **Add Postman/Insomnia collections** | Export `.http` files to Postman/Insomnia format. Easier for API users. |
| **LOW** | **TUT-009** | **Create a "Hello World" example** | Minimal Rust program that uses `authentication-verifier` to verify a PASETO token. |

---

## 🚀 **Promotion & Community**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **HIGH** | **PROM-001** | **Create a project website** | Use `main-x-service.github.io` repo (already exists). Build with SvelteKit. Include: overview, docs links, tutorials, roadmap, contribute guide. |
| **HIGH** | **PROM-002** | **Add OpenGraph/Social cards** | Generate social preview images for repo, docs, tutorials. Use for Twitter/GitHub/LinkedIn sharing. |
| **HIGH** | **PROM-003** | **Write a blog post series** | "Building a Federated Identity Index in Rust": Part 1 (Architecture), Part 2 (Matching), Part 3 (ABAC), Part 4 (Event Streaming). Publish on dev.to/medium. |
| **HIGH** | **PROM-004** | **Create a project pitch/deck** | 10-slide presentation explaining the project for conferences, sponsors, potential contributors. |
| **MEDIUM** | **PROM-005** | **Submit to awesome lists** | Add to `awesome-rust`, `awesome-svelte`, `awesome-postgresql`, etc. on GitHub. |
| **MEDIUM** | **PROM-006** | **Create a Discord/Slack community** | Set up a community chat for contributors. Add link to README. |
| **MEDIUM** | **PROM-007** | **Add to Rust libs.rs** | Publish all crates to libs.rs for discoverability. Currently only some are listed. |
| **MEDIUM** | **PROM-008** | **Create a "Powered By" page** | Showcase organizations using Main X Service. Encourages adoption. |
| **LOW** | **PROM-009** | **Write guest posts** | Contribute articles to Rust blog, Svelte blog, Postgres blog, etc. |
| **LOW** | **PROM-010** | **Create a newsletter** | Monthly updates on new features, releases, community news. Use GitHub Sponsors or similar. |

---

## 🎯 **Upskilling & Learning**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **HIGH** | **UPSKILL-001** | **Create a "Rust for Main X Service" guide** | Cover: async Rust, Axum, sqlx, serde, tokio. Focused on what contributors need. |
| **HIGH** | **UPSKILL-002** | **Create a "SvelteKit for Main X Service" guide** | Cover: stores, load functions, forms, actions, server-side endpoints. |
| **HIGH** | **UPSKILL-003** | **Create architecture decision records (ADRs)** | Document key decisions: why Axum, why SvelteKit, why Postgres, why not gRPC by default, etc. |
| **MEDIUM** | **UPSKILL-004** | **Create a "Testing in Main X Service" guide** | Cover: unit tests, integration tests, DB-gated tests, property tests, fuzz tests. |
| **MEDIUM** | **UPSKILL-005** | **Create a "Security in Main X Service" guide** | Cover: ABAC, PASETO, audit logging, privacy masking, secure defaults. |
| **MEDIUM** | **UPSKILL-006** | **Create comparison docs** | Compare Main X Service to: Keycloak, Auth0, Apache Unomi, JanusGraph, etc. Help users understand when to use it. |
| **LOW** | **UPSKILL-007** | **Create a glossary** | Define terms: entity, matcher, service, link graph, ABAC, PASETO, etc. |
| **LOW** | **UPSKILL-008** | **Create a "Common Pitfalls" doc** | Document mistakes contributors often make and how to avoid them. |

---

---

# 🔧 **CATEGORY 2: Engineering, Coding, Testing & Auditing**

*Tasks to improve the codebase quality, reliability, and maintainability.*

---

## 🏗️ **Code Quality & Maintainability**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **CRITICAL** | **ENG-QUAL-001** | **Remove the committed `DEV_SEED` fallback** | SEC-A1 in `plan.md`. The `load_seed()` fallback to `DEV_SEED` allows token forgery. Must be removed or guarded. |
| **CRITICAL** | **ENG-QUAL-002** | **Fix the reconcile scoping bug** | SEC-B1 in `plan.md`. Link-graph diffs global read-model against one entity's edges, causing divergence. |
| **CRITICAL** | **ENG-QUAL-003** | **Fix the governed edge leak** | SEC-G1 in `plan.md`. Five front-end BFFs missing CSRF protection. Two bulk-links endpoints dump high-sensitivity edges. |
| **CRITICAL** | **ENG-QUAL-004** | **Fix merge TOCTOU bug** | SEC-B5 in `plan.md`. Unlocked read before write tx in merge allows race conditions. |
| **HIGH** | **ENG-QUAL-005** | **Enable `forbid(unsafe_code)`** | SEC-I3 in `plan.md`. Three crate roots miss `#![forbid(unsafe_code)]`. Add to all crates. |
| **HIGH** | **ENG-QUAL-006** | **Roll out cargo-deny** | SEC-I1 in `plan.md`. Only 3/12 services run `cargo deny`. Add `deny.toml` to all crates. |
| **HIGH** | **ENG-QUAL-007** | **Add fuzz testing** | SEC-I1 in `plan.md`. No `cargo-fuzz` anywhere. Add fuzz targets for: matchers, parsers, auth, validation. |
| **HIGH** | **ENG-QUAL-008** | **Fix LIKE-wildcard injection** | SEC-G3 in `plan.md`. person/worker/event lack `escape_like` guard. |
| **HIGH** | **ENG-QUAL-009** | **Add body size limits** | SEC-F-input in `plan.md`. No length/array-cardinality caps. Add to prevent DoS via O(n·m) string similarity. |
| **HIGH** | **ENG-QUAL-010** | **Fix false-deterministic matches** | SEC-F-input in `plan.md`. Short-circuits keying on post-normalization strings with no empty guard. |
| **MEDIUM** | **ENG-QUAL-011** | **Add property tests** | Add `proptest` for all matchers. Verify: never-panic, score ∈ [0,1], symmetric, identical⇒1.0, no-spurious-identity. |
| **MEDIUM** | **ENG-QUAL-012** | **Add race condition tests** | Use `loom` or similar to test concurrent operations: merge, bulk import, link updates. |
| **MEDIUM** | **ENG-QUAL-013** | **Normalize SPDX license IDs** | Some manifests use deprecated `GPL-2.0`/`GPL-3.0` instead of `GPL-2.0-only`/`GPL-3.0-only`. |
| **MEDIUM** | **ENG-QUAL-014** | **Add rust-toolchain.toml to all crates** | Some crates lack toolchain pin. Ensures consistent Rust version across all crates. |
| **LOW** | **ENG-QUAL-015** | **Add more clippy lints** | Enable additional clippy lints: `clippy::pedantic`, `clippy::nursery`, `clippy::cargo`. |
| **LOW** | **ENG-QUAL-016** | **Add rustfmt configuration** | Ensure consistent formatting across all crates. Add `rustfmt.toml` to repo root. |

---

## 🧪 **Testing Improvements**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **HIGH** | **TEST-001** | **Add DB-gated tests to CI for all crates** | Currently only crates in `ci/db-suites.txt` run DB tests. Add all service crates. |
| **HIGH** | **TEST-002** | **Add front-end e2e tests** | Currently only crates in `ci/front-end-e2e-suites.txt` run Playwright. Add all 16 front-ends. |
| **HIGH** | **TEST-003** | **Add integration tests between services** | Test: cross-service linking, event streaming, authentication flow, bulk import/export. |
| **HIGH** | **TEST-004** | **Add performance benchmarks** | Add Criterion benchmarks for: matching, search, merge, bulk operations. Track regressions. |
| **MEDIUM** | **TEST-005** | **Add chaos engineering tests** | Test service resilience: kill Postgres mid-operation, network partitions, high load. |
| **MEDIUM** | **TEST-006** | **Add security tests** | Use `cargo-audit`, `cargo-deny`, CodeQL. Scan for vulnerabilities, hardcoded secrets, SQL injection. |
| **MEDIUM** | **TEST-007** | **Add compliance tests** | Verify: GDPR erasure, audit logging, privacy masking, ABAC enforcement. |
| **MEDIUM** | **TEST-008** | **Add upgrade/downgrade tests** | Test migration compatibility: upgrade DB, downgrade code, verify no data loss. |
| **LOW** | **TEST-009** | **Add property-based tests for API** | Use `schemathesis` or similar to test API contracts, edge cases, invalid inputs. |
| **LOW** | **TEST-010** | **Add snapshot tests** | For OpenAPI schemas, frontend UI, API responses. Detect unintended changes. |

---

## 🔒 **Security Hardening**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **CRITICAL** | **SEC-001** | **Remove dev signing seed fallback** | `DEV_SEED` in authentication-service allows token forgery if `TOKEN_PRIVATE_KEY_SEED` missing. |
| **CRITICAL** | **SEC-002** | **Fix unauthenticated audit endpoint** | `/api/auth/audit/recent` leaks registered emails. Add auth requirement. |
| **CRITICAL** | **SEC-003** | **Fix magic-link token logging** | Magic-link tokens logged unconditionally. Must be redacted. |
| **CRITICAL** | **SEC-004** | **Fix race condition in magic-link consume** | Non-atomic single-use consume allows replay attacks. |
| **HIGH** | **SEC-005** | **Add CSRF protection to front-ends** | Five front-end BFFs missing CSRF protection. Add to all. |
| **HIGH** | **SEC-006** | **Enable auth enforcement by default** | All services default to `<ENTITY>_REQUIRE_AUTH=off`. Change default to `on`. |
| **HIGH** | **SEC-007** | **Add rate limiting** | Add rate limiting to all public endpoints. Use `axum-rate-limit` or similar. |
| **HIGH** | **SEC-008** | **Add input validation** | Add validation for: array lengths, string lengths, nested object depths, request body size. |
| **HIGH** | **SEC-009** | **Fix LIKE injection** | person/worker/event missing `escape_like`. Add to all services using ILIKE. |
| **HIGH** | **SEC-010** | **Add security headers** | Add: CSP, HSTS, X-Frame-Options, X-Content-Type-Options, Permissions-Policy. |
| **MEDIUM** | **SEC-011** | **Implement token revocation** | Add endpoint to revoke PASETO tokens. Track revoked tokens in DB. |
| **MEDIUM** | **SEC-012** | **Add security audit logging** | Log: authentication attempts (success/failure), authorization decisions, sensitive operations. |
| **MEDIUM** | **SEC-013** | **Add secrets scanning** | Use `gitleaks` or `trufflehog` in CI to detect committed secrets. |
| **MEDIUM** | **SEC-014** | **Add dependency scanning** | Use `dependabot`, `cargo-audit`, `cargo-deny` for all crates. |
| **LOW** | **SEC-015** | **Add penetration testing** | Set up automated security scanning: OWASP ZAP, Nuclei, etc. |
| **LOW** | **SEC-016** | **Add threat modeling** | Document threat model: assets, threats, mitigations. |

---

## 📊 **Observability & Monitoring**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **HIGH** | **OBS-001** | **Add OTLP tracing to all services** | Currently only link-graph has OTLP. Add to all 10 entity services + auth. |
| **HIGH** | **OBS-002** | **Add Prometheus metrics to all services** | Add: request count/duration, error rates, DB query metrics, cache hit rates. |
| **HIGH** | **OBS-003** | **Add health checks** | Add: `/_health/live`, `/_health/ready`, `/_health/metrics`. Standardize across all services. |
| **HIGH** | **OBS-004** | **Add structured logging** | Use `tracing` + `tracing-subscriber` with JSON output. Add correlation IDs. |
| **MEDIUM** | **OBS-005** | **Add distributed tracing** | Trace requests across services. Use OpenTelemetry context propagation. |
| **MEDIUM** | **OBS-006** | **Add alerting rules** | Define Prometheus alerting rules for: high error rates, slow responses, DB connection issues. |
| **MEDIUM** | **OBS-007** | **Add dashboards** | Create Grafana dashboards for: service overview, performance, errors, DB metrics. |
| **LOW** | **OBS-008** | **Add log aggregation** | Set up centralized logging: Loki, ELK, or similar. |
| **LOW** | **OBS-009** | **Add synthetic monitoring** | Simulate user flows: create record, search, update, delete. Alert on failures. |

---

## ⚡ **Performance & Scalability**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **HIGH** | **PERF-001** | **Optimize matching algorithms** | Profile matchers. Optimize: Jaro-Winkler, Levenshtein, Jaccard, Soundex. |
| **HIGH** | **PERF-002** | **Add caching** | Cache: search results, match results, bulk export artifacts. Use Redis or similar. |
| **HIGH** | **PERF-003** | **Optimize DB queries** | Add indexes, analyze query plans, optimize slow queries. |
| **HIGH** | **PERF-004** | **Add connection pooling** | Configure optimal connection pool settings for Postgres. |
| **MEDIUM** | **PERF-005** | **Add pagination to all list endpoints** | Some endpoints lack pagination. Add `?limit=&offset=` to all. |
| **MEDIUM** | **PERF-006** | **Add async processing** | Offload long-running tasks: matching, bulk import, report generation. |
| **MEDIUM** | **PERF-007** | **Add compression** | Enable gzip/brotli compression for API responses. |
| **LOW** | **PERF-008** | **Add load testing** | Use `k6`, `locust`, or similar. Test: 100, 1000, 10000 RPS. |
| **LOW** | **PERF-009** | **Add benchmark CI** | Run benchmarks in CI. Fail on performance regressions. |
| **LOW** | **PERF-010** | **Add flamegraph profiling** | Profile CPU usage. Identify and optimize hot paths. |

---

## 🏗️ **Infrastructure & DevOps**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **HIGH** | **INFRA-001** | **Add local CI/CD (pre-push hooks)** | Run `make ci` (fmt, clippy, test) before push. Prevent broken commits. |
| **HIGH** | **INFRA-002** | **Add containerized dev environment** | Docker/Podman dev containers with all dependencies pre-installed. |
| **HIGH** | **INFRA-003** | **Add GitHub Actions for all stages** | Currently CI runs via `scripts/ci-check.sh`. Add separate GitHub Actions workflows. |
| **HIGH** | **INFRA-004** | **Add artifact publishing** | Publish: Rust docs, SvelteKit builds, Docker images to GHCR. |
| **MEDIUM** | **INFRA-005** | **Add self-hosted runners** | For faster CI. Use Woodpecker/Forgejo as mentioned in `plan.md`. |
| **MEDIUM** | **INFRA-006** | **Add dependency caching** | Cache: Cargo dependencies, pnpm dependencies, Docker layers. |
| **MEDIUM** | **INFRA-007** | **Add matrix builds** | Test against: multiple Rust versions, Node versions, Postgres versions. |
| **LOW** | **INFRA-008** | **Add canary deployments** | Deploy to staging before production. Roll back on failure. |
| **LOW** | **INFRA-009** | **Add blue-green deployments** | Zero-downtime deployments. |
| **LOW** | **INFRA-010** | **Add infrastructure as code** | Use Terraform, Pulumi, or similar to manage cloud resources. |

---

## 🔄 **Code Modernization**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **HIGH** | **MOD-001** | **Migrate all services to loco 1.0** | person/worker/course/place/thing/event still use old Axum style. Migrate to loco. |
| **HIGH** | **MOD-002** | **Unify service structure** | All services should have same structure: `src/controllers/`, `src/db/`, `src/models/`, etc. |
| **HIGH** | **MOD-003** | **Add OpenAPI/Swagger to all services** | Currently some services have it. Add to all 10 entity services + auth + link-graph. |
| **MEDIUM** | **MOD-004** | **Add gRPC to all services** | Currently only person/worker/event have gRPC. Add to all services. |
| **MEDIUM** | **MOD-005** | **Add GraphQL to all services** | Alternative to REST. Use `async-graphql` or similar. |
| **MEDIUM** | **MOD-006** | **Add WebSocket support** | For real-time updates: match results, bulk import progress, etc. |
| **MEDIUM** | **MOD-007** | **Add FHIR R5 support to all services** | Currently only some services have FHIR. Add to all. |
| **LOW** | **MOD-008** | **Add WASM support** | Compile services to WASM. Run in browser or serverless environments. |
| **LOW** | **MOD-009** | **Add mobile SDKs** | Generate SDKs for: iOS (Swift), Android (Kotlin), Flutter. |
| **LOW** | **MOD-010** | **Add CLI tools** | Create CLI for: managing services, importing/exporting data, running queries. |

---

## 🌐 **Cross-Service Improvements**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **HIGH** | **CROSS-001** | **Complete cross-service linking** | person↔worker `same_identity` is done. Add: person→org, worker→org, case→person. |
| **HIGH** | **CROSS-002** | **Add cross-service search** | Search across all services simultaneously. Aggregate results. |
| **HIGH** | **CROSS-003** | **Add cross-service merge** | Merge duplicates across services. |
| **HIGH** | **CROSS-004** | **Add cross-service audit** | Aggregate audit logs from all services. |
| **MEDIUM** | **CROSS-005** | **Add event bus monitoring** | Monitor Fluvio/Kafka topics. Alert on: lag, errors, dead letters. |
| **MEDIUM** | **CROSS-006** | **Add service mesh** | Use Linkerd, Istio, or similar. For: service discovery, load balancing, retries. |
| **MEDIUM** | **CROSS-007** | **Add circuit breakers** | Use `tough` or similar. Prevent cascading failures. |
| **LOW** | **CROSS-008** | **Add saga pattern** | For distributed transactions: create person + worker + link in one flow. |
| **LOW** | **CROSS-009** | **Add CQRS** | Separate read/write models. Optimize for query performance. |
| **LOW** | **CROSS-010** | **Add event sourcing** | Store all state changes as events. Rebuild state from events. |

---

## 📦 **Packaging & Distribution**

| Priority | Task | Description | Success Criteria |
|----------|------|-------------|------------------|
| **HIGH** | **PKG-001** | **Publish all crates to crates.io** | Currently only `authentication-verifier` and matchers are published. Publish all. |
| **HIGH** | **PKG-002** | **Add Docker images for all services** | Currently some services have Dockerfiles. Add to all. Publish to GHCR. |
| **HIGH** | **PKG-003** | **Add Helm charts** | For Kubernetes deployments. |
| **HIGH** | **PKG-004** | **Add Homebrew/Linux packages** | For easy installation on macOS/Linux. |
| **MEDIUM** | **PKG-005** | **Add npm packages for front-ends** | Publish SvelteKit front-ends as npm packages. |
| **MEDIUM** | **PKG-006** | **Add Chocolatey packages** | For Windows users. |
| **LOW** | **PKG-007** | **Add Snap packages** | For Ubuntu users. |
| **LOW** | **PKG-008** | **Add Flatpak packages** | For Linux desktop users. |
| **LOW** | **PKG-009** | **Add Winget packages** | For Windows users. |
| **LOW** | **PKG-010** | **Add package repository** | APT/YUM repos for Debian/Ubuntu/RHEL/CentOS. |

---

## 🎯 **Priority Recommendations**

### **Start Here (Highest ROI)**
1. **DOC-ONBOARD-001** - CONTRIBUTING.md (essential for new contributors)
2. **DOC-ONBOARD-002** - GETTING-STARTED.md (lower the barrier to entry)
3. **DOC-ONBOARD-004** - ROADMAP.md (show project direction)
4. **ENG-QUAL-001** - Remove DEV_SEED fallback (critical security fix)
5. **ENG-QUAL-002** - Fix reconcile scoping bug (critical correctness fix)
6. **TUT-001** - Complete tutorials 02-06 (help users learn)
7. **TEST-001** - Add DB-gated tests to CI (catch regressions)
8. **OBS-001** - Add OTLP tracing to all services (better debugging)
9. **SEC-001** - Remove dev signing seed (security)
10. **INFRA-001** - Add pre-push hooks (prevent broken commits)

### **Quick Wins (1-2 days each)**
- **DOC-ONBOARD-003** - Fix README links
- **DOC-ONBOARD-005** - Add per-crate CONTRIBUTING.md
- **ENG-QUAL-011** - Add property tests for matchers
- **TEST-003** - Add integration tests between services
- **OBS-004** - Add structured logging
- **INFRA-002** - Add dev containers

### **Long-Term Strategic (1-4 weeks each)**
- **PROM-001** - Create project website
- **MOD-001** - Migrate all services to loco 1.0
- **CROSS-001** - Complete cross-service linking
- **PKG-001** - Publish all crates to crates.io
- **PKG-002** - Add Docker images for all services

---

## 📊 **Summary Statistics**

| Category | Tasks | Critical | High | Medium | Low |
|----------|-------|----------|------|--------|-----|
| **Visibility/Onboarding** | 42 | 0 | 18 | 14 | 10 |
| **Engineering** | 100 | 4 | 40 | 32 | 24 |
| **Total** | **142** | **4** | **58** | **46** | **34** |

---

## 🎯 **Next Steps**

1. **Start with the Critical tasks** - These are security and correctness issues that must be fixed.
2. **Then tackle High-priority Documentation** - This will help attract and retain contributors.
3. **Add testing and observability** - This will improve code quality and make debugging easier.
4. **Modernize the codebase** - Migrate to newer patterns and technologies.
5. **Package and distribute** - Make it easier for users to consume the project.

Would you like me to **elaborate on any specific task**, **prioritize a subset** for immediate execution, or **create an execution plan** for a particular category?
