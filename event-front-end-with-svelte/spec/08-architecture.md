## 8. Architecture

```
                +-----------------------------+
                |        Browser (SPA)        |
                |  +-----------------------+  |
                |  |  SvelteKit routes     |  |
                |  |  + Svelte 5 components|  |
                |  +----------+------------+  |
                |             |               |
                |             v               |
                |  +-----------------------+  |
                |  |  EventRepository     |  |
                |  |  (lib/api/v1/events.ts) |  |
                |  +----------+------------+  |
                |             |               |
                |             v               |
                |  +-----------------------+  |
                |  |  ApiClient            |  |
                |  |  (lib/api/client.ts)  |  |
                |  +----------+------------+  |
                +-------------|---------------+
                              | HTTP JSON
                              v
                +-----------------------------+
                |   event-service-rust-crate |
                |   Axum + SeaORM + Tantivy   |
                +-----------------------------+
```

