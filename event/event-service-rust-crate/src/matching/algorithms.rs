//! Component matching algorithms for events.
//!
//! Each `mod` here scores a single facet (name, time, location, …)
//! in `[0.0, 1.0]`. The scorers in [`crate::matching::scoring`] combine
//! these into an overall match.
//!
//! All scorers are pure functions of their inputs (no I/O, no shared
//! state), which makes them cheap to test and to call in a hot loop
//! over candidate events.
//!
//! # Examples
//!
//! ```
//! use event_service::matching::algorithms::name_matching::match_titles;
//!
//! // Identical (case-insensitive) titles score 1.0.
//! assert!(match_titles("Concert", "concert") > 0.99);
//! // A small typo still scores high thanks to Jaro-Winkler.
//! assert!(match_titles("Conference", "Conferance") > 0.85);
//! // An empty title can never match.
//! assert_eq!(match_titles("", "Concert"), 0.0);
//! ```

use chrono::{DateTime, Utc};
use strsim::{jaro_winkler, normalized_levenshtein};

use crate::models::{Address, Identifier, Location, Party, Reference};

// ============================================================================
// Name / title matching
// ============================================================================

/// Name / title similarity scoring.
pub mod name_matching {
    use super::*;

    /// Score two event titles. Combines case-insensitive
    /// Jaro-Winkler, normalized Levenshtein, and a Soundex phonetic
    /// floor for sound-alike titles.
    ///
    /// Returns `0.0` when either side is empty/whitespace, `1.0` for a
    /// case-insensitive exact match, otherwise the max of the three
    /// similarity measures (the phonetic floor is `0.85`, applied only
    /// when the Soundex codes are equal).
    pub fn match_titles(a: &str, b: &str) -> f64 {
        let a = a.trim();
        let b = b.trim();
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let al = a.to_lowercase();
        let bl = b.to_lowercase();
        if al == bl {
            return 1.0;
        }
        let jw = jaro_winkler(&al, &bl);
        let lev = normalized_levenshtein(&al, &bl);
        let phonetic = crate::matching::phonetic::phonetic_similarity(&al, &bl);
        let phonetic_floor = if phonetic >= 1.0 { 0.85 } else { 0.0 };
        f64::max(f64::max(jw, lev), phonetic_floor)
    }

    /// Take the best pairwise score across two name lists
    /// (primary `name` versus `alternate_names`, in either direction).
    pub fn match_name_with_alternates(
        primary_a: &str,
        alternates_a: &[String],
        primary_b: &str,
        alternates_b: &[String],
    ) -> f64 {
        let names_a: Vec<&str> = std::iter::once(primary_a)
            .chain(alternates_a.iter().map(|s| s.as_str()))
            .collect();
        let names_b: Vec<&str> = std::iter::once(primary_b)
            .chain(alternates_b.iter().map(|s| s.as_str()))
            .collect();
        let mut best: f64 = 0.0;
        for x in &names_a {
            for y in &names_b {
                let s = match_titles(x, y);
                if s > best {
                    best = s;
                }
            }
        }
        best
    }
}

// ============================================================================
// Date / time proximity matching
// ============================================================================

/// Date / time proximity scoring.
pub mod time_matching {
    use super::*;

    /// Score how close two `start_date`s are.
    ///
    /// Exact match = 1.0; within 5 min ≈ 0.99; within 1 h ≈ 0.95;
    /// within 1 day ≈ 0.80; within 1 week ≈ 0.40; further → 0.0.
    pub fn match_start_dates(a: DateTime<Utc>, b: DateTime<Utc>) -> f64 {
        let secs_diff = a.signed_duration_since(b).num_seconds().unsigned_abs() as f64;
        // Exponential decay; half-life ≈ 1 hour.
        let half_life_secs: f64 = 3600.0;
        let score = (-secs_diff / half_life_secs).exp2().max(0.0);
        score.clamp(0.0, 1.0)
    }

    /// Score end-date proximity. Handles unknown end dates as
    /// neutral (0.5) when both are missing, or 0.0 when only one is
    /// missing.
    pub fn match_end_dates(a: Option<DateTime<Utc>>, b: Option<DateTime<Utc>>) -> f64 {
        match (a, b) {
            (None, None) => 0.5,
            (None, Some(_)) | (Some(_), None) => 0.0,
            (Some(x), Some(y)) => match_start_dates(x, y),
        }
    }

