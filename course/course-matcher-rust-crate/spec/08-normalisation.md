## 8. Normalisation

`src/normalize.rs`:

- `fold(s)` — trim → NFKC → lowercase.
- `course_code(s)` — strip whitespace → uppercase.
- `fold_set(xs)` — fold each → drop empties → sort → dedup.

Detailed rules: [`AGENTS/normalization.md`](../AGENTS/normalization.md).

