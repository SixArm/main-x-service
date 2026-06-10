## 7. Non-Functional Requirements

- **Throughput:** ≥1000 req/s sustained on a single 4-core host for `GET /courses/{id}`.
- **Latency p95:** `GET /courses/{id}` ≤ 25 ms; `GET /courses/search` ≤ 100 ms; `POST /courses/match` ≤ 500 ms.
- **Bundle size (binary):** < 30 MB stripped.
- **Memory:** ≤ 256 MB resident for 1M courses + 5M instances indexed.
- **Search consistency:** a `POST /courses` MUST be observable via `GET /courses/search` immediately on subsequent requests (`SearchEngine::reload()` after every commit, matching person-service).

