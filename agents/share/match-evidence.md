# Match evidence — consistent with a duplicate vs. proven duplicate

How every matcher crate tells **"nothing here contradicts a match"** from
**"the data proves a match"**. A similarity score cannot do both. Two
records holding only the same name and birth date agree perfectly on
everything they hold, and so do twins. A score of `1.0` for that pair says
the present data is alike, not that the pair is one entity. A merge
deletes a record and is hard to undo, so the family gates it on evidence,
not on similarity.

This is a design document. It fixes the evidence model, the formula, the
result shape, the contradiction rule and the default bar, so each matcher
adopts it identically. Only the per-component `m`/`u` table, the
correlated groups and the frequency components differ per crate
(§5–§7).

## 1. Two questions, two numbers

| Output | Question | Missing data | Used for |
|---|---|---|---|
| `score` (existing, `0.0`–`1.0`) | How alike is the data both records hold? | dropped from numerator and denominator | ranking, `is_match`, review queue |
| `evidence.weight_bits` (new) | How much does the data prove? | contributes exactly `0` | auto-merge, delete |
| `evidence.contradictions` (new) | Does any hard fact rule the pair out? | cannot contradict | blocks merging at any score |

`score`, `is_match` and `confidence` keep their meaning and their values.
Evidence is added beside them; nothing existing changes.

Renormalising is right for `score` and wrong as the merge gate. Counting a
missing field as `0` is wrong for both (it was event-service T-16 and
worker-service T-23): absence is not disagreement.

## 2. The evidence formula (Fellegi–Sunter match weights)

Each compared component carries two probabilities:

- **`m`** — the chance the component agrees when the records are the same
  entity (below `1` because of typos, moves, name changes).
- **`u`** — the chance it agrees by coincidence when they are different
  entities (high for gender or country, tiny for a street address).

Agreement adds `A = log2(m / u)` bits; disagreement adds
`D = log2((1 − m) / (1 − u))`, which is negative. A component either
record lacks adds `0` and is not counted. The total is a log-likelihood
ratio in bits: each bit doubles the odds that the pair is one entity.

Matcher components are similarities, not agree/disagree flags, so a
similarity `s` maps onto the line between `D` and `A`:

```rust
/// Similarity at or above which a component counts as full agreement.
pub const AGREE_AT: f64 = 0.95;
/// Similarity at or below which a component counts as full disagreement.
pub const DISAGREE_AT: f64 = 0.60;

/// The weight, in bits, one compared component adds.
#[must_use]
pub fn component_bits(similarity: f64, m: f64, u: f64) -> f64 {
    let agree = (m / u).log2();
    let disagree = ((1.0 - m) / (1.0 - u)).log2();
    let s = if similarity.is_finite() { similarity.clamp(0.0, 1.0) } else { 0.0 };
    if s >= AGREE_AT {
        agree
    } else if s <= DISAGREE_AT {
        disagree
    } else {
        disagree + (agree - disagree) * (s - DISAGREE_AT) / (AGREE_AT - DISAGREE_AT)
    }
}
```

The weight is the sum of `component_bits` over every component that is
`Some` in the breakdown, plus the identifier contribution (§3).

## 3. Identifiers

A value shared under a **globally unique scheme**, the same one the crate's
deterministic short-circuit already uses, contributes
`IDENTIFIER_BITS = log2(0.99 / 1e-12) ≈ 39.85` bits once per pair. That
clears the default bar (§4) alone, which is the meaning of the
short-circuit.

The person and worker matchers also short-circuit on an exact
name + birth date + gender tuple. That rule is **not** an identifier and
adds no identifier bits: its components are weighed like any others. It
is exactly the "consistent, not proven" case this document exists for.

A crate whose short-circuit returns before computing components still
computes evidence: the identifier contribution plus whatever components it
does compute.

### Contradictions

A **contradiction** is recorded for each unique scheme under which **both**
records hold at least one non-empty value and the two value sets share
none. Two different LEIs, or two different NHS numbers, mean the records
are different entities or one record is wrong. Either way, a person must
look. A contradicted scheme also adds `log2(0.01 / (1 − 1e-12)) ≈ −6.64`
bits.

A record with several values under one scheme (an old and a new number) is
not contradicted by a partner holding either of them. Blank values, after
the crate's own normalisation, are ignored, as for the short-circuit
(SEC-M2).

**Document numbers prove but never contradict.** A passport or similar
document number identifies a document, not its holder, and changes on
every renewal. A shared `(country, number)` pair adds `IDENTIFIER_BITS`,
but two different passports record no contradiction, or a renewed
passport would contradict its holder's older record (person and worker).
For the same reason, `same_as` URL sets never contradict: one record may
cite a Wikidata page and the other an OER entry for the same thing.

