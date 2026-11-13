//! The instrument catalogue for `instance_measures` (spec §13 T-15b).
//!
//! A measure may name a **catalogued instrument**. The catalogue is
//! code-reviewed reference data: for each instrument, the valid score
//! range, an optional screen threshold with its citation, and a licence
//! status. A free-named measure is still allowed and is untouched.
//!
//! **A screen is not a diagnosis.** A score at or above an instrument's
//! threshold is reported as `screen_positive`, and nothing here says
//! "case", "PTSD" or "depressed". The flag is **derived on read** and never
//! stored, so a corrected score corrects every view (the time-based
//! analysis rule: derived, never asserted).
//!
//! **What is stored is a total score, never the questionnaire.** The
//! registry holds one number per administration and none of the item text,
//! so a copyrighted instrument's wording is never reproduced here. The
//! licence status therefore concerns whether a *deployment* may use the
//! instrument, which it must check; this crate cannot decide that for it.
//!
//! **Nothing here was checked against a licence-holder.** Every entry's
//! licence status is `Unconfirmed` or `Restricted`. None is `Open`, because
//! the only support for "free to use" found while seeding this catalogue
//! was secondary (clinic pages and calculators), and one primary source
//! contradicted the public-domain claim for the PHQ-9.

use serde::Serialize;

/// What an instrument measures. `MentalHealth` drives the masking of its
/// scores (spec §13 T-15a), so adding a category is a privacy decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Psychological distress, trauma, anxiety or mood.
    MentalHealth,
}

/// Who completes the instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Administration {
    /// A clinician scores it from observation or interview.
    Clinician,
    /// The patient completes it.
    SelfReport,
}

/// Whether a deployment may use the instrument. Deliberately without an
/// "open" variant: see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Licence {
    /// Reported as free to use, but not confirmed with the holder. Check
    /// before relying on it.
    Unconfirmed,
    /// Licensed commercially. The entry carries a range so a score can be
    /// recorded by a licensed deployment, and **no threshold**, because a
    /// threshold is part of what the licence covers.
    Restricted,
}

/// One catalogued instrument.
#[derive(Debug, Clone, Serialize)]
pub struct Instrument {
    /// Stable id, as sent in a measure's `instrument`.
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// What it measures.
    pub category: Category,
    /// Who completes it.
    pub administration: Administration,
    /// Lowest valid total score.
    pub min: f64,
    /// Highest valid total score.
    pub max: f64,
    /// A score **at or above** this is a positive screen. `None` where no
    /// defensible single threshold exists or the licence forbids one.
    pub screen_at: Option<f64>,
    /// Source for the range and the threshold.
    pub citation: &'static str,
    /// Licence status, see [`Licence`].
    pub licence: Licence,
    /// What was and was not established about the licence, and any caveat
    /// on the threshold. Never empty.
    pub licence_note: &'static str,
    /// A LOINC code, only if one was checked against a LOINC release. None
    /// is seeded: LOINC codes are exact, and an unchecked one is worse than
    /// none.
    pub loinc: Option<&'static str>,
}

