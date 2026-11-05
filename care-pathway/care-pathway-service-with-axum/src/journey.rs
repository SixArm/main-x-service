//! **Stitched journeys** — following a `continues_as` chain across
//! service boundaries so time-based analysis measures the whole journey
//! rather than the leg that happens to live here.
//!
//! The link that makes this possible is the write side in
//! [`crate::controllers::links`]; this module is the read side.
//!
//! # The three design questions, answered
//!
//! **Who fetches?** This service, server-side. The aggregator was the
//! obvious alternative and is the wrong one: it is a *link graph*, it
//! serves neighbours rather than segment detail, and giving it a
//! timeline read-model would duplicate every owning service's data to
//! answer a question those services can answer themselves. Making the
//! browser fetch each leg is worse still — it would need a credential
//! for every service, which is exactly what the BFF pattern exists to
//! avoid.
//!
//! **Under whose authorisation?** The **caller's**. Their bearer is
//! forwarded to the far service, which applies its own policy to the
//! real caller. The alternative — a service identity — would make this
//! a **confused deputy**: a caller entitled to read the pathway journey
//! but not the inpatient stay would receive the stay's timeline anyway,
//! because the far service would see only a trusted peer asking. The
//! family's tokens carry a family-wide `aud`, so forwarding works
//! without minting anything. When the caller has no bearer (enforcement
//! off) none is sent, which preserves the default-off posture rather
//! than silently escalating.
//!
//! **What happens when a leg cannot be read?** It is reported, and the
//! **combined figures are withheld**. A stitched lead time that is
//! missing a leg is not an imprecise number, it is a wrong one — it
//! understates the journey by exactly the part nobody could see. So
//! every leg carries a status, and the totals are `null` with a stated
//! reason unless every leg resolved. Each resolved leg's own figures are
//! still returned, so a partial answer is useful without being
//! misleading.
//!
//! # Safety
//!
//! Only operator-configured base URLs are contacted
//! (`CARE_PATHWAY_JOURNEY_URL_<TYPE>`) — never a host derived from a
//! record. The client does not follow redirects, so a peer cannot steer
//! it at an internal address (SEC-B11). Requests are timeout-bounded and
//! the chain walk is depth- and cycle-bounded.

use std::collections::BTreeSet;
use std::time::Duration;

use entity_ref::{EntityRef, EntityType};
use serde::Serialize;
use uuid::Uuid;

/// How far a `continues_as` chain is followed. A journey with more legs
/// than this is a data problem, not a patient history.
pub const MAX_LEGS: usize = 16;

/// Per-leg request timeout. A far service that is slow must not hold the
/// caller's connection open indefinitely.
pub const LEG_TIMEOUT: Duration = Duration::from_secs(5);

/// Why a leg has no timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LegStatus {
    /// Resolved: this leg's timeline is present.
    Resolved,
    /// No `CARE_PATHWAY_JOURNEY_URL_<TYPE>` is configured for the far
    /// service, so it was never contacted. Not an error — a deployment
    /// that has not wired a peer says so rather than failing.
    NotConfigured,
    /// The far service refused, or reported nothing there. Deliberately
    /// **one** status: the far service folds a denial into a `404` for
    /// the same reason this one does, and distinguishing them here would
    /// re-open the leak from the other side.
    UnavailableOrDenied,
    /// The far service could not be reached, timed out, redirected, or
    /// answered with something unusable.
    Unreachable,
    /// The chain was longer than [`MAX_LEGS`], or revisited a leg.
    Truncated,
}

impl LegStatus {
    /// Whether this leg contributed a timeline.
    #[must_use]
    pub const fn is_resolved(self) -> bool {
        matches!(self, LegStatus::Resolved)
    }

    /// A short human explanation, carried in the response so a partial
    /// answer explains itself.
    #[must_use]
    pub const fn detail(self) -> &'static str {
        match self {
            LegStatus::Resolved => "timeline read",
            LegStatus::NotConfigured => {
                "no journey URL configured for this service; the link is known, \
                 its timeline was not requested"
            }
            LegStatus::UnavailableOrDenied => {
                "the far service reported nothing there, or declined to say — \
                 these are deliberately indistinguishable"
            }
            LegStatus::Unreachable => "the far service could not be reached",
            LegStatus::Truncated => "the chain was cut short by the leg limit or a cycle",
        }
    }
}

