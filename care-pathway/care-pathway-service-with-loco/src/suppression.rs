//! **Disclosure control: modes and marginals** (spec `13-tasks.md`
//! T-14k) — generalising the small-cohort floor `tba::cohort_time_analysis`
//! already applied ([`agents/share/time-based-analysis.md`][tba-10]'s
//! TBA-10) into one shared, deployment-configurable primitive every
//! aggregate can use.
//!
//! [tba-10]: ../../agents/share/time-based-analysis.md
//!
//! Two independent capabilities live here:
//!
//! 1. **[`is_suppressed`] / [`min_cell_count`]** — the scalar case: hide
//!    detail (not the count itself) when a cohort's `n` is below the
//!    floor. This is what `cohort_time_analysis` and `cohort_constraints`
//!    use — see `spec/time-based-analysis.md` §12.2, "a bulk or
//!    aggregate read must never reveal more than the equivalent single
//!    read."
//! 2. **[`Table`] / [`decide`] / [`render`]** — the stratified case a
//!    2-D breakdown needs (T-14f's rule-based cohort splits, not yet
//!    built): a cell below the floor is a **primary** suppression, but
//!    a primary suppression alone can leak by arithmetic — `total −
//!    Σ(visible siblings)` recovers a lone withheld cell exactly. This
//!    module adds **secondary suppression**: whenever a partition
//!    (a row, a column, or any other declared group whose members sum
//!    to a published margin) ends up with exactly one suppressed cell,
//!    one more cell in that partition is suppressed too, so the
//!    arithmetic has at least two unknowns and one equation — never
//!    solvable. `decide` computes *which* cells to suppress once;
//!    [`Mode::Withhold`] and [`Mode::Remove`] only change how a
//!    suppressed cell is *rendered*, so the two modes can never disagree
//!    on which cells are small (T-14k's own acceptance criterion).
//!
//! Two of TreatmentPatterns' three modes are deliberately **not**
//! adopted: `mean` substitutes a made-up count (dishonest), and its
//! `minCellCount` mode reports the suppressed cell *as* the threshold,
//! which reads as a real count rather than a redaction.
//!
//! **T-14a's export codecs are exempt, not suppressed.** They render
//! patient-level rows, not aggregates, and the family answer for
//! patient-level data is access control + audit
//! (`agents/share/bulk-import-export.md` §8), never suppression —
//! suppressing a row would just delete the record the export exists to
//! carry. `tests/requests/exports.rs`'s round trip already covers a
//! tiny (`n = 1`) cohort with no suppression applied; see that file
//! rather than duplicating the proof here.

use std::collections::BTreeSet;

/// The floor below the family's own `spec/time-based-analysis.md` §17
/// lean ("fixed at 5, configurable upward only"). A deployment may
/// raise it; [`min_cell_count`] refuses to honour a lower value.
pub const DEFAULT_MIN_CELL_COUNT: usize = 5;

/// Env var naming a deployment's raised floor. Unset, blank, or an
/// unparseable/lower value all fall back to [`DEFAULT_MIN_CELL_COUNT`]
/// — silently *weakening* disclosure control from a bad env var would
/// be the wrong failure mode, so this direction fails closed.
pub const MIN_CELL_COUNT_ENV: &str = "CARE_PATHWAY_MIN_CELL_COUNT";

/// The deployment's cell-count floor: [`DEFAULT_MIN_CELL_COUNT`], or
/// [`MIN_CELL_COUNT_ENV`] when it parses as an integer at least that
/// large. Read once and cached, matching the family's other
/// once-at-boot env switches (e.g. `disclosure::fail_closed`).
#[must_use]
pub fn min_cell_count() -> usize {
    static CACHE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| {
        std::env::var(MIN_CELL_COUNT_ENV)
            .ok()
            .and_then(|raw| raw.trim().parse::<usize>().ok())
            .map_or(DEFAULT_MIN_CELL_COUNT, |configured| {
                configured.max(DEFAULT_MIN_CELL_COUNT)
            })
    })
}