**Placeholder scores are missing data.** Some components return a neutral
`0.5` when neither side holds a comparable sub-field (an address with no
shared part). Evidence treats that as absent, not as a partial
disagreement (place, worker). Event's empty-location `0.5` (its OQ-J)
still weighs as one, and is recorded there.

## 4. Strength

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStrength {
    /// A contradiction rules the pair out of any automatic merge.
    Contradicted,
    /// The evidence clears `proof_bits`, with no contradiction.
    Proven,
    /// `is_match`, no contradiction, but the evidence is short of proof.
    Consistent,
    /// Not a match.
    #[default]
    NonMatch,
}
```

Checked in that order: any contradiction gives `Contradicted`, even with a
shared identifier elsewhere. Then the bar gives `Proven`, then `is_match`
gives `Consistent`.

**The bar.** `MatchConfig::proof_bits`, default **30**. A registry of `N`
records puts the prior odds that a random pair is one entity near `1/N`.
Reaching posterior odds of `1/α`, where `α` is the false-merge rate a
deployment accepts, needs `log2(N) + log2(1/α)` bits. A million records at
one false merge in a thousand needs `20 + 10 = 30`. A deployment sets its
own bar; the default is deliberately strict, because a false merge mixes
two people's records.

What each strength permits:

| Strength | Auto-merge | Review queue | Delete either record |
|---|---|---|---|
| `Proven` | allowed | optional | allowed, through merge |
| `Consistent` | **never** | yes | no; link or review instead |
| `Contradicted` | **never** | yes, flagged | no |
| `NonMatch` | no | no | no |

The matcher only reports a strength. Services enforce the table when they
adopt it (§7).

## 5. The result shape (identical in every matcher)

```rust
/// One hard conflict between the two records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contradiction {
    /// What conflicts, e.g. `"identifier:lei"`.
    pub field: String,
}

/// How much the compared data proves, beside the similarity score.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Evidence {
    /// Sum of the components' log-likelihood weights, in bits.
    pub weight_bits: f64,
    /// Components both records held (identifiers count as one).
    pub fields_compared: u32,
    /// Hard conflicts; any one makes the strength `Contradicted`.
    pub contradictions: Vec<Contradiction>,
    /// The verdict (§4).
    pub strength: MatchStrength,
}
```

- `MatchResult` gains `#[serde(default)] pub evidence: Evidence`, so JSON
  written before the field still deserialises.
- `MatchConfig` gains `#[serde(default = "default_proof_bits")] pub proof_bits: f64`.
  Where a crate validates its config, `proof_bits` must be finite and
  positive.
- The code lives in each crate's own `src/evidence.rs`, copied rather than
  shared, the same posture as the rest of the matcher family. Each crate
  re-exports `Evidence`, `Contradiction` and `MatchStrength` from its root.

**The per-component table** is each crate's `m`/`u` constants in
`src/evidence.rs`, one entry per breakdown component, each with a line on
where its `u` comes from. A starting table is an informed estimate, not a
measurement. Rules of thumb:

- `u` is roughly the chance that two unrelated records agree on the
  field: about `0.5` for gender, `1/365` for a calendar day, small for a
  full name, tiny for a street address or URL.
- `m` sits between `0.8` and `0.99`, lower where the field legitimately
  changes over time (address, organiser, tags).
- Set-overlap components (keywords, tags, relationships) carry weak
  weights: overlapping sets corroborate, they do not identify.

## 6. Correlated components

The sum in §2 assumes each component is independent of the others, given
whether the pair is one entity. Some are not, and summing them counts one
fact more than once:

- **Derived from each other.** Start and end date of one event; a venue
  and its address.
- **Recorded together.** A household's address, landline and postcode
  move together. When one agrees because two people live in the same
  house, the others agree for the same reason.
- **Determined by one fact.** Death date and death place; an
  organization's address and its jurisdiction.

Two records from one household then look like `Proven` duplicates when
the only fact they share is the house.

**The rule.** Each crate declares `CORRELATED_GROUPS`, a list of groups of
component names. Within a group, the component with the largest absolute
weight counts in full, and every other present member counts at
`CORRELATED_DISCOUNT = 0.5`:

```rust
/// Weight of each member of a correlated group after its strongest.
pub const CORRELATED_DISCOUNT: f64 = 0.5;

/// The bits a correlated group adds, from its present members' bits.
#[must_use]
pub fn group_bits(member_bits: &[f64]) -> f64 {
    let Some((strongest, rest)) = strongest_first(member_bits) else {
        return 0.0;
    };
    strongest + CORRELATED_DISCOUNT * rest.iter().sum::<f64>()
}
```

`strongest_first` orders by absolute value, breaking ties by the larger
value, so the result does not depend on component order. A component in
no group stands alone. Disagreements are discounted the same way:
correlated fields disagreeing for one reason (a move) are one piece of
evidence against, not several.

