## 18a. Match evidence

`MatchResult.evidence` reports how much the compared data **proves**,
beside the similarity `score` that reports how alike it is. The
family-wide contract is
[`agents/share/match-evidence.md`](../../../agents/share/match-evidence.md)
(Fellegi–Sunter log-likelihood weights); this section records only what
is specific to this crate. `score`, `is_match` and `confidence` are
unchanged by it (§17, §18).

Code: `src/evidence.rs` (`component_bits`, `AGREE_AT` = 0.95,
`DISAGREE_AT` = 0.60, `IDENTIFIER_BITS` ≈ 39.85, `Evidence`,
`Contradiction`, `MatchStrength`), re-exported from the crate root.

### Outputs

| Field | Meaning |
|---|---|
| `weight_bits` | Sum of per-component weights plus the identifier contribution, in bits. A component either record lacks adds `0`. |
| `fields_compared` | Components both records held; all identifier rules together count as one. |
| `contradictions` | Hard identifier conflicts (below). |
| `strength` | `Contradicted`, else `Proven` (`weight_bits >= MatchConfig::proof_bits`), else `Consistent` (`is_match`), else `NonMatch`. |

`MatchResult.evidence` is `#[serde(default)]`, and
`MatchConfig::proof_bits` (default **30**, §7) is
`#[serde(default = …)]`, so JSON written before either field existed
still deserialises.

Evidence is computed on every path. When the deterministic
short-circuit (§15, §16) fires, the components are still computed for
the evidence; the returned `breakdown` stays as it was (all `None`,
`deterministic_match: true`).

### Per-component m/u table

Informed starting estimates, not measurements. Agreement adds
`log2(m/u)`, disagreement `log2((1−m)/(1−u))`.

| Component | m | u | agree | disagree | Where `u` comes from |
|---|---|---|---|---|---|
| name | 0.95 | 0.005 | +7.57 | −4.31 | generic titles recur across providers |
| provider | 0.95 | 0.05 | +4.25 | −4.25 | many courses per provider in one registry |
| educational level | 0.90 | 0.20 | +2.17 | −3.00 | a handful of levels, unevenly used |
| keywords | 0.80 | 0.10 | +3.00 | −2.17 | subject keywords shared across a field |
| teaches | 0.80 | 0.05 | +4.00 | −2.25 | shared by courses with one syllabus |
| relationships | 0.70 | 0.05 | +3.81 | −1.66 | sibling courses cite the same related courses |
| tags | 0.70 | 0.10 | +2.81 | −1.58 | coarse operator-applied labels |

The **course-code** component (§10) is not in the table: within one
provider a course code is an identifier, so it is weighed as one (below)
rather than counted twice.

### Identifiers

`IDENTIFIER_BITS` is added **once** when any of these holds:

- a shared value under a globally unique scheme — DOI, Wikidata, OER,
  LOM, URI, UUID (§15 R-0);
- an overlapping `same_as` URL (§16 R-2);
- the same non-empty `provider_id` and the same non-empty normalised
  `course_code` (§15 R-1). A course code is identity **only within one
  provider**, so across providers it is neither an identifier hit nor a
  contradiction.

### Contradictions

Each adds `log2(0.01 / (1 − 1e-12))` ≈ −6.64 bits.

- `identifier:<scheme>` (`doi`, `wikidata`, `oer`, `lom`, `uri`,
  `uuid`): both records hold at least one non-empty value (after
  `normalize::fold`) under that scheme, and the value sets share none.
  A record holding an old and a new value is not contradicted by a
  partner holding either.
- `identifier:course_code`: the same non-empty provider, both codes
  non-empty after `normalize::course_code`, and the codes differ.

Disjoint `same_as` sets are **not** a contradiction: one record may
cite a Wikidata page and the other an OER entry for the same course.

### What the data can prove

Two identical records holding only a name weigh **+7.57 bits**:
`Consistent`, not `Proven`. The richest identifier-free record this
crate models (name, provider, level, keywords, teaches, relationships,
tags, all agreeing) weighs **≈ 27.6 bits**, below the default bar of 30.
That is deliberate: two sections or two years of one course agree on all
of those fields and are still separate records. Without an identifier or
a provider-scoped course code, course data is consistent with a
duplicate but never proof of one. A deployment may lower `proof_bits`.
