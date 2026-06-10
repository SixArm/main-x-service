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
                |  |  ThingRepository     |  |
                |  |  (lib/api/things.ts) |  |
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
                |   thing-service-rust-crate |
                |   Axum + SeaORM + Tantivy   |
                +-----------------------------+
```

