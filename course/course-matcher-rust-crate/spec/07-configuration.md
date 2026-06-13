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

Sum of weights = 1.00. Per §17 they're renormalised over the
*present* components.

Convenience presets: `MatchConfig::strict()` (threshold = 0.95) and
`MatchConfig::lenient()` (threshold = 0.70). Same weights.