/// One leg of a stitched journey.
#[derive(Debug, Clone, Serialize)]
pub struct Leg {
    /// The leg's `EntityRef` URN.
    pub entity_ref: String,
    /// Hops from the starting instance; the start itself is `0`.
    pub hop: usize,
    /// Whether the timeline was read, and why not when it was not.
    pub status: LegStatus,
    /// What the status means.
    pub detail: &'static str,
    /// The leg's own elapsed span, when resolved.
    pub lead_time_ms: Option<i64>,
    /// Value-adding time on this leg, when resolved.
    pub value_time_ms: Option<i64>,
    /// Clock bounds, so the stitched span can be computed without
    /// assuming the legs are contiguous — journeys have gaps *between*
    /// episodes too, and those are usually the interesting ones.
    pub clock_start_ms: Option<i64>,
    /// The far end of this leg's clock.
    pub clock_stop_ms: Option<i64>,
}

/// The URL **template** for fetching a far service's timeline, or `None`
/// when the deployment has not configured that peer.
///
/// One variable per **entity type**, not per service, so a deployment
/// grants exactly the reach it intends — and a template rather than a
/// bare host, so the only URL ever contacted is one an operator wrote.
/// Nothing is derived from the record.
///
/// ```text
/// CARE_PATHWAY_JOURNEY_URL_PATIENT_FLOW_STAY=\
///   http://patient-flow:5150/api/stays/{id}/time-analysis
/// ```
#[must_use]
pub fn url_template_for(entity_type: EntityType) -> Option<String> {
    let name = format!(
        "CARE_PATHWAY_JOURNEY_URL_{}",
        entity_type.as_str().to_ascii_uppercase()
    );
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Substitute a ref's id into a configured template.
///
/// Only `{id}` is substituted, and only with a UUID that already parsed
/// — so nothing caller-controlled reaches the URL. A template without
/// `{id}` is used as-is, which lets a peer expose a query-style
/// endpoint without a special case here.
#[must_use]
pub fn leg_url(template: &str, id: Uuid) -> String {
    template.replace("{id}", &id.to_string())
}

/// The **timeline contract** a peer must satisfy to take part in a
/// stitched journey.
///
/// Deliberately tiny — four numbers — because a journey does not need
/// the far service's segment detail, only its span and how much of that
/// span was the work. Anything richer would couple this service to
/// another's domain model.
///
/// Fields are read from the response body at either the top level or
/// under `analysis`, so this service's own
/// `/api/instances/{pid}/time-analysis` satisfies the contract
/// unmodified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegTimeline {
    /// Start of the leg's clock, epoch milliseconds.
    pub clock_start_ms: i64,
    /// End of it.
    pub clock_stop_ms: i64,
    /// Elapsed span.
    pub lead_time_ms: i64,
    /// Value-adding time within the span.
    pub value_time_ms: i64,
}

/// Read a [`LegTimeline`] out of a peer response, tolerating either the
/// top level or an `analysis` wrapper.
///
/// Returns `None` rather than defaulting a missing number to zero: a
/// leg whose value time we invented would flow straight into the
/// stitched ratio, and a fabricated zero is worse than a reported
/// failure.
#[must_use]
pub fn parse_leg(body: &serde_json::Value) -> Option<LegTimeline> {
    let scope = body.get("analysis").unwrap_or(body);
    let clock = scope.get("clock").unwrap_or(scope);
    let num = |v: &serde_json::Value, key: &str| v.get(key).and_then(serde_json::Value::as_i64);
    Some(LegTimeline {
        clock_start_ms: num(clock, "start_ms")?,
        clock_stop_ms: num(clock, "stop_ms")?,
        lead_time_ms: num(scope, "lead_time_ms")?,
        value_time_ms: num(scope, "value_time_ms")?,
    })
}

/// The shared, **non-redirecting** HTTP client for leg fetches.
///
/// Disabling redirects closes the SSRF-via-redirect vector: the only
/// host contacted is the one in the operator-configured template, and a
/// peer returning a `3xx` to an internal address (cloud metadata,
/// another service) can no longer make this service follow it.
///
/// # Panics
///
/// Never in practice: the builder is given a static redirect policy and
/// a static timeout, neither of which can fail. A panic here would mean
/// the TLS backend failed to initialise at process start, which is not
/// a condition this service can serve through.
pub fn leg_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(LEG_TIMEOUT)
            .build()
            .expect("build the non-redirecting journey-leg client")
    })
}

