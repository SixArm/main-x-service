## 21. Compatibility

- Crate version follows semver. Pre-1.0 we allow patch-level field
  renames; from 1.0 onward field renames are major bumps.
- Adding a new optional field to `Course` is patch-level.
- Changing default weights is **minor** — semantically observable to
  downstream tests.
- Changing `IdentifierScheme::is_deterministic` for an existing
  variant is **major** — could create false positives.

