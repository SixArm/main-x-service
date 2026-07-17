//! Allocation eligibility + ranking (spec `bed-management.md` rules
//! 1–5, PF-D7): pure functions over plain fact structs. The allocator
//! **advises** — it returns eligible beds ranked; the operator picks;
//! rules 2 (sex) and 5 (ward fit) are overridable with a recorded
//! reason, which the controller audits.

use serde::{Deserialize, Serialize};

use super::bed_state::BedState;

/// A bed request's requirement flags (stored as the request's
/// `requirements` JSON).
#[allow(clippy::struct_excessive_bools)] // requirement flags are independent by nature
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requirements {
    /// Needs isolation (side room or isolation-capable bed).
    #[serde(default)]
    pub isolation: bool,
    /// Needs a side room specifically.
    #[serde(default)]
    pub side_room: bool,
    /// Needs piped oxygen.
    #[serde(default)]
    pub oxygen: bool,
    /// Needs a bariatric bed.
    #[serde(default)]
    pub bariatric: bool,
    /// The patient's sex token (rule 2), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sex: Option<String>,
}

/// Everything the rules need to know about one candidate bed. The
/// controller assembles this from the bed + bay + ward rows.
#[allow(clippy::struct_excessive_bools)] // rule inputs are independent facts
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedFacts {
    /// Current state (only `Available` is eligible).
    pub state: BedState,
    /// Ward is open and accepting admissions.
    pub ward_open: bool,
    /// Ward-level outbreak closure.
    pub ward_closed_to_admissions: bool,
    /// Bay-level outbreak closure.
    pub bay_closed_to_admissions: bool,
    /// Bay sex designation token (rule 2).
    pub bay_sex_designation: String,
    /// Single-occupancy side room.
    pub side_room: bool,
    /// Isolation-capable bed.
    pub isolation_capable: bool,
    /// Piped oxygen.
    pub oxygen: bool,
    /// Bariatric bed.
    pub bariatric: bool,
    /// The bed's ward matches the request's target ward (or no target
    /// was named).
    pub ward_matches_target: bool,
    /// The ward's specialty matches the request's (or none named).
    pub specialty_matches: bool,
    /// A virtual-ward slot.
    pub is_virtual: bool,
}

/// A rule breach. `overridable` breaches pass when the operator
/// supplies an override reason (recorded + audited).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Breach {
    /// Rule 1: bed not available / ward closed / bay closed.
    NotOpen,
    /// Rule 2: bay sex designation conflicts (overridable).
    SexSegregation,
    /// Rule 3: isolation / side-room need unmet.
    Isolation,
    /// Rule 4: oxygen / bariatric equipment unmet.
    Equipment,
    /// Rule 5: wrong ward / specialty — an outlier placement
    /// (overridable).
    WardFit,
}

impl Breach {
    /// Whether an operator override (with a recorded reason) may pass
    /// this breach.
    #[must_use]
    pub const fn overridable(self) -> bool {
        matches!(self, Self::SexSegregation | Self::WardFit)
    }
}

/// Which overrides the operator supplied.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Overrides {
    /// Override rule 2 (sex segregation) — a reportable event.
    pub sex: bool,
    /// Override rule 5 (ward fit) — an outlier placement.
    pub ward_fit: bool,
}

/// Evaluate rules 1–5. Returns the breaches that remain after
/// applying `overrides` (empty ⇒ eligible).
#[must_use]
pub fn breaches(facts: &BedFacts, req: &Requirements, overrides: Overrides) -> Vec<Breach> {
    let mut out = Vec::new();
    // Rule 1 — open for allocation. Never overridable.
    if facts.state != BedState::Available
        || !facts.ward_open
        || facts.ward_closed_to_admissions
        || facts.bay_closed_to_admissions
    {
        out.push(Breach::NotOpen);
    }
    // Rule 2 — sex segregation: the bay matches the patient's sex, or
    // is mixed/flexible, or is a side room (single occupancy).
    let sex_ok = facts.side_room
        || matches!(facts.bay_sex_designation.as_str(), "mixed" | "flexible")
        || req.sex.is_none()
        || req.sex.as_deref() == Some(facts.bay_sex_designation.as_str());
    if !sex_ok && !overrides.sex {
        out.push(Breach::SexSegregation);
    }
    // Rule 3 — isolation. `side_room` demands a side room; `isolation`
    // accepts a side room or an isolation-capable bed. Never
    // overridable (IPC).
    if (req.side_room && !facts.side_room)
        || (req.isolation && !(facts.side_room || facts.isolation_capable))
    {
        out.push(Breach::Isolation);
    }
    // Rule 4 — equipment. Never overridable.
    if (req.oxygen && !facts.oxygen) || (req.bariatric && !facts.bariatric) {
        out.push(Breach::Equipment);
    }
    // Rule 5 — ward fit: target ward or specialty matches.
    if !(facts.ward_matches_target || facts.specialty_matches || overrides.ward_fit) {
        out.push(Breach::WardFit);
    }
    out
}