/// Classify a peer response status into a leg status.
///
/// `404` and `403` collapse to one outcome on purpose: the far service
/// folds a denial into a not-found for the same reason this one does,
/// and separating them here would re-open that leak from this side. A
/// `3xx` is `Unreachable` — the client does not follow redirects, so a
/// redirect is a peer trying to send us somewhere we will not go.
#[must_use]
pub fn classify_status(status: u16) -> LegStatus {
    match status {
        200..=299 => LegStatus::Resolved,
        403 | 404 | 410 => LegStatus::UnavailableOrDenied,
        _ => LegStatus::Unreachable,
    }
}

/// The stitched journey's combined figures, or the reason there are
/// none.
#[derive(Debug, Clone, Serialize)]
pub struct StitchedTotals {
    /// Earliest clock start to latest clock stop across every leg.
    /// `None` unless **every** leg resolved.
    pub lead_time_ms: Option<i64>,
    /// The same in days.
    pub lead_time_days: Option<f64>,
    /// Summed value-adding time across the legs.
    pub value_time_ms: Option<i64>,
    /// Value time over the stitched span.
    pub value_adding_ratio: Option<f64>,
    /// Why the totals are absent.
    pub reason: Option<String>,
    /// Legs that contributed.
    pub legs_resolved: usize,
    /// Legs that did not.
    pub legs_unresolved: usize,
}

/// Combine resolved legs into the stitched figures.
///
/// **Withholds every total unless all legs resolved.** A stitched lead
/// time missing a leg understates the journey by exactly the part nobody
/// could see; publishing it as though complete would be the same class
/// of quiet lie that the coverage figure exists to prevent elsewhere.
///
/// The span is `earliest start → latest stop`, not the sum of the legs:
/// the gap *between* two episodes is real waiting and is usually the
/// most interesting part of a cross-service journey. Summing the legs
/// would delete it.
#[must_use]
pub fn stitch(legs: &[Leg]) -> StitchedTotals {
    let resolved = legs.iter().filter(|l| l.status.is_resolved()).count();
    let unresolved = legs.len().saturating_sub(resolved);

    if legs.is_empty() {
        return StitchedTotals {
            lead_time_ms: None,
            lead_time_days: None,
            value_time_ms: None,
            value_adding_ratio: None,
            reason: Some("no legs".to_string()),
            legs_resolved: 0,
            legs_unresolved: 0,
        };
    }
    if unresolved > 0 {
        return StitchedTotals {
            lead_time_ms: None,
            lead_time_days: None,
            value_time_ms: None,
            value_adding_ratio: None,
            reason: Some(format!(
                "{unresolved} of {} legs could not be read, so a stitched total \
                 would understate the journey by exactly the part nobody could \
                 see. The resolved legs are reported individually.",
                legs.len()
            )),
            legs_resolved: resolved,
            legs_unresolved: unresolved,
        };
    }

    let start = legs.iter().filter_map(|l| l.clock_start_ms).min();
    let stop = legs.iter().filter_map(|l| l.clock_stop_ms).max();
    let (Some(start), Some(stop)) = (start, stop) else {
        return StitchedTotals {
            lead_time_ms: None,
            lead_time_days: None,
            value_time_ms: None,
            value_adding_ratio: None,
            reason: Some("a leg reported no clock bounds".to_string()),
            legs_resolved: resolved,
            legs_unresolved: unresolved,
        };
    };
    let lead = stop.saturating_sub(start).max(0);
    let value = legs.iter().fold(0i64, |acc, l| {
        acc.saturating_add(l.value_time_ms.unwrap_or(0))
    });

    StitchedTotals {
        lead_time_ms: Some(lead),
        lead_time_days: Some(crate::tba::as_days(lead)),
        value_time_ms: Some(value),
        value_adding_ratio: crate::tba::Ratio::new(value, lead).value,
        reason: None,
        legs_resolved: resolved,
        legs_unresolved: unresolved,
    }
}

/// Decide the next hop and guard the walk.
///
/// Returns `Some(next)` when the chain continues, or `None` when it ends
/// — including when a cap fires, which the caller reports as a
/// [`LegStatus::Truncated`] leg rather than silently stopping.
#[must_use]
pub fn next_hop(
    seen: &mut BTreeSet<String>,
    candidate: &EntityRef,
    hop: usize,
) -> Option<EntityRef> {
    if hop >= MAX_LEGS {
        return None;
    }
    if !seen.insert(candidate.to_string()) {
        // A cycle. The write path refuses a self-link, but a longer
        // ring (A → B → A) is not visible from either end at write
        // time, so the read must terminate on its own.
        return None;
    }
    Some(*candidate)
}

