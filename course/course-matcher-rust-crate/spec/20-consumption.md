## 20. Consumption

Embedded as a path dependency by
[`course-service`](../../course-service-rust-crate/) via an `adapter`
module. The service's `Course` is the richer schema; the adapter
projects it down to the matcher's `Course` and back-fills missing
fields with defaults.

The adapter lives in the service crate (not here). This crate has no
SeaORM / Axum / Tantivy dependencies.

