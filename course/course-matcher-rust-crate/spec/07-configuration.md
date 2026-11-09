## 7. Configuration

`src/config.rs::MatchConfig` — as shipped:

| Field | Default |
|---|---|
| `threshold` | 0.85 |
| `name_weight` | 0.35 |
| `course_code_weight` | 0.15 |
| `provider_weight` | 0.15 |
| `educational_level_weight` | 0.10 |
| `keywords_weight` | 0.10 |
| `teaches_weight` | 0.15 |

These six identifying weights sum to 1.00 (pinned by the
`default_weights_sum_to_one` test in `src/config.rs`); per §17 they are
renormalised over the *present* components, so the running denominator
is the present weight, not a fixed total.

Two further weights (§23 T-11 / T-12, shipped):

| Field | Default |
|---|---|
| `relationships_weight` | 0.05 |
| `tags_weight` | 0.05 |

`relationships_weight` and `tags_weight` sit in a **supporting-signal**
cluster: a low weight each because shared course references / shared
operational tags corroborate but never single-handedly establish
identity. They are deliberately **excluded** from the six-weight
partition of unity above and from `MatchConfig::weight_total()` (test-only) —
adding them there would break the documented "sums to 1.00" invariant
for no benefit, since the renormaliser divides by the *present* weight
regardless.

> Change-control: when `relationships_weight` / `tags_weight` land
> (§5.1 / §5.2), do not alter either without editing this section
> **and** `CHANGELOG.md`.

Convenience presets: `MatchConfig::strict()` (threshold = 0.95) and
`MatchConfig::lenient()` (threshold = 0.70). Same weights.

Evidence bar (§18a, §23 T-16):

| Field | Default |
|---|---|
| `proof_bits` | 30.0 |

The bits `MatchResult.evidence.weight_bits` must reach for
`MatchStrength::Proven` (`agents/share/match-evidence.md` §4). It is
`#[serde(default)]`, so a config serialised before the field existed
still loads. The presets keep the default. Like the weights, it is not
validated (§22): a caller setting it non-positive makes every
uncontradicted pair `Proven`.