/// This service's own URN for an instance.
#[must_use]
pub fn instance_ref(pid: Uuid) -> String {
    format!("{}:{pid}", EntityType::CarePathwayInstance.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leg(hop: usize, status: LegStatus, start: Option<i64>, stop: Option<i64>) -> Leg {
        Leg {
            entity_ref: format!("care_pathway_instance:{}", Uuid::nil()),
            hop,
            status,
            detail: status.detail(),
            lead_time_ms: stop.zip(start).map(|(b, a)| b - a),
            value_time_ms: Some(1_000),
            clock_start_ms: start,
            clock_stop_ms: stop,
        }
    }

    const DAY: i64 = 86_400_000;

    #[test]
    fn the_stitched_span_runs_end_to_end_not_leg_by_leg() {
        // Two legs with a 10-day gap between them. The stitched span is
        // 30 days, not the 20 the legs sum to — the gap between
        // episodes is real waiting, and it is usually the interesting
        // part of a cross-service journey.
        let legs = vec![
            leg(0, LegStatus::Resolved, Some(0), Some(10 * DAY)),
            leg(1, LegStatus::Resolved, Some(20 * DAY), Some(30 * DAY)),
        ];
        let totals = stitch(&legs);
        assert_eq!(totals.lead_time_ms, Some(30 * DAY));
        assert_eq!(totals.legs_resolved, 2);
        assert_eq!(totals.legs_unresolved, 0);
        assert_eq!(totals.reason, None);
        assert_eq!(totals.value_time_ms, Some(2_000), "value time sums");
    }

    #[test]
    fn an_unresolved_leg_withholds_every_total() {
        // A missing leg understates the journey by exactly the part
        // nobody could see, so no total is published at all.
        for missing in [
            LegStatus::NotConfigured,
            LegStatus::UnavailableOrDenied,
            LegStatus::Unreachable,
            LegStatus::Truncated,
        ] {
            let legs = vec![
                leg(0, LegStatus::Resolved, Some(0), Some(10 * DAY)),
                leg(1, missing, None, None),
            ];
            let totals = stitch(&legs);
            assert_eq!(totals.lead_time_ms, None, "for {missing:?}");
            assert_eq!(totals.value_adding_ratio, None, "for {missing:?}");
            assert_eq!(totals.legs_resolved, 1);
            assert_eq!(totals.legs_unresolved, 1);
            let reason = totals.reason.expect("a null must say why");
            assert!(reason.contains("could not be read"), "{reason}");
        }
    }

    #[test]
    fn a_single_resolved_leg_is_a_complete_journey() {
        let totals = stitch(&[leg(0, LegStatus::Resolved, Some(0), Some(5 * DAY))]);
        assert_eq!(totals.lead_time_ms, Some(5 * DAY));
        assert_eq!(totals.reason, None);
    }

    #[test]
    fn an_empty_or_clockless_journey_is_null_with_a_reason() {
        let empty = stitch(&[]);
        assert_eq!(empty.lead_time_ms, None);
        assert!(empty.reason.is_some());

        let clockless = stitch(&[leg(0, LegStatus::Resolved, None, None)]);
        assert_eq!(clockless.lead_time_ms, None);
        assert!(
            clockless
                .reason
                .expect("reason")
                .contains("no clock bounds")
        );
    }

    #[test]
    fn the_walk_terminates_on_a_cycle() {
        // The write path refuses a self-link, but A → B → A is not
        // visible from either end at write time, so the read must
        // terminate on its own.
        let a: EntityRef = format!("care_pathway_instance:{}", Uuid::nil())
            .parse()
            .expect("ref");
        let mut seen = BTreeSet::new();
        assert!(next_hop(&mut seen, &a, 0).is_some(), "first visit");
        assert!(next_hop(&mut seen, &a, 1).is_none(), "revisit stops");
    }

    #[test]
    fn the_walk_stops_at_the_leg_limit() {
        let a: EntityRef = format!("care_pathway_instance:{}", Uuid::nil())
            .parse()
            .expect("ref");
        let mut seen = BTreeSet::new();
        assert!(next_hop(&mut seen, &a, MAX_LEGS).is_none());
    }

    #[test]
    fn only_an_operator_written_url_is_ever_contacted() {
        // The id is substituted into a template; nothing else about the
        // record reaches the URL.
        let id = Uuid::nil();
        assert_eq!(
            leg_url("http://peer/api/stays/{id}/time-analysis", id),
            format!("http://peer/api/stays/{id}/time-analysis")
        );
        // A template without `{id}` is used as-is rather than mangled.
        assert_eq!(
            leg_url("http://peer/api/current", id),
            "http://peer/api/current"
        );
    }

    #[test]
    fn a_peer_status_maps_to_one_leg_outcome() {
        assert_eq!(classify_status(200), LegStatus::Resolved);
        assert_eq!(classify_status(204), LegStatus::Resolved);
        // A denial and an absence are one outcome, so this side does not
        // re-open the leak the link endpoints closed.
        assert_eq!(classify_status(403), LegStatus::UnavailableOrDenied);
        assert_eq!(classify_status(404), LegStatus::UnavailableOrDenied);
        // A redirect is a peer pointing somewhere we will not follow.
        assert_eq!(classify_status(302), LegStatus::Unreachable);
        assert_eq!(classify_status(500), LegStatus::Unreachable);
        assert_eq!(classify_status(401), LegStatus::Unreachable);
    }

    /// The exact body `patient-flow-service` returns from
    /// `GET /api/stays/{pid}/time-analysis`, pinned here so the two
    /// services cannot drift apart silently. If patient-flow renames a
    /// field, this fails — rather than every stitched journey quietly
    /// reporting the stay leg as unreachable.
    #[test]
    fn the_patient_flow_response_satisfies_the_contract() {
        let body = serde_json::json!({
            "as_of": "2026-03-09T00:00:00Z",
            "stay": { "pid": "0c4f1e2a-0000-4000-8000-000000000000", "status": "admitted" },
            "note": "one leg of a stitched patient journey",
            "clock": {
                "start_ms": 1_772_000_000_000_i64,
                "stop_ms": 1_772_432_000_000_i64,
                "start_source": "admitted_at",
                "stop_source": "discharged_at",
                "running": false
            },
            "lead_time_ms": 432_000_000_i64,
            "value_time_ms": 172_800_000_i64,
            "span_days": 5,
            "classified_days": 5,
            "green_days": 2,
            "coverage": 1.0,
            "confidence": "classified"
        });
        let parsed = parse_leg(&body).expect("patient-flow satisfies the contract");
        assert_eq!(parsed.clock_start_ms, 1_772_000_000_000);
        assert_eq!(parsed.clock_stop_ms, 1_772_432_000_000);
        assert_eq!(parsed.lead_time_ms, 432_000_000);
        assert_eq!(parsed.value_time_ms, 172_800_000, "two green days");
    }

    #[test]
    fn a_leg_timeline_is_read_from_either_shape() {
        // This service's own time-analysis response, wrapped.
        let wrapped = serde_json::json!({
            "analysis": {
                "clock": { "start_ms": 10, "stop_ms": 110 },
                "lead_time_ms": 100,
                "value_time_ms": 14
            }
        });
        let parsed = parse_leg(&wrapped).expect("wrapped");
        assert_eq!(parsed.clock_start_ms, 10);
        assert_eq!(parsed.value_time_ms, 14);

        // A peer that answers flat.
        let flat = serde_json::json!({
            "start_ms": 10, "stop_ms": 110, "lead_time_ms": 100, "value_time_ms": 14
        });
        assert_eq!(parse_leg(&flat), Some(parsed));
    }

    #[test]
    fn a_missing_number_fails_the_leg_rather_than_defaulting_to_zero() {
        // An invented zero would flow straight into the stitched ratio.
        let no_value = serde_json::json!({
            "clock": { "start_ms": 0, "stop_ms": 10 }, "lead_time_ms": 10
        });
        assert_eq!(parse_leg(&no_value), None);
        assert_eq!(parse_leg(&serde_json::json!({})), None);
        assert_eq!(parse_leg(&serde_json::json!({ "analysis": {} })), None);
    }

    #[test]
    fn a_peer_is_only_contacted_when_configured() {
        // Unset ⇒ the link is known and its timeline is simply not
        // requested. A deployment that has not wired a peer says so
        // rather than failing.
        assert_eq!(url_template_for(EntityType::PatientFlowStay), None);
        assert_eq!(
            LegStatus::NotConfigured.detail(),
            "no journey URL configured for this service; the link is known, \
             its timeline was not requested"
        );
    }

    #[test]
    fn denial_and_absence_are_one_status() {
        // Distinguishing them here would re-open, from this side, the
        // leak the link endpoints just closed.
        assert!(!LegStatus::UnavailableOrDenied.is_resolved());
        let detail = LegStatus::UnavailableOrDenied.detail();
        assert!(detail.contains("indistinguishable"), "{detail}");
    }
}
