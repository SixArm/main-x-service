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
                |  |  PlaceRepository     |  |
                |  |  (lib/api/places.ts) |  |
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
                |   place-service-rust-crate |
                |   Axum + SeaORM + Tantivy   |
                +-----------------------------+
```

