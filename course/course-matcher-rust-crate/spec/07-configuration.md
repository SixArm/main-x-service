## 7. Configuration

`src/config.rs::MatchConfig`:

| Field | Default |
|---|---|
| `threshold` | 0.85 |
| `name_weight` | 0.35 |
| `course_code_weight` | 0.15 |
| `provider_weight` | 0.15 |
| `educational_level_weight` | 0.10 |
| `keywords_weight` | 0.10 |
| `teaches_weight` | 0.15 |
| `relationships_weight` | 0.05 |
| `tags_weight` | 0.05 |

`relationships_weight` (0.05) and `tags_weight` (0.05) sit in the
**supporting-signal** cluster: a low weight each because shared course
references / shared operational tags corroborate but never
single-handedly establish identity. The six identifying weights still
sum to 1.00; per §17 every weight (including the supporting
`relationships_weight` and `tags_weight`) is renormalised over the
*present* components, so the running denominator is the present weight,
not a fixed total.

> Change-control: `relationships_weight` was added with the §5.1
> relationships component; `tags_weight` was added with the §5.2 tags
> component. Per the crate's change rule, do not alter either without
> editing this section **and** `CHANGELOG.md`.

Convenience presets: `MatchConfig::strict()` (threshold = 0.95) and
`MatchConfig::lenient()` (threshold = 0.70). Same weights.

