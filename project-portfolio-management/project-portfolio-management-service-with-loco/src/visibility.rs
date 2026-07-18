//! Pure PPM Phase-B visibility rules (spec/15-roadmap PPM-6/7/8/9):
//! flexible-date parsing, dependency cycle detection, finish-start
//! violation checks, the critical path, RAG health, capacity
//! arithmetic, and CSV escaping — DB-free and unit-tested.

use std::collections::HashMap;

use chrono::NaiveDate;
use uuid::Uuid;

/// Parse the matcher's flexible date shape (`YYYY`, `YYYY-MM`, or
/// `YYYY-MM-DD`). `end_of_period` selects the last day of a partial
/// period (for `target_date`) vs the first (for `start_date`).
#[must_use]
pub fn parse_flex_date(value: &str, end_of_period: bool) -> Option<NaiveDate> {
    let value = value.trim();
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return Some(date);
    }
    if value.len() == 7 {
        let year: i32 = value.get(0..4)?.parse().ok()?;
        let month: u32 = value.get(5..7)?.parse().ok()?;
        let first = NaiveDate::from_ymd_opt(year, month, 1)?;
        return if end_of_period {
            Some(last_day_of_month(first))
        } else {
            Some(first)
        };
    }
    if value.len() == 4 {
        let year: i32 = value.parse().ok()?;
        return if end_of_period {
            NaiveDate::from_ymd_opt(year, 12, 31)
        } else {
            NaiveDate::from_ymd_opt(year, 1, 1)
        };
    }
    None
}

/// The last day of the month containing `date`.
fn last_day_of_month(date: NaiveDate) -> NaiveDate {
    let (year, month) = (chrono::Datelike::year(&date), chrono::Datelike::month(&date));
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    next.and_then(|d| d.pred_opt()).unwrap_or(date)
}

/// Whether adding the edge `predecessor → successor` to `edges` would
/// create a cycle (i.e. `predecessor` is already reachable **from**
/// `successor`). DFS over the existing edge list; also true for a
/// self-edge.
#[must_use]
pub fn would_create_cycle(edges: &[(Uuid, Uuid)], predecessor: Uuid, successor: Uuid) -> bool {
    if predecessor == successor {
        return true;
    }
    let mut adjacency: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (from, to) in edges {
        adjacency.entry(*from).or_default().push(*to);
    }
    let mut stack = vec![successor];
    let mut seen = std::collections::HashSet::new();
    while let Some(node) = stack.pop() {
        if node == predecessor {
            return true;
        }
        if seen.insert(node)
            && let Some(nexts) = adjacency.get(&node) {
                stack.extend(nexts.iter().copied());
            }
    }
    false
}

/// One work item's schedule facts.
#[derive(Debug, Clone, Copy)]
pub struct ScheduleItem {
    /// The item's pid.
    pub pid: Uuid,
    /// Parsed start (first day of the flexible period).
    pub start: Option<NaiveDate>,
    /// Parsed target (last day of the flexible period).
    pub end: Option<NaiveDate>,
}

/// A finish-start edge with lag.
#[derive(Debug, Clone, Copy)]
pub struct ScheduleEdge {
    /// The edge's pid.
    pub pid: Uuid,
    /// Must finish first.
    pub predecessor: Uuid,
    /// May start `lag_days` after the predecessor finishes.
    pub successor: Uuid,
    /// Working lag in days (may be 0).
    pub lag_days: i32,
}

/// A violated finish-start constraint: the successor starts before
/// `earliest_start` (predecessor end + lag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Violation {
    /// The offending edge.
    pub edge_pid: Uuid,
    /// The predecessor item.
    pub predecessor: Uuid,
    /// The successor item.
    pub successor: Uuid,
    /// Predecessor end + lag.
    pub earliest_start: NaiveDate,
    /// The successor's actual planned start.
    pub actual_start: NaiveDate,
}

/// Check every edge whose endpoints both carry the needed dates; an
/// edge with a missing date on either side cannot be judged and is
/// skipped (the schedule view lists undated items separately).
#[must_use]
pub fn violations(items: &[ScheduleItem], edges: &[ScheduleEdge]) -> Vec<Violation> {
    let by_pid: HashMap<Uuid, &ScheduleItem> = items.iter().map(|i| (i.pid, i)).collect();
    let mut out = Vec::new();
    for edge in edges {
        let (Some(pred), Some(succ)) = (by_pid.get(&edge.predecessor), by_pid.get(&edge.successor))
        else {
            continue;
        };
        let (Some(pred_end), Some(succ_start)) = (pred.end, succ.start) else {
            continue;
        };
        let earliest = pred_end + chrono::Days::new(u64::try_from(edge.lag_days.max(0)).unwrap_or(0));
        if succ_start < earliest {
            out.push(Violation {
                edge_pid: edge.pid,
                predecessor: edge.predecessor,
                successor: edge.successor,
                earliest_start: earliest,
                actual_start: succ_start,
            });
        }
    }
    out
}

