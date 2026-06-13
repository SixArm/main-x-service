# main-x-service

Monorepo for web services: the **Main X Index** family — a federated
identity index, one entity per directory. Each entity directory holds
a front-end web app, a matcher (or verifier) library crate, a service
API crate, plus entity-level `spec/` and `AGENTS/` umbrella docs.

| Entity | Front-end (Svelte) | Library crate | Service crate | Umbrella docs |
| ------ | ------------------ | ------------- | ------------- | ------------- |
| authentication | [authentication-front-end-with-svelte](../authentication/authentication-front-end-with-svelte/) | [authentication-verifier-rust-crate](../authentication/authentication-verifier-rust-crate/) | [authentication-service-rust-crate](../authentication/authentication-service-rust-crate/) | [spec](../authentication/spec/index.md) · [AGENTS](../authentication/AGENTS/index.md) |
| care-pathway | [care-pathway-front-end-with-svelte](../care-pathway/care-pathway-front-end-with-svelte/) | [care-pathway-matcher-rust-crate](../care-pathway/care-pathway-matcher-rust-crate/) | [care-pathway-service-rust-crate](../care-pathway/care-pathway-service-rust-crate/) | [spec](../care-pathway/spec/index.md) · [AGENTS](../care-pathway/AGENTS/index.md) |
| case | [case-front-end-with-svelte](../case/case-front-end-with-svelte/) | [case-matcher-rust-crate](../case/case-matcher-rust-crate/) | [case-service-rust-crate](../case/case-service-rust-crate/) | [spec](../case/spec/index.md) · [AGENTS](../case/AGENTS/index.md) |
| course | [course-front-end-with-svelte](../course/course-front-end-with-svelte/) | [course-matcher-rust-crate](../course/course-matcher-rust-crate/) | [course-service-rust-crate](../course/course-service-rust-crate/) | [spec](../course/spec/index.md) · [AGENTS](../course/AGENTS/index.md) |
| event | [event-front-end-with-svelte](../event/event-front-end-with-svelte/) | [event-matcher-rust-crate](../event/event-matcher-rust-crate/) | [event-service-rust-crate](../event/event-service-rust-crate/) | [spec](../event/spec/index.md) · [AGENTS](../event/AGENTS/index.md) |
| organization | [organization-front-end-with-svelte](../organization/organization-front-end-with-svelte/) | [organization-matcher-rust-crate](../organization/organization-matcher-rust-crate/) | [organization-service-rust-crate](../organization/organization-service-rust-crate/) | [spec](../organization/spec/index.md) · [AGENTS](../organization/AGENTS/index.md) |
| person | [person-front-end-with-svelte](../person/person-front-end-with-svelte/) | [person-matcher-rust-crate](../person/person-matcher-rust-crate/) | [person-service-rust-crate](../person/person-service-rust-crate/) | [spec](../person/spec/index.md) · [AGENTS](../person/AGENTS/index.md) |
| place | [place-front-end-with-svelte](../place/place-front-end-with-svelte/) | [place-matcher-rust-crate](../place/place-matcher-rust-crate/) | [place-service-rust-crate](../place/place-service-rust-crate/) | [spec](../place/spec/index.md) · [AGENTS](../place/AGENTS/index.md) |
| thing | [thing-front-end-with-svelte](../thing/thing-front-end-with-svelte/) | [thing-matcher-rust-crate](../thing/thing-matcher-rust-crate/) | [thing-service-rust-crate](../thing/thing-service-rust-crate/) | [spec](../thing/spec/index.md) · [AGENTS](../thing/AGENTS/index.md) |
| worker | [worker-front-end-with-svelte](../worker/worker-front-end-with-svelte/) | [worker-matcher-rust-crate](../worker/worker-matcher-rust-crate/) | [worker-service-rust-crate](../worker/worker-service-rust-crate/) | [spec](../worker/spec/index.md) · [AGENTS](../worker/AGENTS/index.md) |

## Monorepo-wide docs

| Document | Description |
| -------- | ----------- |
| [data.md](data.md) | Data conventions |
| [data-modeling.md](data-modeling.md) | Data-modeling rules (SQL-first, child tables, discriminators, JSONB policy) |
| [../AGENTS.md](../AGENTS.md) | Root agent guide — subproject directory and shared reference docs |
| [../agents/share/index.md](../agents/share/index.md) | Shared reference docs (architecture, matching, compliance, …) |
