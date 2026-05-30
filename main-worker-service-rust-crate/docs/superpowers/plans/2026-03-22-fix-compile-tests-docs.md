# MPI Fix, Test & Docs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all 60 compilation errors, add comprehensive tests, and update all documentation files.

**Architecture:** Systematic fix of missing `.await` calls on async repository methods in API handlers, type reference corrections, then test expansion across unit/integration/benchmark/REST/gRPC, then documentation update.

**Tech Stack:** Rust, SeaORM, Axum, Tantivy, Tonic, Criterion

---

### Task 1: Fix REST API Handlers — Missing `.await` on Async Calls

**Files:**

- Modify: `src/api/rest/handlers.rs`

The `WorkerRepository` trait uses `async fn` methods. All `match state.worker_repository.*()` and `state.audit_log.*()` calls need `.await`. Also fix `DbAuditLog` → `crate::db::models::audit_log::Model` (3 occurrences) and cast audit `limit` from `i64` to `u64`.

- [ ] **Step 1:** Add `.await` to every `state.worker_repository.create/get_by_id/update/delete/list_active()` call
- [ ] **Step 2:** Add `.await` to every `state.audit_log.get_logs_for_entity/get_recent_logs/get_logs_by_user()` call
- [ ] **Step 3:** Replace `crate::db::models::DbAuditLog` with `crate::db::models::audit_log::Model` (3 occurrences)
- [ ] **Step 4:** Cast audit `limit` from `i64` to `u64` in the 3 audit handler functions
- [ ] **Step 5:** Run `cargo check 2>&1 | grep "src/api/rest/handlers"` — Expected: no errors

### Task 2: Fix FHIR API Handlers — Missing `.await` on Async Calls

**Files:**

- Modify: `src/api/fhir/handlers.rs`

Same pattern — all `state.worker_repository.*()` calls need `.await`.

- [ ] **Step 1:** Add `.await` to every `state.worker_repository.create/get_by_id/update/delete()` call
- [ ] **Step 2:** Run `cargo check 2>&1 | grep "src/api/fhir/handlers"` — Expected: no errors

### Task 3: Fix Repositories — Missing `Expr` Import

**Files:**

- Modify: `src/db/repositories.rs`

The `search()` method uses `Expr::cust_with_values` but `Expr` isn't directly available from `use sea_orm::*`.

- [ ] **Step 1:** Add `use sea_orm::sea_query::Expr;` import
- [ ] **Step 2:** Run `cargo check 2>&1 | grep "src/db/repositories"` — Expected: no errors

### Task 4: Fix utoipa-swagger-ui Build Cache

- [ ] **Step 1:** Run `cargo clean -p utoipa-swagger-ui`
- [ ] **Step 2:** Run `cargo check` — Expected: full compilation succeeds with 0 errors
- [ ] **Step 3:** Run `cargo test --lib` — Expected: all unit tests pass
- [ ] **Step 4:** Commit: "fix: resolve 60 compilation errors — add missing .await, fix type refs, fix imports"

### Task 5: Add Comprehensive Unit Tests

**Files:**

- Modify: `src/matching/algorithms.rs` — add edge-case tests
- Modify: `src/matching/phonetic.rs` — add edge-case tests
- Modify: `src/matching/scoring.rs` — add edge-case tests
- Modify: `src/validation/mod.rs` — add comprehensive validation tests
- Modify: `src/privacy/mod.rs` — add privacy/masking tests
- Modify: `src/models/worker.rs` — add model construction tests
- Modify: `src/models/document.rs` — add document validation tests
- Modify: `src/models/emergency_contact.rs` — add emergency contact tests
- Modify: `src/search/index.rs` — add search engine tests
- Modify: `src/db/repositories.rs` — add repository unit tests (mocked)
- Modify: `src/db/audit.rs` — add audit log tests (mocked)

- [ ] **Step 1:** Add matching algorithm edge-case tests (empty strings, unicode, case sensitivity)
- [ ] **Step 2:** Add scoring tests (boundary conditions, deterministic short-circuit)
- [ ] **Step 3:** Add validation tests (all field validators, normalization)
- [ ] **Step 4:** Add privacy/masking tests (all sensitive field types)
- [ ] **Step 5:** Add model tests (construction, serialization, defaults)
- [ ] **Step 6:** Add search index tests (indexing, retrieval, fuzzy, deletion)
- [ ] **Step 7:** Run `cargo test --lib` — Expected: all tests pass
- [ ] **Step 8:** Commit: "test: add comprehensive unit tests"

### Task 6: Add Integration Tests

**Files:**

- Modify: `tests/api_integration_test.rs` — expand REST API tests
- Create: `tests/fhir_integration_test.rs` — FHIR endpoint tests

- [ ] **Step 1:** Add integration tests for search, match, merge, deduplicate, export, masked, audit endpoints
- [ ] **Step 2:** Add FHIR integration tests (CRUD, search, bundles)
- [ ] **Step 3:** Run `cargo test --test api_integration_test` (requires PostgreSQL)
- [ ] **Step 4:** Commit: "test: add integration tests for REST and FHIR APIs"

### Task 7: Add Benchmark Tests

**Files:**

- Create: `benches/matching_bench.rs` — matching algorithm benchmarks
- Create: `benches/search_bench.rs` — search engine benchmarks
- Create: `benches/validation_bench.rs` — validation benchmarks

- [ ] **Step 1:** Create matching benchmarks (name matching, full worker matching, scoring)
- [ ] **Step 2:** Create search benchmarks (indexing, searching, fuzzy search)
- [ ] **Step 3:** Create validation benchmarks (validate_worker, normalize)
- [ ] **Step 4:** Run `cargo bench` — Expected: benchmarks compile and run
- [ ] **Step 5:** Commit: "test: add criterion benchmark tests"

### Task 8: Update Documentation

**Files:**

- Modify: `AGENTS/index.md`
- Modify: `AGENTS/architecture.md`
- Modify: `AGENTS/matching.md`
- Modify: `AGENTS/models.md`
- Modify: `AGENTS/restful.md`
- Modify: `AGENTS/testing.md`
- Modify: `CLAUDE.md`
- Modify: `plan.md`
- Create: `tasks.md`

- [ ] **Step 1:** Update AGENTS/index.md with complete file listing
- [ ] **Step 2:** Fill in AGENTS/matching.md with algorithm details
- [ ] **Step 3:** Fill in AGENTS/models.md with domain model reference
- [ ] **Step 4:** Update AGENTS/restful.md with complete endpoint docs
- [ ] **Step 5:** Fill in AGENTS/testing.md with test strategy and coverage
- [ ] **Step 6:** Update AGENTS/architecture.md with current architecture
- [ ] **Step 7:** Update CLAUDE.md test counts and coverage
- [ ] **Step 8:** Update plan.md with current status
- [ ] **Step 9:** Create tasks.md with remaining work items
- [ ] **Step 10:** Commit: "docs: update all documentation files"