/// The reason string every withheld scalar or cell carries — a closed,
/// greppable vocabulary of one, so a UI need not invent its own copy.
pub const SUPPRESSED_REASON: &str = "withheld: fewer than the minimum cell count";

/// Whether a cohort of this size should have its detail withheld,
/// against [`min_cell_count`]. `n == 0` is never suppressed — an empty
/// cohort discloses nothing to hide, and treating it as suppressed
/// would make "no data" indistinguishable from "one patient's data,
/// hidden" (the opposite of what suppression exists to do — see
/// `render`'s `Mode::Withhold` for the same reasoning on a cell).
#[must_use]
pub fn is_suppressed(n: usize) -> bool {
    n > 0 && n < min_cell_count()
}

/// How a suppressed value renders. The *decision* of which cells are
/// small ([`decide`]) never depends on this — only the rendering does,
/// which is what makes the two modes unable to disagree on which cells
/// are small.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Keep the row/cell, `null` the value, carry [`SUPPRESSED_REASON`]
    /// — the default. A caller sees exactly which strata exist and
    /// which are withheld, never a silently-missing row.
    Withhold,
    /// Drop the row/cell entirely, as `TreatmentPatterns`' `remove` mode
    /// does — for a caller that wants a table with no withheld rows at
    /// all, at the cost of no longer seeing that a stratum existed.
    Remove,
}

impl Mode {
    /// Parse a `?mode=` query value. Anything other than `"remove"`
    /// (case-insensitively) — including absent, blank, or a typo — is
    /// [`Mode::Withhold`], the safer default: an unrecognised mode
    /// should never silently drop rows the caller did not ask to drop.
    #[must_use]
    pub fn parse(raw: Option<&str>) -> Self {
        if raw.map(str::to_ascii_lowercase).as_deref() == Some("remove") {
            Self::Remove
        } else {
            Self::Withhold
        }
    }
}

/// One cell of a stratified count table, at an arbitrary coordinate
/// (e.g. `["urgency:routine", "outcome:improved"]` for a 2-D
/// cross-tabulation, or a single-element coordinate for a plain
/// partition).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// This cell's stratum coordinates.
    pub coords: Vec<String>,
    /// The true count. Never published directly for a suppressed cell.
    pub count: usize,
}

/// A named group of cell indices whose **visible** counts sum to a
/// published margin (a row total, a column total, the grand total —
/// whatever the caller declares) — the thing a withheld cell could
/// otherwise be recovered from by subtraction. A table with no
/// declared margins at all needs no secondary suppression, because
/// there is nothing to difference against; declaring every margin the
/// rendered output actually publishes is the caller's responsibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    /// A label for diagnostics; not otherwise load-bearing.
    pub label: String,
    /// Indices into the owning [`Table`]'s `cells`.
    pub cell_indices: Vec<usize>,
}

/// A stratified count table: cells plus every partition whose margin
/// is published (so [`decide`] knows what a withheld cell could be
/// recovered against).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Table {
    /// Every cell.
    pub cells: Vec<Cell>,
    /// Every group whose members' visible sum is a published margin.
    pub partitions: Vec<Partition>,
}

