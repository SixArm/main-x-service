## 11. Provider

- When both have `provider_id`:
  - Equal → 1.0; not equal → 0.0.
- Else when both have `provider_name`:
  - `jaro_winkler(fold(a), fold(b))`.
- Else `None`.