/// The catalogue. Reference data, not clinical advice.
pub const CATALOGUE: &[Instrument] = &[
    Instrument {
        id: "ipat",
        name: "Intensive care Psychological Assessment Tool (IPAT)",
        category: Category::MentalHealth,
        administration: Administration::Clinician,
        min: 0.0,
        max: 20.0,
        screen_at: Some(7.0),
        citation: "Wade DM et al. Crit Care 2014;18:519 (10 items scored 0-2; cut-point 7 \
                   detects concurrent anxiety at 82% sensitivity / 65% specificity)",
        licence: Licence::Unconfirmed,
        licence_note: "Published in an open-access journal; reuse terms not checked. The \
                       threshold detects current distress. Its prediction of later \
                       morbidity is weak (sensitivity 57%, AUC 0.7), so a positive screen \
                       is a prompt to assess, not a forecast.",
        loinc: None,
    },
    Instrument {
        id: "pcl5",
        name: "PTSD Checklist for DSM-5 (PCL-5)",
        category: Category::MentalHealth,
        administration: Administration::SelfReport,
        min: 0.0,
        max: 80.0,
        screen_at: Some(31.0),
        citation: "Weathers FW et al., National Center for PTSD (2013); ISTSS guidance gives \
                   a 31-33 range, lower for screening",
        licence: Licence::Unconfirmed,
        licence_note: "Reported as not copyrighted (US VA National Center for PTSD), from \
                       secondary sources only. The threshold is the screening end of a \
                       31-33 range, chosen to maximise detection. A deployment wanting \
                       fewer false positives should treat 33 as its line.",
        loinc: None,
    },
    Instrument {
        id: "phq9",
        name: "Patient Health Questionnaire-9 (PHQ-9)",
        category: Category::MentalHealth,
        administration: Administration::SelfReport,
        min: 0.0,
        max: 27.0,
        screen_at: Some(10.0),
        citation: "Kroenke K, Spitzer RL, Williams JB. J Gen Intern Med 2001;16:606-13",
        licence: Licence::Unconfirmed,
        licence_note: "Secondary sources say Pfizer placed it in the public domain in \
                       2010; that was not confirmed, and a primary document directs users \
                       to the holder for permission. Check before use.",
        loinc: None,
    },
    Instrument {
        id: "gad7",
        name: "Generalized Anxiety Disorder-7 (GAD-7)",
        category: Category::MentalHealth,
        administration: Administration::SelfReport,
        min: 0.0,
        max: 21.0,
        screen_at: Some(10.0),
        citation: "Spitzer RL et al. Arch Intern Med 2006;166:1092-7",
        licence: Licence::Unconfirmed,
        licence_note: "Same position as the PHQ-9: reported public domain, unconfirmed.",
        loinc: None,
    },
    Instrument {
        id: "ies_r",
        name: "Impact of Event Scale-Revised (IES-R)",
        category: Category::MentalHealth,
        administration: Administration::SelfReport,
        min: 0.0,
        max: 88.0,
        screen_at: None,
        citation: "Weiss DS, Marmar CR (1997). The Impact of Event Scale-Revised",
        licence: Licence::Unconfirmed,
        licence_note: "Described as copyrighted, to be obtained from the authors. No \
                       threshold: the often-quoted 33 comes from a single veteran sample \
                       and is not author-endorsed. The score is recorded; no screen is \
                       derived from it.",
        loinc: None,
    },
    Instrument {
        id: "hads_a",
        name: "Hospital Anxiety and Depression Scale, anxiety subscale (HADS-A)",
        category: Category::MentalHealth,
        administration: Administration::SelfReport,
        min: 0.0,
        max: 21.0,
        screen_at: None,
        citation: "Zigmond AS, Snaith RP. Acta Psychiatr Scand 1983;67:361-70",
        licence: Licence::Restricted,
        licence_note: "Commercially licensed. Recordable by a deployment that holds a \
                       licence. No threshold is seeded.",
        loinc: None,
    },
    Instrument {
        id: "hads_d",
        name: "Hospital Anxiety and Depression Scale, depression subscale (HADS-D)",
        category: Category::MentalHealth,
        administration: Administration::SelfReport,
        min: 0.0,
        max: 21.0,
        screen_at: None,
        citation: "Zigmond AS, Snaith RP. Acta Psychiatr Scand 1983;67:361-70",
        licence: Licence::Restricted,
        licence_note: "As HADS-A.",
        loinc: None,
    },
];

/// Look one instrument up by id.
#[must_use]
pub fn find(id: &str) -> Option<&'static Instrument> {
    CATALOGUE.iter().find(|i| i.id == id)
}