/// The **critical path**: the chain of dependent items with the
/// largest total duration (days, inclusive; undated items count 0).
/// Edges are cycle-free by construction (insert-time check), so a
/// memoised longest-path walk terminates.
#[must_use]
pub fn critical_path(items: &[ScheduleItem], edges: &[ScheduleEdge]) -> Vec<Uuid> {
    // Memoised best (total, path) starting at each node.
    fn best(
        node: Uuid,
        duration: &HashMap<Uuid, i64>,
        successors: &HashMap<Uuid, Vec<Uuid>>,
        memo: &mut HashMap<Uuid, (i64, Vec<Uuid>)>,
    ) -> (i64, Vec<Uuid>) {
        if let Some(hit) = memo.get(&node) {
            return hit.clone();
        }
        let own = *duration.get(&node).unwrap_or(&0);
        let mut result = (own, vec![node]);
        if let Some(nexts) = successors.get(&node) {
            for next in nexts {
                let (tail_total, tail_path) = best(*next, duration, successors, memo);
                if own + tail_total > result.0 {
                    let mut path = vec![node];
                    path.extend(tail_path.iter().copied());
                    result = (own + tail_total, path);
                }
            }
        }
        memo.insert(node, result.clone());
        result
    }
    let duration: HashMap<Uuid, i64> = items
        .iter()
        .map(|i| {
            let days = match (i.start, i.end) {
                (Some(start), Some(end)) if end >= start => (end - start).num_days() + 1,
                _ => 0,
            };
            (i.pid, days)
        })
        .collect();
    let mut successors: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for edge in edges {
        successors.entry(edge.predecessor).or_default().push(edge.successor);
    }
    let mut memo = HashMap::new();
    let mut top: (i64, Vec<Uuid>) = (0, Vec::new());
    for item in items {
        let candidate = best(item.pid, &duration, &successors, &mut memo);
        if candidate.0 > top.0 {
            top = candidate;
        }
    }
    top.1
}

/// RAG health for one work item — a documented heuristic, not a
/// score: **red** when a risk has materialised, the target date has
/// passed on an unfinished item, or the budget is overrun; **amber**
/// when any open risk has exposure ≥ 15 or a schedule dependency is
/// violated; else **green**.
#[must_use]
pub fn rag(
    finished: bool,
    target: Option<NaiveDate>,
    today: NaiveDate,
    materialised_risks: usize,
    max_open_exposure: i32,
    budget_overrun: bool,
    has_violation: bool,
) -> &'static str {
    if materialised_risks > 0 || budget_overrun || (!finished && target.is_some_and(|t| t < today)) {
        "red"
    } else if max_open_exposure >= 15 || has_violation {
        "amber"
    } else {
        "green"
    }
}

/// A person's summed allocation percentage over a window: every
/// active allocation whose `[start, end]` overlaps `[from, to]`
/// counts in full (the conservative reading — concurrent commitments
/// add). Over 100 ⇒ over-allocated.
#[must_use]
pub fn summed_percent(
    allocations: &[(i32, Option<NaiveDate>, Option<NaiveDate>)],
    from: NaiveDate,
    to: NaiveDate,
) -> i64 {
    allocations
        .iter()
        .filter(|(_, start, end)| {
            start.is_none_or(|s| s <= to) && end.is_none_or(|e| e >= from)
        })
        .map(|(percent, _, _)| i64::from(*percent))
        .sum()
}