    /// Score the overlap between two events' time windows.
    /// Returns the Jaccard ratio: |intersection| / |union|.
    /// If either end is open-ended, fall back to `match_start_dates`.
    pub fn match_window_overlap(
        a_start: DateTime<Utc>,
        a_end: Option<DateTime<Utc>>,
        b_start: DateTime<Utc>,
        b_end: Option<DateTime<Utc>>,
    ) -> f64 {
        let (Some(ae), Some(be)) = (a_end, b_end) else {
            return match_start_dates(a_start, b_start);
        };
        let inter_start = a_start.max(b_start);
        let inter_end = ae.min(be);
        if inter_end <= inter_start {
            return 0.0;
        }
        let inter = inter_end.signed_duration_since(inter_start).num_seconds() as f64;
        let union_start = a_start.min(b_start);
        let union_end = ae.max(be);
        let union = union_end.signed_duration_since(union_start).num_seconds() as f64;
        if union <= 0.0 {
            return 0.0;
        }
        (inter / union).clamp(0.0, 1.0)
    }
}

// ============================================================================
// Location matching
// ============================================================================

/// Location matching, dispatched by [`Location`] variant.
pub mod location_matching {
    use super::*;

    /// Score the best pairwise location match across two lists.
    ///
    /// Returns `0.0` if either list is empty; otherwise the maximum
    /// [`match_location`] score over the Cartesian product.
    pub fn match_locations(a: &[Location], b: &[Location]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let mut best: f64 = 0.0;
        for x in a {
            for y in b {
                let s = match_location(x, y);
                if s > best {
                    best = s;
                }
            }
        }
        best
    }

    /// Score a single pair of locations, dispatching on the variant
    /// combination:
    ///
    /// - `Place ↔ Place`: short-circuit `1.0` when both share an
    ///   external `id`; else `0.4·name + 0.4·address + 0.2·geo`.
    /// - `PostalAddress ↔ PostalAddress`: [`match_address`].
    /// - `Place ↔ PostalAddress`: compares the place's address.
    /// - `Virtual ↔ Virtual`: case-insensitive URL equality.
    /// - `Text ↔ Text`: title similarity.
    /// - any other cross-variant pairing: `0.0`.
    pub fn match_location(a: &Location, b: &Location) -> f64 {
        match (a, b) {
            (Location::Place(p1), Location::Place(p2)) => {
                if let (Some(i1), Some(i2)) = (p1.id, p2.id) {
                    if i1 == i2 {
                        return 1.0;
                    }
                }
                // Fall back to name + address comparison.
                let name_score = name_matching::match_titles(&p1.name, &p2.name);
                let addr_score = match (p1.address.as_ref(), p2.address.as_ref()) {
                    (Some(a1), Some(a2)) => match_address(a1, a2),
                    _ => 0.0,
                };
                let geo_score = match (p1.latitude, p1.longitude, p2.latitude, p2.longitude) {
                    (Some(la1), Some(lo1), Some(la2), Some(lo2)) => {
                        geo_proximity(la1, lo1, la2, lo2)
                    }
                    _ => 0.0,
                };
                let combined = (name_score * 0.4) + (addr_score * 0.4) + (geo_score * 0.2);
                combined.clamp(0.0, 1.0)
            }
            (Location::PostalAddress(a1), Location::PostalAddress(a2)) => match_address(a1, a2),
            (Location::Place(p), Location::PostalAddress(a))
            | (Location::PostalAddress(a), Location::Place(p)) => match p.address.as_ref() {
                Some(pa) => match_address(pa, a),
                None => 0.0,
            },
            (Location::Virtual(v1), Location::Virtual(v2)) => {
                if v1.url.trim().eq_ignore_ascii_case(v2.url.trim()) {
                    1.0
                } else {
                    0.0
                }
            }
            (Location::Text { value: x }, Location::Text { value: y }) => {
                name_matching::match_titles(x, y)
            }
            _ => 0.0,
        }
    }