/// Decide which cell indices to suppress: every cell below
/// [`min_cell_count`] (primary), plus — for any partition left with
/// **exactly one** suppressed cell — one more cell from that partition
/// (secondary), repeated to a fixed point, since a secondary
/// suppression can itself leave a different partition at exactly one.
///
/// The secondary pick is the smallest remaining visible cell in the
/// partition: minimising how much additional detail is withheld,
/// deterministically (no two runs over the same table disagree).
///
/// A partition with only one cell (no siblings to hide behind) is left
/// as decided by the primary pass — there is nothing this algorithm
/// can add that would help, and a singleton partition's margin already
/// equals that one cell's value wherever the margin itself is
/// published, which is a modelling question for the caller, not this
/// function.
#[must_use]
pub fn decide(table: &Table, min_cell_count: usize) -> BTreeSet<usize> {
    let mut suppressed: BTreeSet<usize> = table
        .cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| cell.count < min_cell_count)
        .map(|(index, _)| index)
        .collect();

    // Bounded by the number of cells: each round either adds at least
    // one new suppression or terminates, and there are only so many
    // cells to add.
    for _ in 0..=table.cells.len() {
        let mut changed = false;
        for partition in &table.partitions {
            let suppressed_here: Vec<usize> = partition
                .cell_indices
                .iter()
                .copied()
                .filter(|index| suppressed.contains(index))
                .collect();
            if suppressed_here.len() == 1 {
                let visible_smallest = partition
                    .cell_indices
                    .iter()
                    .copied()
                    .filter(|index| !suppressed.contains(index))
                    .min_by_key(|index| table.cells[*index].count);
                if let Some(pick) = visible_smallest {
                    suppressed.insert(pick);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    suppressed
}

/// One rendered cell — `value` is `None` under [`Mode::Withhold`] for a
/// suppressed cell; a suppressed cell never appears at all under
/// [`Mode::Remove`] (filtered out of the returned `Vec` entirely).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedCell {
    /// The cell's stratum coordinates.
    pub coords: Vec<String>,
    /// `Some(count)` when visible; `None` when withheld.
    pub value: Option<usize>,
    /// [`SUPPRESSED_REASON`] on a withheld cell; `None` otherwise.
    pub reason: Option<&'static str>,
}

/// Render a table under one [`Mode`], having already decided (via
/// [`decide`]) which cells are small. Splitting the decision from the
/// rendering is what makes `Mode::Withhold` and `Mode::Remove` unable
/// to disagree on *which* cells are small — they share one decision.
#[must_use]
pub fn render(table: &Table, min_cell_count: usize, mode: Mode) -> Vec<RenderedCell> {
    let suppressed = decide(table, min_cell_count);
    table
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            if suppressed.contains(&index) {
                match mode {
                    Mode::Withhold => Some(RenderedCell {
                        coords: cell.coords.clone(),
                        value: None,
                        reason: Some(SUPPRESSED_REASON),
                    }),
                    Mode::Remove => None,
                }
            } else {
                Some(RenderedCell {
                    coords: cell.coords.clone(),
                    value: Some(cell.count),
                    reason: None,
                })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(coord: &str, count: usize) -> Cell {
        Cell {
            coords: vec![coord.to_string()],
            count,
        }
    }

    fn one_row_table(counts: &[usize]) -> Table {
        let cells: Vec<Cell> = counts
            .iter()
            .enumerate()
            .map(|(i, &count)| cell(&format!("cell{i}"), count))
            .collect();
        let cell_indices = (0..cells.len()).collect();
        Table {
            cells,
            partitions: vec![Partition {
                label: "row".to_string(),
                cell_indices,
            }],
        }
    }

    /// No cell below the floor: nothing suppressed.
    #[test]
    fn a_clean_partition_suppresses_nothing() {
        let table = one_row_table(&[10, 20, 30]);
        assert!(decide(&table, 5).is_empty());
    }

    /// Exactly one cell below the floor gets a secondary partner, so
    /// the lone withheld cell is never recoverable as `total −
    /// Σ(visible)`.
    #[test]
    fn a_lone_small_cell_gets_a_secondary_suppression() {
        let table = one_row_table(&[2, 30, 40]);
        let suppressed = decide(&table, 5);
        assert!(suppressed.contains(&0), "the small cell itself");
        assert_eq!(
            suppressed.len(),
            2,
            "a lone suppression must recruit exactly one partner: {suppressed:?}"
        );
        // The partner is the smallest remaining visible cell.
        assert!(suppressed.contains(&1), "30 is smaller than 40");
    }

    /// Two cells already below the floor: already safe, no secondary
    /// suppression needed (two unknowns, one equation).
    #[test]
    fn two_small_cells_need_no_secondary_suppression() {
        let table = one_row_table(&[2, 3, 40]);
        let suppressed = decide(&table, 5);
        assert_eq!(suppressed, BTreeSet::from([0, 1]));
    }

    /// A 2-D table: suppressing one cell can leave both its row and its
    /// column at exactly one suppressed cell, needing the fixed-point
    /// loop to resolve both.
    #[test]
    fn secondary_suppression_propagates_across_row_and_column_partitions() {
        // A 2x2 table:
        //        col0  col1
        // row0:   2     30
        // row1:  40     50
        let cells = vec![
            cell("row0,col0", 2),
            cell("row0,col1", 30),
            cell("row1,col0", 40),
            cell("row1,col1", 50),
        ];
        let table = Table {
            cells,
            partitions: vec![
                Partition {
                    label: "row0".to_string(),
                    cell_indices: vec![0, 1],
                },
                Partition {
                    label: "row1".to_string(),
                    cell_indices: vec![2, 3],
                },
                Partition {
                    label: "col0".to_string(),
                    cell_indices: vec![0, 2],
                },
                Partition {
                    label: "col1".to_string(),
                    cell_indices: vec![1, 3],
                },
            ],
        };
        let suppressed = decide(&table, 5);
        // Cell 0 (the lone small one) is suppressed; its row (row0)
        // recruits cell 1, and its column (col0) recruits cell 2 —
        // every partition ends with 0 or >=2 suppressed cells.
        assert!(suppressed.contains(&0));
        for partition in &table.partitions {
            let count = partition
                .cell_indices
                .iter()
                .filter(|i| suppressed.contains(i))
                .count();
            assert_ne!(count, 1, "partition {} left recoverable", partition.label);
        }
    }

    /// `Mode::Withhold` and `Mode::Remove` never disagree on *which*
    /// cells are small — only on how a small one renders.
    #[test]
    fn withhold_and_remove_agree_on_which_cells_are_small() {
        let table = one_row_table(&[2, 30, 40]);
        let withheld = render(&table, 5, Mode::Withhold);
        let removed = render(&table, 5, Mode::Remove);
        let withheld_coords: BTreeSet<&str> = withheld
            .iter()
            .filter(|c| c.value.is_none())
            .map(|c| c.coords[0].as_str())
            .collect();
        let removed_coords: BTreeSet<&str> =
            table.cells.iter().map(|c| c.coords[0].as_str()).collect();
        let kept_coords: BTreeSet<&str> = removed.iter().map(|c| c.coords[0].as_str()).collect();
        let removed_missing: BTreeSet<&str> =
            removed_coords.difference(&kept_coords).copied().collect();
        assert_eq!(
            withheld_coords, removed_missing,
            "the two modes must suppress the identical cell set"
        );
    }

    /// A suppressed cell under `Withhold` carries the reason and no
    /// value; a visible cell carries a value and no reason.
    #[test]
    fn withheld_cells_carry_a_reason_never_a_value() {
        let table = one_row_table(&[2, 30, 40]);
        for rendered in render(&table, 5, Mode::Withhold) {
            if rendered.value.is_none() {
                assert_eq!(rendered.reason, Some(SUPPRESSED_REASON));
            } else {
                assert_eq!(rendered.reason, None);
            }
        }
    }

    /// A partition with only one cell is left as the primary pass
    /// decided — there is no sibling to recruit.
    #[test]
    fn a_singleton_partition_has_no_secondary_partner_to_recruit() {
        let table = Table {
            cells: vec![cell("only", 2)],
            partitions: vec![Partition {
                label: "solo".to_string(),
                cell_indices: vec![0],
            }],
        };
        assert_eq!(decide(&table, 5), BTreeSet::from([0]));
    }

    /// Property test: over many small, randomly generated tables (a
    /// deterministic, dependency-free generator — the same choice
    /// `data::journeys` makes, and for the same reason), no partition
    /// with any declared margin ever ends with exactly one suppressed
    /// cell. That is precisely the condition under which a withheld
    /// cell is recoverable as `margin − Σ(visible siblings)`, so
    /// "never exactly one" is a direct proof of the acceptance
    /// criterion, not merely a proxy for it.
    #[test]
    fn no_partition_is_ever_left_with_exactly_one_suppressed_cell() {
        // A tiny SplitMix64-style generator — see `data::journeys`'s
        // `Rng` for the identical rationale not to pull in `rand` here.
        struct Rng(u64);
        impl Rng {
            fn next_u64(&mut self) -> u64 {
                self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
                let mut z = self.0;
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                z ^ (z >> 31)
            }
            #[allow(clippy::cast_possible_truncation)] // reduced mod (hi - lo + 1) first; fixture generation, not a security boundary
            fn range(&mut self, lo: usize, hi: usize) -> usize {
                let span = u64::try_from(hi - lo + 1).unwrap_or(1);
                lo + (self.next_u64() % span) as usize
            }
        }

        for seed in 0..500u64 {
            let mut rng = Rng(seed);
            let rows = rng.range(2, 5);
            let cols = rng.range(2, 5);
            let mut cells = Vec::with_capacity(rows * cols);
            for r in 0..rows {
                for c in 0..cols {
                    cells.push(Cell {
                        coords: vec![format!("r{r}"), format!("c{c}")],
                        count: rng.range(0, 12),
                    });
                }
            }
            let mut partitions = Vec::new();
            for r in 0..rows {
                partitions.push(Partition {
                    label: format!("row{r}"),
                    cell_indices: (0..cols).map(|c| r * cols + c).collect(),
                });
            }
            for c in 0..cols {
                partitions.push(Partition {
                    label: format!("col{c}"),
                    cell_indices: (0..rows).map(|r| r * cols + c).collect(),
                });
            }
            let table = Table { cells, partitions };
            let suppressed = decide(&table, 5);
            for partition in &table.partitions {
                let count = partition
                    .cell_indices
                    .iter()
                    .filter(|i| suppressed.contains(i))
                    .count();
                assert_ne!(
                    count, 1,
                    "seed {seed}: partition {} left with exactly one suppressed cell",
                    partition.label
                );
            }
        }
    }

    /// The floor is fixed at 5 and only raisable — a lower or garbage
    /// env value never weakens it (unit-tested against the parse rule
    /// directly, since the getter itself caches process-wide).
    #[test]
    fn the_floor_only_ever_goes_up() {
        let parse = |raw: &str| -> usize {
            raw.trim()
                .parse::<usize>()
                .ok()
                .map_or(DEFAULT_MIN_CELL_COUNT, |v| v.max(DEFAULT_MIN_CELL_COUNT))
        };
        assert_eq!(
            parse("3"),
            DEFAULT_MIN_CELL_COUNT,
            "a lower value is clamped up"
        );
        assert_eq!(parse("10"), 10, "a higher value is honoured");
        assert_eq!(parse("garbage"), DEFAULT_MIN_CELL_COUNT);
        assert_eq!(parse(""), DEFAULT_MIN_CELL_COUNT);
    }

    /// `n == 0` is never suppressed — an empty cohort has nothing to
    /// hide, and "no data" must stay distinguishable from "hidden
    /// data".
    #[test]
    fn an_empty_cohort_is_never_suppressed() {
        assert!(!is_suppressed(0));
        assert!(is_suppressed(1));
        assert!(is_suppressed(4));
        assert!(!is_suppressed(5));
        assert!(!is_suppressed(100));
    }

    /// `?mode=` parses `remove` case-insensitively; anything else,
    /// including absent, is the safer `Withhold` default.
    #[test]
    fn mode_parses_remove_case_insensitively_else_withhold() {
        assert_eq!(Mode::parse(Some("remove")), Mode::Remove);
        assert_eq!(Mode::parse(Some("REMOVE")), Mode::Remove);
        assert_eq!(Mode::parse(Some("withhold")), Mode::Withhold);
        assert_eq!(Mode::parse(Some("typo")), Mode::Withhold);
        assert_eq!(Mode::parse(None), Mode::Withhold);
    }
}