/// Ranking key for an eligible bed — **lower sorts first** (spec:
/// right ward first, then side-room conservation: don't burn a side
/// room on a patient who doesn't need one).
#[must_use]
pub fn rank_key(facts: &BedFacts, req: &Requirements) -> (u8, u8) {
    let ward = u8::from(!facts.ward_matches_target);
    // A side room offered to a patient who doesn't need one ranks last.
    let conserve = u8::from(facts.side_room && !(req.side_room || req.isolation));
    (ward, conserve)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_bed() -> BedFacts {
        BedFacts {
            state: BedState::Available,
            ward_open: true,
            ward_closed_to_admissions: false,
            bay_closed_to_admissions: false,
            bay_sex_designation: "female".to_string(),
            side_room: false,
            isolation_capable: false,
            oxygen: false,
            bariatric: false,
            ward_matches_target: true,
            specialty_matches: true,
            is_virtual: false,
        }
    }

    fn no_req() -> Requirements {
        Requirements::default()
    }

    /// A clean open bed with no requirements is eligible.
    #[test]
    fn open_bed_is_eligible() {
        assert!(breaches(&open_bed(), &no_req(), Overrides::default()).is_empty());
    }

    /// Rule 1: every non-available state or closure flag breaches, and
    /// is never overridable.
    #[test]
    fn rule1_not_open() {
        for facts in [
            BedFacts { state: BedState::Occupied, ..open_bed() },
            BedFacts { state: BedState::Reserved, ..open_bed() },
            BedFacts { state: BedState::AwaitingClean, ..open_bed() },
            BedFacts { state: BedState::Closed, ..open_bed() },
            BedFacts { ward_open: false, ..open_bed() },
            BedFacts { ward_closed_to_admissions: true, ..open_bed() },
            BedFacts { bay_closed_to_admissions: true, ..open_bed() },
        ] {
            let b = breaches(&facts, &no_req(), Overrides { sex: true, ward_fit: true });
            assert!(b.contains(&Breach::NotOpen), "{facts:?}");
            assert!(!Breach::NotOpen.overridable());
        }
    }

    /// Rule 2: a male patient into a female bay breaches; a side room,
    /// a flexible/mixed bay, an unknown sex, or an override passes.
    #[test]
    fn rule2_sex_segregation() {
        let male = Requirements { sex: Some("male".to_string()), ..no_req() };
        assert!(breaches(&open_bed(), &male, Overrides::default()).contains(&Breach::SexSegregation));
        assert!(Breach::SexSegregation.overridable());
        // Override passes.
        assert!(breaches(&open_bed(), &male, Overrides { sex: true, ..Default::default() }).is_empty());
        // Side room passes.
        let side = BedFacts { side_room: true, ..open_bed() };
        assert!(breaches(&side, &male, Overrides::default()).is_empty());
        // Flexible bay passes.
        let flex = BedFacts { bay_sex_designation: "flexible".to_string(), ..open_bed() };
        assert!(breaches(&flex, &male, Overrides::default()).is_empty());
        // Matching sex passes.
        let female = Requirements { sex: Some("female".to_string()), ..no_req() };
        assert!(breaches(&open_bed(), &female, Overrides::default()).is_empty());
    }

    /// Rule 3: isolation demands a side room or isolation-capable bed;
    /// `side_room` demands a side room specifically; not overridable.
    #[test]
    fn rule3_isolation() {
        let iso = Requirements { isolation: true, ..no_req() };
        assert!(breaches(&open_bed(), &iso, Overrides::default()).contains(&Breach::Isolation));
        let capable = BedFacts { isolation_capable: true, ..open_bed() };
        assert!(breaches(&capable, &iso, Overrides::default()).is_empty());
        let side_req = Requirements { side_room: true, ..no_req() };
        assert!(breaches(&capable, &side_req, Overrides::default()).contains(&Breach::Isolation));
        let side = BedFacts { side_room: true, ..open_bed() };
        assert!(breaches(&side, &side_req, Overrides::default()).is_empty());
        assert!(!Breach::Isolation.overridable());
    }

    /// Rule 4: oxygen/bariatric must match; not overridable.
    #[test]
    fn rule4_equipment() {
        let o2 = Requirements { oxygen: true, ..no_req() };
        assert!(breaches(&open_bed(), &o2, Overrides::default()).contains(&Breach::Equipment));
        let with_o2 = BedFacts { oxygen: true, ..open_bed() };
        assert!(breaches(&with_o2, &o2, Overrides::default()).is_empty());
        let bar = Requirements { bariatric: true, ..no_req() };
        assert!(breaches(&open_bed(), &bar, Overrides::default()).contains(&Breach::Equipment));
    }

    /// Rule 5: wrong ward + wrong specialty is an outlier breach,
    /// overridable; either match passes.
    #[test]
    fn rule5_ward_fit() {
        let outlier = BedFacts { ward_matches_target: false, specialty_matches: false, ..open_bed() };
        assert!(breaches(&outlier, &no_req(), Overrides::default()).contains(&Breach::WardFit));
        assert!(Breach::WardFit.overridable());
        assert!(breaches(&outlier, &no_req(), Overrides { ward_fit: true, ..Default::default() }).is_empty());
        let specialty_only = BedFacts { ward_matches_target: false, ..open_bed() };
        assert!(breaches(&specialty_only, &no_req(), Overrides::default()).is_empty());
    }

    /// Ranking: right-ward beds first; side rooms rank last for
    /// patients who don't need one, first-equal for those who do.
    #[test]
    fn ranking_conserves_side_rooms() {
        let right_ward = open_bed();
        let right_side = BedFacts { side_room: true, ..open_bed() };
        let other_ward = BedFacts { ward_matches_target: false, ..open_bed() };
        assert!(rank_key(&right_ward, &no_req()) < rank_key(&right_side, &no_req()));
        assert!(rank_key(&right_side, &no_req()) < rank_key(&other_ward, &no_req()));
        let iso = Requirements { isolation: true, ..no_req() };
        assert_eq!(rank_key(&right_side, &iso), (0, 0));
    }
}