/// Check a score against an instrument's range, returning the reason it
/// is refused. The id must be in the catalogue.
///
/// # Errors
///
/// When the instrument is unknown, the score is missing, not finite, or
/// outside the instrument's range.
pub fn check_score(instrument_id: &str, score: Option<f64>) -> Result<&'static Instrument, String> {
    let instrument = find(instrument_id).ok_or_else(|| {
        let ids: Vec<&str> = CATALOGUE.iter().map(|i| i.id).collect();
        format!("unknown instrument `{instrument_id}` (instruments: {ids:?})")
    })?;
    let Some(score) = score else {
        return Err(format!(
            "instrument `{instrument_id}` needs a numeric value_numeric (the total score)"
        ));
    };
    if !score.is_finite() || score < instrument.min || score > instrument.max {
        return Err(format!(
            "score {score} is outside the range {}-{} for `{instrument_id}`",
            instrument.min, instrument.max
        ));
    }
    Ok(instrument)
}

/// Whether a recorded score is a positive screen. `None` when the measure
/// names no instrument, the instrument has no threshold, there is no
/// numeric score, or the id is no longer in the catalogue: nothing is
/// derived where nothing can be defended.
#[must_use]
pub fn screen_positive(instrument_id: Option<&str>, score: Option<f64>) -> Option<bool> {
    let threshold = find(instrument_id?)?.screen_at?;
    Some(score? >= threshold)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every entry is complete: a citation, a licence note, a sane range,
    /// a threshold inside it, and no unchecked LOINC code.
    #[test]
    fn every_entry_is_complete() {
        let mut seen = std::collections::BTreeSet::new();
        for i in CATALOGUE {
            assert!(seen.insert(i.id), "duplicate id {}", i.id);
            assert!(!i.citation.trim().is_empty(), "{} has no citation", i.id);
            assert!(
                !i.licence_note.trim().is_empty(),
                "{} has no licence note",
                i.id
            );
            assert!(i.min < i.max, "{} has an empty range", i.id);
            if let Some(t) = i.screen_at {
                assert!(
                    t > i.min && t <= i.max,
                    "{} threshold outside its range",
                    i.id
                );
            }
            assert!(
                i.loinc.is_none(),
                "{}: a LOINC code needs a checked source",
                i.id
            );
        }
    }

    /// A restricted instrument carries no threshold: the threshold is
    /// part of what the licence covers.
    #[test]
    fn restricted_instruments_have_no_threshold() {
        for i in CATALOGUE
            .iter()
            .filter(|i| i.licence == Licence::Restricted)
        {
            assert!(
                i.screen_at.is_none(),
                "{} is restricted but has a threshold",
                i.id
            );
        }
    }

    /// The range is enforced at both ends and for non-finite input.
    #[test]
    fn scores_outside_the_range_are_refused() {
        assert!(check_score("ipat", Some(0.0)).is_ok());
        assert!(check_score("ipat", Some(20.0)).is_ok());
        assert!(check_score("ipat", Some(20.5)).is_err());
        assert!(check_score("ipat", Some(-1.0)).is_err());
        assert!(check_score("ipat", Some(f64::NAN)).is_err());
        assert!(check_score("ipat", Some(f64::INFINITY)).is_err());
        assert!(check_score("ipat", None).is_err());
        assert!(check_score("nope", Some(1.0)).is_err());
    }

    /// The threshold is inclusive: IPAT 7 is positive and 6 is not.
    #[test]
    fn screen_threshold_is_at_or_above() {
        assert_eq!(screen_positive(Some("ipat"), Some(7.0)), Some(true));
        assert_eq!(screen_positive(Some("ipat"), Some(6.0)), Some(false));
        assert_eq!(screen_positive(Some("phq9"), Some(10.0)), Some(true));
        assert_eq!(screen_positive(Some("phq9"), Some(9.0)), Some(false));
    }

    /// Nothing is derived where nothing can be defended.
    #[test]
    fn no_screen_without_an_instrument_threshold_or_score() {
        assert_eq!(screen_positive(None, Some(30.0)), None);
        assert_eq!(screen_positive(Some("ies_r"), Some(80.0)), None);
        assert_eq!(screen_positive(Some("hads_a"), Some(21.0)), None);
        assert_eq!(screen_positive(Some("ipat"), None), None);
        assert_eq!(screen_positive(Some("retired_id"), Some(30.0)), None);
    }
}