    /// Compare two postal addresses by postal_code / city / state / line1.
    pub fn match_address(a: &Address, b: &Address) -> f64 {
        const W_POSTAL: f64 = 0.30;
        const W_CITY: f64 = 0.20;
        const W_STATE: f64 = 0.20;
        const W_STREET: f64 = 0.30;
        let postal = match_postal_codes(a.postal_code.as_deref(), b.postal_code.as_deref());
        let city = match_text_field(a.city.as_deref(), b.city.as_deref());
        let state = match_exact_field(a.state.as_deref(), b.state.as_deref());
        let street = match_text_field(a.line1.as_deref(), b.line1.as_deref());
        postal * W_POSTAL + city * W_CITY + state * W_STATE + street * W_STREET
    }

    /// Compare two postal codes after stripping dashes: exact `1.0`,
    /// shared 5-digit prefix `0.95`, shared 3-digit prefix `0.70`,
    /// else `0.0`. Missing on either side scores `0.0`.
    fn match_postal_codes(a: Option<&str>, b: Option<&str>) -> f64 {
        match (a, b) {
            (Some(x), Some(y)) => {
                let x = x.trim().replace('-', "");
                let y = y.trim().replace('-', "");
                if x == y {
                    1.0
                } else if x.len() >= 5 && y.len() >= 5 && x[..5] == y[..5] {
                    0.95
                } else if x.len() >= 3 && y.len() >= 3 && x[..3] == y[..3] {
                    0.70
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    /// Case-insensitive Jaro-Winkler comparison of an optional text
    /// field (e.g. city, street). Missing on either side scores `0.0`.
    fn match_text_field(a: Option<&str>, b: Option<&str>) -> f64 {
        match (a, b) {
            (Some(x), Some(y)) => {
                let xl = x.trim().to_lowercase();
                let yl = y.trim().to_lowercase();
                if xl == yl {
                    1.0
                } else {
                    jaro_winkler(&xl, &yl)
                }
            }
            _ => 0.0,
        }
    }

    /// All-or-nothing comparison of an optional field (e.g. state):
    /// `1.0` for a case-insensitive exact match, else `0.0`.
    fn match_exact_field(a: Option<&str>, b: Option<&str>) -> f64 {
        match (a, b) {
            (Some(x), Some(y)) if x.trim().eq_ignore_ascii_case(y.trim()) => 1.0,
            _ => 0.0,
        }
    }

    /// Haversine-with-sigmoid-decay proximity, in `[0, 1]`. Coincident
    /// points score `1.0`; the score decays with great-circle distance.
    fn geo_proximity(la1: f64, lo1: f64, la2: f64, lo2: f64) -> f64 {
        let r = 6371.0_f64; // Earth radius km
        let dlat = (la2 - la1).to_radians();
        let dlon = (lo2 - lo1).to_radians();
        let a = (dlat / 2.0).sin().powi(2)
            + la1.to_radians().cos() * la2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        let dist_km = r * c;
        // Sigmoid: at 0km → 1.0; at 1km → ~0.88; at 10km → ~0.05.
        1.0 / (1.0 + (dist_km / 2.0).exp() - 1.0).max(0.0)
    }
}

// ============================================================================
// Party (organizer / performer / attendee) matching
// ============================================================================

/// Party (organizer / performer / attendee) matching.
pub mod party_matching {
    use super::*;

    /// Score the best pair across two party lists. `0.0` when either
    /// list is empty.
    pub fn match_parties(a: &[Party], b: &[Party]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let mut best: f64 = 0.0;
        for x in a {
            for y in b {
                let s = match_party(x, y);
                if s > best {
                    best = s;
                }
            }
        }
        best
    }

    /// Score a single pair of parties:
    ///
    /// - Different [`PartyKind`](crate::models::PartyKind) → `0.0`.
    /// - Same external `id` → `1.0` (deterministic short-circuit).
    /// - Otherwise `max(name similarity, exact-email match)`.
    pub fn match_party(a: &Party, b: &Party) -> f64 {
        if a.kind != b.kind {
            return 0.0;
        }
        if let (Some(i1), Some(i2)) = (a.id, b.id) {
            if i1 == i2 {
                return 1.0;
            }
        }
        let name_score = name_matching::match_titles(&a.name, &b.name);
        let email_score = match (a.email.as_deref(), b.email.as_deref()) {
            (Some(x), Some(y)) if x.eq_ignore_ascii_case(y) => 1.0,
            _ => 0.0,
        };
        f64::max(name_score, email_score)
    }
}

// ============================================================================
// Identifier matching
// ============================================================================

/// Identifier matching (exact + formatting-tolerant).
pub mod identifier_matching {
    use super::*;

    /// Best pairwise identifier score across two lists. `0.0` when
    /// either list is empty.
    pub fn match_identifiers(a: &[Identifier], b: &[Identifier]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let mut best: f64 = 0.0;
        for x in a {
            for y in b {
                let s = match_identifier(x, y);
                if s > best {
                    best = s;
                }
            }
        }
        best
    }

    /// Score a single pair of identifiers:
    ///
    /// - Different type or system → `0.0`.
    /// - Identical normalized value → `1.0`.
    /// - Identical once dashes/spaces are stripped → `0.98`.
    /// - Otherwise → `0.0`.
    pub fn match_identifier(a: &Identifier, b: &Identifier) -> f64 {
        if a.identifier_type != b.identifier_type || a.system != b.system {
            return 0.0;
        }
        let xl = a.value.trim().to_lowercase();
        let yl = b.value.trim().to_lowercase();
        if xl == yl {
            return 1.0;
        }
        let xc = xl.replace('-', "").replace(' ', "");
        let yc = yl.replace('-', "").replace(' ', "");
        if xc == yc {
            return 0.98;
        }
        0.0
    }
}

// ============================================================================
// Reference matching (about / works)
// ============================================================================

/// Reference matching for `about` / `works` lists.
pub mod reference_matching {
    use super::*;

    /// Best pairwise reference score across two lists. `0.0` when
    /// either list is empty.
    pub fn match_references(a: &[Reference], b: &[Reference]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let mut best: f64 = 0.0;
        for x in a {
            for y in b {
                let s = match_reference(x, y);
                if s > best {
                    best = s;
                }
            }
        }
        best
    }

    /// Score a single pair of references: shared external `id` →
    /// `1.0`; otherwise name similarity.
    pub fn match_reference(a: &Reference, b: &Reference) -> f64 {
        if let (Some(i1), Some(i2)) = (a.id, b.id) {
            if i1 == i2 {
                return 1.0;
            }
        }
        name_matching::match_titles(&a.name, &b.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Address, Place};
    use chrono::TimeZone;

    /// Build a fixed UTC timestamp for deterministic time tests.
    fn dt(y: i32, mo: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, 0, 0).unwrap()
    }

    /// Identical titles score ~1.0.
    #[test]
    fn exact_title_match() {
        let s = name_matching::match_titles("Concert", "Concert");
        assert!(s > 0.99);
    }

    /// A one-letter typo still scores above 0.85 (Jaro-Winkler bonus).
    #[test]
    fn fuzzy_title_match() {
        let s = name_matching::match_titles("Annual Conference", "Annual Conferance");
        assert!(s > 0.85, "got {s}");
    }

    /// An empty title on either side scores 0.0.
    #[test]
    fn empty_titles_score_zero() {
        assert_eq!(name_matching::match_titles("", "anything"), 0.0);
        assert_eq!(name_matching::match_titles("anything", ""), 0.0);
    }

    /// The best match across primary + alternate names wins, even
    /// cross-list (an alias of A matches the primary of B).
    #[test]
    fn name_with_alternates() {
        let s = name_matching::match_name_with_alternates(
            "Main",
            &["Alias".into()],
            "Something Else",
            &["Main".into()],
        );
        assert!(s > 0.99);
    }

    /// Same start instant → ~1.0.
    #[test]
    fn exact_start_date_match() {
        let s = time_matching::match_start_dates(dt(2026, 3, 1, 9), dt(2026, 3, 1, 9));
        assert!(s > 0.99);
    }

    /// One hour apart sits near the half-life (~0.5).
    #[test]
    fn close_start_date_match() {
        let s = time_matching::match_start_dates(dt(2026, 3, 1, 9), dt(2026, 3, 1, 10));
        assert!(s > 0.4 && s < 0.6, "got {s}");
    }

    /// A month apart decays to near zero.
    #[test]
    fn distant_start_date_low() {
        let s = time_matching::match_start_dates(dt(2026, 3, 1, 9), dt(2026, 4, 1, 9));
        assert!(s < 0.1, "got {s}");
    }

    /// Window overlap is the Jaccard ratio of the two intervals.
    #[test]
    fn window_overlap() {
        let s = time_matching::match_window_overlap(
            dt(2026, 3, 1, 9),
            Some(dt(2026, 3, 1, 12)),
            dt(2026, 3, 1, 10),
            Some(dt(2026, 3, 1, 13)),
        );
        // intersection 10–12 (2h) ; union 9–13 (4h) → 0.5
        assert!((s - 0.5).abs() < 0.01, "got {s}");
    }

    /// Two places sharing an external id match deterministically (1.0).
    #[test]
    fn location_place_id_short_circuits() {
        let id = uuid::Uuid::new_v4();
        let p1 = Place {
            id: Some(id),
            name: "x".into(),
            address: None,
            latitude: None,
            longitude: None,
            url: None,
        };
        let p2 = Place {
            id: Some(id),
            name: "y".into(),
            address: None,
            latitude: None,
            longitude: None,
            url: None,
        };
        assert_eq!(
            location_matching::match_location(&Location::Place(p1), &Location::Place(p2)),
            1.0
        );
    }

    /// Identical postal addresses score ~1.0.
    #[test]
    fn location_address_matches() {
        let a = Address {
            use_type: None,
            line1: Some("1 Main St".into()),
            line2: None,
            city: Some("Town".into()),
            state: Some("CA".into()),
            postal_code: Some("94000".into()),
            country: Some("US".into()),
        };
        let b = a.clone();
        let s = location_matching::match_address(&a, &b);
        assert!(s > 0.99, "got {s}");
    }

    /// Two virtual locations with the same URL match (1.0) regardless
    /// of differing display names.
    #[test]
    fn virtual_url_exact_match() {
        let v1 = crate::models::VirtualLocation {
            name: None,
            url: "https://x.test".into(),
        };
        let v2 = crate::models::VirtualLocation {
            name: Some("y".into()),
            url: "https://x.test".into(),
        };
        let s = location_matching::match_location(&Location::Virtual(v1), &Location::Virtual(v2));
        assert_eq!(s, 1.0);
    }

    /// Parties sharing an external id match deterministically (1.0).
    #[test]
    fn party_match_by_id_short_circuits() {
        use crate::models::{Party, PartyKind};
        let id = uuid::Uuid::new_v4();
        let a = Party {
            kind: PartyKind::Person,
            id: Some(id),
            name: "x".into(),
            email: None,
            url: None,
        };
        let b = Party {
            kind: PartyKind::Person,
            id: Some(id),
            name: "y".into(),
            email: None,
            url: None,
        };
        assert_eq!(party_matching::match_party(&a, &b), 1.0);
    }

    /// A person and an organization never match, even with equal names.
    #[test]
    fn party_kind_mismatch() {
        use crate::models::{Party, PartyKind};
        let a = Party {
            kind: PartyKind::Person,
            id: None,
            name: "Acme".into(),
            email: None,
            url: None,
        };
        let b = Party {
            kind: PartyKind::Organization,
            id: None,
            name: "Acme".into(),
            email: None,
            url: None,
        };
        assert_eq!(party_matching::match_party(&a, &b), 0.0);
    }

    /// Identical type + system + value scores 1.0.
    #[test]
    fn identifier_exact_match() {
        use crate::models::{Identifier, IdentifierType};
        let a = Identifier::new(
            IdentifierType::BookingNumber,
            "sys".into(),
            "ABC-123".into(),
        );
        let b = Identifier::new(
            IdentifierType::BookingNumber,
            "sys".into(),
            "ABC-123".into(),
        );
        assert_eq!(identifier_matching::match_identifier(&a, &b), 1.0);
    }

    /// Values differing only by dashes/spaces score 0.98 (tolerant).
    #[test]
    fn identifier_formatting_difference() {
        use crate::models::{Identifier, IdentifierType};
        let a = Identifier::new(
            IdentifierType::BookingNumber,
            "sys".into(),
            "ABC-123".into(),
        );
        let b = Identifier::new(
            IdentifierType::BookingNumber,
            "sys".into(),
            "abc 123".into(),
        );
        let s = identifier_matching::match_identifier(&a, &b);
        assert!(s > 0.97 && s < 1.0, "got {s}");
    }

    /// Same value but different type scores 0.0.
    #[test]
    fn identifier_type_mismatch() {
        use crate::models::{Identifier, IdentifierType};
        let a = Identifier::new(IdentifierType::BookingNumber, "sys".into(), "X".into());
        let b = Identifier::new(IdentifierType::TicketNumber, "sys".into(), "X".into());
        assert_eq!(identifier_matching::match_identifier(&a, &b), 0.0);
    }
}
