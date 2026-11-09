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
| `components` | Every term of the sum as `ComponentBits { component, bits, correlated, frequency_adjusted }`: one per compared component (`name`, `provider`, `educational_level`, `keywords`, `teaches`, `relationships`, `tags`), then `identifier` when an identifier rule hit, then `contradiction:identifier:<scheme>` per contradiction. The terms add up to `weight_bits` exactly (`weight_bits` is their sum, in list order). `#[serde(default)]`. |

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

### Correlated groups (public contract)

Per `match-evidence.md` §6, within each group of `CORRELATED_GROUPS`
the present member with the largest absolute bits (ties to the larger
value) counts in full and every other present member counts at
`CORRELATED_DISCOUNT` = 0.5 (`group_bits`, `strongest_first`). Its
term is marked `correlated: true`. Disagreements are discounted the same
way. The result does not depend on component order.

| Group | Why the members are correlated |
|---|---|
| `keywords`, `teaches`, `tags` | Often authored from one syllabus or catalogue entry: two unrelated courses on one subject agree on all three for one reason. |
| `provider`, `educational_level` | A provider's catalogue sits at a few levels (a university's is all higher education), so two unrelated courses from one provider usually share a level as well. |

`name` and `relationships` stand alone. The course code is weighed as an
identifier, not a component (above), so it is in no group.

### Value frequencies (public contract)

Per `match-evidence.md` §7, `MatchingEngine::match_courses_with_frequencies(a, b, &dyn ValueFrequencies)`
takes a frequency source; `match_courses` calls it with `NoFrequencies`,
so plain calls are unchanged. `score`, `is_match`, `confidence` and
`breakdown` are identical with and without a source.

`FREQUENCY_COMPONENTS` = `["name"]`. Course titles such as
"Introduction to Biology" are offered by many providers, so agreeing on
one proves far less than the table's average `u`.

| Component | Key (`frequency_key(component, raw)`) | Adjusted when |
|---|---|---|
| `name` | the **primary** name under `normalize::fold` (trim, NFKC, lowercase); `None` when that is empty | both primary names give the same key and the source knows its share |

The agreement weight is then `frequency_bits(m, share)` =
`log2(m / clamp(share, MIN_SHARE = 1e-9, m))`, so a share at or above
`m` gives `0` bits, never negative. Partial agreement (e.g. "Intro to
Biology" against "Introduction to Biology", or a match on an alternate
name) and disagreement keep the m/u table. Any other component key gives
`None`. `FrequencyTable` (`new`, `insert(component, key, share)`,
`from_counts(counts, total)` from `(component, key, count)` triples)
is provided; a service computes `count / total` over its active records
with keys from `frequency_key`.

Examples (name m = 0.95): share 0.01 gives +6.57 bits (below the
table's +7.57); share 1e-6 gives +19.86 bits.

### What the data can prove

Two identical records holding only a name weigh **+7.57 bits**:
`Consistent`, not `Proven`. The richest identifier-free record this
crate models (name, provider, level, keywords, teaches, relationships,
tags, all agreeing) weighs **≈ 23.62 bits** with the correlated groups
(27.6 before them), below the default bar of 30.
That is deliberate: two sections or two years of one course agree on all
of those fields and are still separate records. Without an identifier or
a provider-scoped course code, course data is consistent with a
duplicate but never proof of one. A deployment may lower `proof_bits`.
