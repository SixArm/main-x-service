# Glossary

> Part of the [Loco edition specification](index.md). Shared domain
> vocabulary: [root glossary](../../spec/glossary.md). Loco/Rust terms:

| Term           | Meaning                                                                                     |
| -------------- | ------------------------------------------------------------------------------------------- |
| Loco           | Rust web framework, "Rails for Rust"; ships with Axum, SeaORM, workers, tasks.              |
| SeaORM         | Async ORM for Rust; Loco's default DB layer.                                                |
| AppContext     | Loco's per-request struct that bundles the DB connection, config, etc.                      |
| `Routes`       | Loco's wrapper around an Axum router with prefix + add helpers.                             |
| Modulus 11     | Check-digit algorithm used by NHS Numbers.                                                  |
| Main-X-Service | The five upstream HTTP services (Patient, Place, Worker, Thing, Event) the tracker proxies. |
| RoutingClient  | The private wrapper that lets request tests swap a stub client into a `Mutex` slot.         |
| StubClient     | In-process fake upstream used by request tests and stub mode.                               |