/// Escape one CSV field (RFC-4180 style: quote when needed, double
/// embedded quotes).
#[must_use]
pub fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn pid(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    /// Flexible dates: full, month (first/last), year (first/last),
    /// junk.
    #[test]
    fn flex_dates() {
        assert_eq!(parse_flex_date("2026-07-18", false), Some(date(2026, 7, 18)));
        assert_eq!(parse_flex_date("2026-02", false), Some(date(2026, 2, 1)));
        assert_eq!(parse_flex_date("2026-02", true), Some(date(2026, 2, 28)));
        assert_eq!(parse_flex_date("2024-02", true), Some(date(2024, 2, 29)));
        assert_eq!(parse_flex_date("2026", false), Some(date(2026, 1, 1)));
        assert_eq!(parse_flex_date("2026", true), Some(date(2026, 12, 31)));
        assert_eq!(parse_flex_date("someday", true), None);
        assert_eq!(parse_flex_date("2026-13", true), None);
    }

    /// Cycle detection: self-edge, direct back-edge, transitive
    /// back-edge; a legal forward edge is fine.
    #[test]
    fn cycles_are_detected() {
        let (a, b, c) = (pid(1), pid(2), pid(3));
        assert!(would_create_cycle(&[], a, a), "self-edge");
        let edges = vec![(a, b), (b, c)];
        assert!(would_create_cycle(&edges, c, a), "c→a closes a→b→c");
        assert!(!would_create_cycle(&edges, a, c), "a→c is a legal shortcut");
        assert!(would_create_cycle(&edges, b, a), "direct back-edge");
    }

    /// Violations: succ starting before pred end + lag flags; lag
    /// honoured; undated endpoints skip.
    #[test]
    fn finish_start_violations() {
        let items = vec![
            ScheduleItem { pid: pid(1), start: Some(date(2026, 1, 1)), end: Some(date(2026, 3, 31)) },
            ScheduleItem { pid: pid(2), start: Some(date(2026, 3, 1)), end: Some(date(2026, 6, 30)) },
            ScheduleItem { pid: pid(3), start: None, end: None },
        ];
        let edge = |p, s, lag| ScheduleEdge { pid: pid(9), predecessor: p, successor: s, lag_days: lag };
        // 2 starts 2026-03-01, before 1 ends 2026-03-31 ⇒ violation.
        let found = violations(&items, &[edge(pid(1), pid(2), 0)]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].earliest_start, date(2026, 3, 31));
        assert_eq!(found[0].actual_start, date(2026, 3, 1));
        // With the successor starting after end + lag ⇒ clean.
        let items_ok = vec![
            ScheduleItem { pid: pid(1), start: Some(date(2026, 1, 1)), end: Some(date(2026, 2, 28)) },
            ScheduleItem { pid: pid(2), start: Some(date(2026, 3, 10)), end: Some(date(2026, 6, 30)) },
        ];
        assert!(violations(&items_ok, &[edge(pid(1), pid(2), 5)]).is_empty());
        // Lag pushes earliest start past the actual start ⇒ violation.
        assert_eq!(violations(&items_ok, &[edge(pid(1), pid(2), 15)]).len(), 1);
        // Undated endpoint ⇒ skipped.
        assert!(violations(&items, &[edge(pid(1), pid(3), 0)]).is_empty());
    }

    /// The critical path picks the longest dependent chain.
    #[test]
    fn critical_path_is_longest_chain() {
        // a(31d) → b(30d) → d(10d)  vs  a(31d) → c(90d)
        let items = vec![
            ScheduleItem { pid: pid(1), start: Some(date(2026, 1, 1)), end: Some(date(2026, 1, 31)) },
            ScheduleItem { pid: pid(2), start: Some(date(2026, 2, 1)), end: Some(date(2026, 3, 2)) },
            ScheduleItem { pid: pid(3), start: Some(date(2026, 2, 1)), end: Some(date(2026, 5, 1)) },
            ScheduleItem { pid: pid(4), start: Some(date(2026, 3, 10)), end: Some(date(2026, 3, 19)) },
        ];
        let edge = |n, p, s| ScheduleEdge { pid: pid(n), predecessor: p, successor: s, lag_days: 0 };
        let path = critical_path(
            &items,
            &[edge(9, pid(1), pid(2)), edge(10, pid(2), pid(4)), edge(11, pid(1), pid(3))],
        );
        assert_eq!(path, vec![pid(1), pid(3)], "a→c (121d) beats a→b→d (71d)");
    }

    /// RAG: red beats amber beats green; finished items don't go red
    /// on a past target.
    #[test]
    fn rag_heuristic() {
        let today = date(2026, 7, 18);
        assert_eq!(rag(false, Some(date(2026, 8, 1)), today, 0, 4, false, false), "green");
        assert_eq!(rag(false, Some(date(2026, 8, 1)), today, 0, 16, false, false), "amber");
        assert_eq!(rag(false, None, today, 0, 0, false, true), "amber");
        assert_eq!(rag(false, Some(date(2026, 7, 1)), today, 0, 0, false, false), "red");
        assert_eq!(rag(true, Some(date(2026, 7, 1)), today, 0, 0, false, false), "green");
        assert_eq!(rag(false, None, today, 1, 0, false, false), "red");
        assert_eq!(rag(false, None, today, 0, 0, true, false), "red");
    }

    /// Capacity: overlapping allocations sum; disjoint ones drop out;
    /// open-ended ones always count.
    #[test]
    fn capacity_sums_overlaps() {
        let window = (date(2026, 7, 1), date(2026, 7, 31));
        let allocations = vec![
            (60, Some(date(2026, 6, 1)), Some(date(2026, 8, 31))), // overlaps
            (50, None, None),                                      // open-ended
            (40, Some(date(2026, 9, 1)), None),                    // starts after
            (30, None, Some(date(2026, 6, 30))),                   // ended before
        ];
        assert_eq!(summed_percent(&allocations, window.0, window.1), 110);
    }

    /// CSV escaping: plain passes, commas/quotes/newlines quote.
    #[test]
    fn csv_escaping() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("line\nbreak"), "\"line\nbreak\"");
    }
}