A discount of `0.5` is a choice, not a measurement. It sits between
"fully independent" (`1.0`, the overcount) and "the same field twice"
(`0.0`, which would ignore real corroboration). Components that are
**derived** from another, such as a Soundex code from the name, are left
out of the sum entirely rather than discounted (§5).

## 7. Common and rare values

The table's `u` is an average over all values. Agreeing on "Smith" is
much weaker than that average, and agreeing on "Zbigniewska" much
stronger. The same holds for placeholder values: a birth date of
`1900-01-01` or 1 January is recorded for many people whose real date is
unknown, so agreeing on it proves almost nothing.

**The rule** (Winkler; Splink's term-frequency adjustment). A caller may
pass a `ValueFrequencies` source. Each crate declares
`FREQUENCY_COMPONENTS`, the components that adjust. For each of them, when
the two records hold the **same normalised value** (an exact agreement,
not a partial one), the agreement weight uses the value's own share of
records in place of `u`:

```rust
/// Where the share of records holding a value comes from.
pub trait ValueFrequencies {
    /// The share, in `0.0..=1.0`, of records whose `component` holds
    /// `value` (already in the matcher's normalised form), or `None`
    /// when unknown.
    fn share(&self, component: &str, value: &str) -> Option<f64>;
}

/// Smallest share used, so a value seen once cannot give unbounded bits.
pub const MIN_SHARE: f64 = 1e-9;

/// Agreement bits for a value held by `share` of records.
#[must_use]
pub fn frequency_bits(m: f64, share: f64) -> f64 {
    let share = if share.is_finite() { share.clamp(MIN_SHARE, m) } else { m };
    (m / share).log2()
}
```

- A share at or above `m` gives `0` bits: a value most records hold is no
  evidence, but agreeing on it is not evidence *against* either.
- Partial agreement and disagreement are not adjusted. A share says how
  often a value occurs, not how often two values are near each other.
- `NoFrequencies` returns `None` for everything, and a matcher's plain
  `match_*` call uses it, so a caller that passes nothing gets exactly the
  §2 table.
- **Keys.** The component names in `FREQUENCY_COMPONENTS` and the
  normalised value form are part of each crate's public contract. Each
  crate exports `frequency_key(component, raw) -> Option<String>`, which
  returns the key its matcher will look up, so a service can count values
  in the same form. A `FrequencyTable` built from counts is provided.
- **Where frequencies come from.** The service counts them, because only
  it sees the population: a share of `count / total` over its active
  records. A share from a small population is noisy, so a service should
  not supply one until the population is large enough to count from.

## 8. Explainability

`Evidence` lists every term of the sum, so a reviewer can see why a pair
reached its strength:

```rust
/// One term of the evidence sum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentBits {
    /// The component, e.g. `"family_name"`, `"identifier"`,
    /// `"contradiction:identifier:lei"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
}
```

`Evidence` gains `#[serde(default)] pub components: Vec<ComponentBits>`.
The terms always add up to `weight_bits`, and a property test in each
crate checks that.

## 9. What this still leaves out

- **Estimated `m`/`u`.** Confirmed and rejected review-queue decisions
  are the training data for estimating them (expectation–maximisation, as
  Splink does). Until a deployment estimates its own, the table's
  defaults stand.
- **Measured correlation.** The `0.5` discount and the group lists are
  judgement. The same review-queue data could measure them.
- **Identifier verification status.** A verified NHS number (a PDS trace)
  is stronger evidence than a typed one. The matcher models carry no
  verification flag today.

## 10. Rollout

1. This document.
2. All nine matcher crates: `src/evidence.rs`, the result and config
   fields, a spec edit, tests. `score`, `is_match` and `confidence` stay
   as they are.
3. All nine matchers: correlated groups (§6), value frequencies (§7) and
   the per-term listing (§8).
4. Services, each as its own task: carry `evidence` through their match
   responses and review-queue rows, gate auto-merge on `Proven`, and
   refuse it for `Consistent` and `Contradicted`. This is where
   event-service T-16 and worker-service T-23 close: their own scorers
   count a missing component as `0`, and they adopt the matcher's
   `score` plus `evidence` in its place.

## References

- I. P. Fellegi and A. B. Sunter, "A Theory for Record Linkage", *JASA*
  64 (1969): agreement weights, the missing-is-zero rule, and the
  three-band decision.
- W. E. Winkler, "String Comparator Metrics and Enhanced Decision Rules in
  the Fellegi–Sunter Model of Record Linkage" (1990): partial agreement
  for string similarity.
- Splink (UK Ministry of Justice), documentation on match weights, null
  comparison levels and term-frequency adjustment.
