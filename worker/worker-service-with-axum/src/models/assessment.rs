//! Workforce **assessments** — aptitude, personality, psychometric, and
//! selection tests recorded against a [`Worker`](crate::models::Worker).
//!
//! An [`Assessment`] is one administration of one instrument (a named
//! test) to one worker. Its [`AssessmentCategory`] says what family of
//! test it is; its [`AssessmentResult`]s carry the per-[`AssessmentScale`]
//! outcome — the dimension measured, the raw / percentile score, and the
//! interpreted [`ScoreBand`].
//!
//! The four categories and the scales they measure:
//!
//! | Category | Scales |
//! |---|---|
//! | [`Aptitude`](AssessmentCategory::Aptitude) — how a person performs at tasks and reacts to situations | numerical reasoning, verbal reasoning, problem solving, logical thinking |
//! | [`Personality`](AssessmentCategory::Personality) — behavioural style and working qualities | work style, team compatibility, introversion/extraversion |
//! | [`Psychometric`](AssessmentCategory::Psychometric) — spans aptitude **and** personality | behavioural style, emotional intelligence, cognitive ability (**plus** every aptitude and personality scale) |
//! | [`Selection`](AssessmentCategory::Selection) — role suitability during hiring | job simulation, skills assessment, judgement test |
//!
//! Psychometric is the deliberate overlap: per the domain definition it
//! "covers aptitude and personality", so
//! [`AssessmentCategory::permits`] accepts aptitude and personality
//! scales on a psychometric assessment as well as its own three. Every
//! other category accepts only its own scales, so a mis-filed result is
//! a `422` rather than silent data drift.
//!
//! Assessment results are **sensitive personal data** (they profile
//! cognition and behaviour), so [`Assessment::masked`] is the redacted
//! projection the read path returns under the ABAC `mask` obligation:
//! bands survive, raw scores / percentiles / narratives do not.
//!
//! # Examples
//!
//! ```
//! use worker_service::models::assessment::{
//!     Assessment, AssessmentCategory, AssessmentResult, AssessmentScale,
//!     AssessmentStatus, ScoreBand,
//! };
//! use uuid::Uuid;
//!
//! let worker_id = Uuid::new_v4();
//! let mut a = Assessment::new(worker_id, AssessmentCategory::Aptitude, "Watson-Glaser III");
//! a.results.push(AssessmentResult::percentile(AssessmentScale::LogicalThinking, 92.0));
//! a.status = AssessmentStatus::Completed;
//!
//! assert_eq!(a.results[0].effective_band(), Some(ScoreBand::High));
//! // A logical-thinking result belongs on an aptitude assessment.
//! assert!(AssessmentCategory::Aptitude.permits(AssessmentScale::LogicalThinking));
//! // …and on a psychometric one, which spans aptitude and personality.
//! assert!(AssessmentCategory::Psychometric.permits(AssessmentScale::LogicalThinking));
//! // …but not on a selection assessment.
//! assert!(!AssessmentCategory::Selection.permits(AssessmentScale::LogicalThinking));
//! ```

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// The family a test belongs to.
///
/// Serializes in `snake_case` (`"aptitude"`, `"personality"`,
/// `"psychometric"`, `"selection"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentCategory {
    /// Measures how a person performs at tasks and reacts to situations:
    /// numerical and verbal reasoning, problem-solving, logical thinking.
    Aptitude,
    /// Assesses behavioural style and working qualities: how someone
    /// works best, team compatibility, introversion/extraversion.
    Personality,
    /// Spans aptitude **and** personality: behavioural styles, emotional
    /// intelligence, cognitive abilities.
    Psychometric,
    /// Used during hiring to evaluate suitability for a role: job
    /// simulations, skills assessments, judgement tests.
    Selection,
}

impl AssessmentCategory {
    /// Every category, in declaration order — the closed vocabulary the
    /// validator and the profile view iterate.
    pub const ALL: [Self; 4] = [
        Self::Aptitude,
        Self::Personality,
        Self::Psychometric,
        Self::Selection,
    ];

    /// The `snake_case` wire token (the stored/persisted form).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aptitude => "aptitude",
            Self::Personality => "personality",
            Self::Psychometric => "psychometric",
            Self::Selection => "selection",
        }
    }

    /// Parse a wire token back to a category; `None` when unknown.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == token)
    }

    /// The scales this category **owns** — the ones whose
    /// [`AssessmentScale::category`] is this category.
    #[must_use]
    pub fn own_scales(self) -> &'static [AssessmentScale] {
        use AssessmentScale as S;
        match self {
            Self::Aptitude => &[
                S::NumericalReasoning,
                S::VerbalReasoning,
                S::ProblemSolving,
                S::LogicalThinking,
            ],
            Self::Personality => &[
                S::WorkStyle,
                S::TeamCompatibility,
                S::IntroversionExtraversion,
            ],
            Self::Psychometric => &[
                S::BehaviouralStyle,
                S::EmotionalIntelligence,
                S::CognitiveAbility,
            ],
            Self::Selection => &[S::JobSimulation, S::SkillsAssessment, S::JudgementTest],
        }
    }

    /// Whether `scale` may be reported on an assessment of this category.
    ///
    /// A category always permits its own scales. [`Psychometric`](Self::Psychometric)
    /// additionally permits every aptitude and personality scale, because a
    /// psychometric test by definition covers both.
    #[must_use]
    pub fn permits(self, scale: AssessmentScale) -> bool {
        let home = scale.category();
        home == self
            || (self == Self::Psychometric && matches!(home, Self::Aptitude | Self::Personality))
    }
}

/// Renders the `snake_case` wire token, matching the serde form.
impl std::fmt::Display for AssessmentCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One measured dimension of an assessment.
///
/// Serializes in `snake_case` (`"numerical_reasoning"`, `"work_style"`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentScale {
    /// Reasoning with numbers, data, and quantitative relationships.
    NumericalReasoning,
    /// Reasoning with written information, comprehension, and inference.
    VerbalReasoning,
    /// Working a novel problem through to a solution.
    ProblemSolving,
    /// Deductive / inductive reasoning over abstract rules.
    LogicalThinking,
    /// How the person works best (pace, structure, autonomy).
    WorkStyle,
    /// Fit with, and contribution to, a team.
    TeamCompatibility,
    /// Where the person sits on the introversion–extraversion continuum.
    IntroversionExtraversion,
    /// Characteristic behavioural style (e.g. a DISC-like profile).
    BehaviouralStyle,
    /// Recognising and managing one's own and others' emotions.
    EmotionalIntelligence,
    /// General cognitive ability / mental processing.
    CognitiveAbility,
    /// Performance in a simulated sample of the actual job.
    JobSimulation,
    /// Demonstrated proficiency in a role-specific skill.
    SkillsAssessment,
    /// Situational judgement — the choice made in a work scenario.
    JudgementTest,
}

impl AssessmentScale {
    /// Every scale, in declaration order.
    pub const ALL: [Self; 13] = [
        Self::NumericalReasoning,
        Self::VerbalReasoning,
        Self::ProblemSolving,
        Self::LogicalThinking,
        Self::WorkStyle,
        Self::TeamCompatibility,
        Self::IntroversionExtraversion,
        Self::BehaviouralStyle,
        Self::EmotionalIntelligence,
        Self::CognitiveAbility,
        Self::JobSimulation,
        Self::SkillsAssessment,
        Self::JudgementTest,
    ];

    /// The category this scale belongs to.
    #[must_use]
    pub fn category(self) -> AssessmentCategory {
        match self {
            Self::NumericalReasoning
            | Self::VerbalReasoning
            | Self::ProblemSolving
            | Self::LogicalThinking => AssessmentCategory::Aptitude,
            Self::WorkStyle | Self::TeamCompatibility | Self::IntroversionExtraversion => {
                AssessmentCategory::Personality
            }
            Self::BehaviouralStyle | Self::EmotionalIntelligence | Self::CognitiveAbility => {
                AssessmentCategory::Psychometric
            }
            Self::JobSimulation | Self::SkillsAssessment | Self::JudgementTest => {
                AssessmentCategory::Selection
            }
        }
    }

    /// The `snake_case` wire token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NumericalReasoning => "numerical_reasoning",
            Self::VerbalReasoning => "verbal_reasoning",
            Self::ProblemSolving => "problem_solving",
            Self::LogicalThinking => "logical_thinking",
            Self::WorkStyle => "work_style",
            Self::TeamCompatibility => "team_compatibility",
            Self::IntroversionExtraversion => "introversion_extraversion",
            Self::BehaviouralStyle => "behavioural_style",
            Self::EmotionalIntelligence => "emotional_intelligence",
            Self::CognitiveAbility => "cognitive_ability",
            Self::JobSimulation => "job_simulation",
            Self::SkillsAssessment => "skills_assessment",
            Self::JudgementTest => "judgement_test",
        }
    }

    /// Parse a wire token back to a scale; `None` when unknown.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.as_str() == token)
    }
}

/// Renders the `snake_case` wire token, matching the serde form.
impl std::fmt::Display for AssessmentScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The interpreted band a percentile falls in — the coarse, shareable
/// reading of a score. Serializes in `snake_case`.
///
/// Bands follow the conventional norm-referenced split: bottom decile
/// low, next fifth below average, the middle two-fifths average, the
/// next fifth above average, top decile high.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScoreBand {
    /// Percentile below 10.
    Low,
    /// Percentile in `[10, 30)`.
    BelowAverage,
    /// Percentile in `[30, 70)`.
    Average,
    /// Percentile in `[70, 90)`.
    AboveAverage,
    /// Percentile at or above 90.
    High,
}

impl ScoreBand {
    /// The band a percentile in `[0, 100]` falls in. Values outside the
    /// range are clamped, so an out-of-range percentile still yields a
    /// definite band rather than panicking (validation rejects it at the
    /// boundary — this is the defensive fallback).
    #[must_use]
    pub fn from_percentile(percentile: f64) -> Self {
        let p = percentile.clamp(0.0, 100.0);
        if p < 10.0 {
            Self::Low
        } else if p < 30.0 {
            Self::BelowAverage
        } else if p < 70.0 {
            Self::Average
        } else if p < 90.0 {
            Self::AboveAverage
        } else {
            Self::High
        }
    }

    /// The `snake_case` wire token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::BelowAverage => "below_average",
            Self::Average => "average",
            Self::AboveAverage => "above_average",
            Self::High => "high",
        }
    }
}

/// Renders the `snake_case` wire token, matching the serde form.
impl std::fmt::Display for ScoreBand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where an assessment is in its lifecycle. Serializes in `snake_case`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    /// Booked but not started.
    Scheduled,
    /// The worker has started the test.
    InProgress,
    /// Finished and scored.
    Completed,
    /// Completed but past its validity date — results no longer count.
    Expired,
    /// Abandoned before completion.
    Cancelled,
}

impl AssessmentStatus {
    /// Every status, in declaration order.
    pub const ALL: [Self; 5] = [
        Self::Scheduled,
        Self::InProgress,
        Self::Completed,
        Self::Expired,
        Self::Cancelled,
    ];

    /// The `snake_case` wire token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse a wire token back to a status; `None` when unknown.
    #[must_use]
    pub fn from_token(token: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.as_str() == token)
    }

    /// Whether this is a terminal state (no further transition).
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Expired | Self::Cancelled)
    }

    /// Whether `self` → `to` is a legal lifecycle move.
    ///
    /// `scheduled → in_progress → completed → expired`, with `cancelled`
    /// reachable from any non-terminal state. Terminal states
    /// ([`Expired`](Self::Expired), [`Cancelled`](Self::Cancelled)) do not
    /// transition, and a status never moves to itself.
    #[must_use]
    pub fn can_transition_to(self, to: Self) -> bool {
        matches!(
            (self, to),
            (
                Self::Scheduled,
                Self::InProgress | Self::Completed | Self::Cancelled
            ) | (Self::InProgress, Self::Completed | Self::Cancelled)
                | (Self::Completed, Self::Expired)
        )
    }
}

/// Renders the `snake_case` wire token, matching the serde form.
impl std::fmt::Display for AssessmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One scale's outcome within an assessment.
///
/// Every score field is optional because instruments report differently:
/// some give a raw score out of a maximum, some a norm-referenced
/// percentile, some only a qualitative band (a behavioural-style profile
/// has no "score"). [`effective_band`](Self::effective_band) reads the
/// explicit band when present and otherwise derives one from the
/// percentile.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssessmentResult {
    /// The dimension measured.
    pub scale: AssessmentScale,
    /// Raw score as reported by the instrument.
    #[serde(default)]
    pub raw_score: Option<f64>,
    /// The maximum raw score obtainable (the denominator for `raw_score`).
    #[serde(default)]
    pub max_score: Option<f64>,
    /// Norm-referenced percentile in `[0, 100]`.
    #[serde(default)]
    pub percentile: Option<f64>,
    /// Explicit band, when the instrument reports one directly.
    #[serde(default)]
    pub band: Option<ScoreBand>,
    /// Free-text interpretation from the report ("prefers structured
    /// work", "strong situational judgement under time pressure", …).
    #[serde(default)]
    pub narrative: Option<String>,
}

impl AssessmentResult {
    /// A result carrying only a scale (no scores yet) — the shape an
    /// in-progress assessment holds.
    #[must_use]
    pub fn new(scale: AssessmentScale) -> Self {
        Self {
            scale,
            raw_score: None,
            max_score: None,
            percentile: None,
            band: None,
            narrative: None,
        }
    }

    /// A percentile-scored result, with its band derived.
    #[must_use]
    pub fn percentile(scale: AssessmentScale, percentile: f64) -> Self {
        Self {
            scale,
            raw_score: None,
            max_score: None,
            percentile: Some(percentile),
            band: Some(ScoreBand::from_percentile(percentile)),
            narrative: None,
        }
    }

    /// The band to report: the explicit [`band`](Self::band) when set,
    /// otherwise one derived from [`percentile`](Self::percentile).
    /// `None` when the result carries neither.
    #[must_use]
    pub fn effective_band(&self) -> Option<ScoreBand> {
        self.band
            .or_else(|| self.percentile.map(ScoreBand::from_percentile))
    }

    /// The redacted projection: the scale and the interpreted band
    /// survive; the raw score, maximum, percentile, and narrative are
    /// dropped. Used under the ABAC `mask` obligation so a caller can see
    /// *that* a dimension was measured, and roughly where it landed,
    /// without reading the profile itself.
    #[must_use]
    pub fn masked(&self) -> Self {
        Self {
            scale: self.scale,
            raw_score: None,
            max_score: None,
            percentile: None,
            band: self.effective_band(),
            narrative: None,
        }
    }
}

/// One administration of one assessment instrument to one worker.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Assessment {
    /// Unique assessment identifier.
    pub id: Uuid,
    /// The worker assessed.
    pub worker_id: Uuid,
    /// The family of test.
    pub category: AssessmentCategory,
    /// The instrument's name ("Watson-Glaser III", "SHL Verify G+", …).
    pub instrument: String,
    /// The test publisher / administering provider, when recorded.
    #[serde(default)]
    pub provider: Option<String>,
    /// Lifecycle status.
    pub status: AssessmentStatus,
    /// The date the assessment was taken.
    #[serde(default)]
    pub administered_on: Option<NaiveDate>,
    /// The date the results stop being treated as current.
    #[serde(default)]
    pub expires_on: Option<NaiveDate>,
    /// Who administered it (an operator or system identity).
    #[serde(default)]
    pub administered_by: Option<String>,
    /// Operator notes about the administration (conditions, adjustments).
    #[serde(default)]
    pub notes: Option<String>,
    /// Per-scale outcomes.
    #[serde(default)]
    pub results: Vec<AssessmentResult>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last-update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Assessment {
    /// Creates a scheduled assessment with a fresh v4 [`Uuid`] and
    /// creation/update timestamps set to now. Scores arrive later.
    ///
    /// # Examples
    ///
    /// ```
    /// use worker_service::models::assessment::{Assessment, AssessmentCategory, AssessmentStatus};
    /// use uuid::Uuid;
    ///
    /// let a = Assessment::new(Uuid::new_v4(), AssessmentCategory::Selection, "Work sample: triage");
    /// assert_eq!(a.status, AssessmentStatus::Scheduled);
    /// assert!(a.results.is_empty());
    /// ```
    #[must_use]
    pub fn new(
        worker_id: Uuid,
        category: AssessmentCategory,
        instrument: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            worker_id,
            category,
            instrument: instrument.into(),
            provider: None,
            status: AssessmentStatus::Scheduled,
            administered_on: None,
            expires_on: None,
            administered_by: None,
            notes: None,
            results: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Whether the assessment's results count as current on `date`:
    /// [`Completed`](AssessmentStatus::Completed), and either open-ended
    /// or not yet past [`expires_on`](Self::expires_on).
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::NaiveDate;
    /// use worker_service::models::assessment::{Assessment, AssessmentCategory, AssessmentStatus};
    /// use uuid::Uuid;
    ///
    /// let mut a = Assessment::new(Uuid::new_v4(), AssessmentCategory::Aptitude, "SHL Verify");
    /// let day = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();
    /// // Scheduled results are not yet current.
    /// assert!(!a.is_valid_on(day(2026, 7, 23)));
    ///
    /// a.status = AssessmentStatus::Completed;
    /// a.expires_on = Some(day(2027, 1, 1));
    /// assert!(a.is_valid_on(day(2026, 7, 23)));
    /// assert!(!a.is_valid_on(day(2027, 6, 1)));
    /// ```
    #[must_use]
    pub fn is_valid_on(&self, date: NaiveDate) -> bool {
        self.status == AssessmentStatus::Completed
            && self.expires_on.is_none_or(|expiry| date <= expiry)
    }

    /// The mean percentile across the results that carry one; `None`
    /// when no result is percentile-scored. Used as the headline figure
    /// on the selection-suitability view — an average of real scores
    /// only, never interpolated from bands.
    #[must_use]
    pub fn mean_percentile(&self) -> Option<f64> {
        let scored: Vec<f64> = self.results.iter().filter_map(|r| r.percentile).collect();
        if scored.is_empty() {
            return None;
        }
        #[allow(clippy::cast_precision_loss)] // result counts are tiny
        let n = scored.len() as f64;
        Some(scored.iter().sum::<f64>() / n)
    }

    /// The redacted projection returned under the ABAC `mask`
    /// obligation: each result is [`masked`](AssessmentResult::masked)
    /// and the operator notes are dropped. The instrument, category,
    /// status, and dates survive so a masked caller still learns that
    /// the assessment happened.
    #[must_use]
    pub fn masked(&self) -> Self {
        Self {
            results: self.results.iter().map(AssessmentResult::masked).collect(),
            notes: None,
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every scale maps to exactly the category that declares it in
    /// `own_scales` — the two directions of the mapping agree.
    #[test]
    fn scale_category_mapping_is_consistent() {
        for category in AssessmentCategory::ALL {
            for scale in category.own_scales() {
                assert_eq!(
                    scale.category(),
                    category,
                    "{scale} should belong to {category}"
                );
            }
        }
        // Every scale is owned by exactly one category.
        for scale in AssessmentScale::ALL {
            let owners: Vec<_> = AssessmentCategory::ALL
                .into_iter()
                .filter(|c| c.own_scales().contains(&scale))
                .collect();
            assert_eq!(
                owners.len(),
                1,
                "{scale} must have exactly one home category"
            );
        }
    }

    /// Psychometric spans aptitude and personality; the other categories
    /// accept only their own scales.
    #[test]
    fn psychometric_spans_aptitude_and_personality() {
        use AssessmentCategory as C;
        use AssessmentScale as S;

        // Its own scales.
        assert!(C::Psychometric.permits(S::EmotionalIntelligence));
        // Aptitude and personality scales are also in scope.
        assert!(C::Psychometric.permits(S::NumericalReasoning));
        assert!(C::Psychometric.permits(S::TeamCompatibility));
        // Selection scales are not.
        assert!(!C::Psychometric.permits(S::JobSimulation));

        // The narrower categories stay narrow.
        assert!(C::Aptitude.permits(S::VerbalReasoning));
        assert!(!C::Aptitude.permits(S::WorkStyle));
        assert!(!C::Personality.permits(S::CognitiveAbility));
        assert!(C::Selection.permits(S::JudgementTest));
        assert!(!C::Selection.permits(S::LogicalThinking));
    }

    /// Percentile → band, including the boundaries and out-of-range
    /// clamping.
    #[test]
    fn bands_split_the_percentile_range() {
        assert_eq!(ScoreBand::from_percentile(0.0), ScoreBand::Low);
        assert_eq!(ScoreBand::from_percentile(9.9), ScoreBand::Low);
        assert_eq!(ScoreBand::from_percentile(10.0), ScoreBand::BelowAverage);
        assert_eq!(ScoreBand::from_percentile(29.9), ScoreBand::BelowAverage);
        assert_eq!(ScoreBand::from_percentile(30.0), ScoreBand::Average);
        assert_eq!(ScoreBand::from_percentile(69.9), ScoreBand::Average);
        assert_eq!(ScoreBand::from_percentile(70.0), ScoreBand::AboveAverage);
        assert_eq!(ScoreBand::from_percentile(89.9), ScoreBand::AboveAverage);
        assert_eq!(ScoreBand::from_percentile(90.0), ScoreBand::High);
        assert_eq!(ScoreBand::from_percentile(100.0), ScoreBand::High);
        // Out-of-range values clamp rather than panic.
        assert_eq!(ScoreBand::from_percentile(-5.0), ScoreBand::Low);
        assert_eq!(ScoreBand::from_percentile(150.0), ScoreBand::High);
    }

    /// The lifecycle: forward moves only, cancel from any open state,
    /// terminal states are stuck, and no self-transition.
    #[test]
    fn status_lifecycle() {
        use AssessmentStatus as S;

        assert!(S::Scheduled.can_transition_to(S::InProgress));
        assert!(S::InProgress.can_transition_to(S::Completed));
        assert!(S::Completed.can_transition_to(S::Expired));
        // A short test may be recorded straight to completed.
        assert!(S::Scheduled.can_transition_to(S::Completed));
        // Cancel from any open state.
        assert!(S::Scheduled.can_transition_to(S::Cancelled));
        assert!(S::InProgress.can_transition_to(S::Cancelled));

        // No going backwards.
        assert!(!S::Completed.can_transition_to(S::InProgress));
        assert!(!S::InProgress.can_transition_to(S::Scheduled));
        // Terminal states are terminal.
        assert!(S::Expired.is_terminal() && S::Cancelled.is_terminal());
        for to in S::ALL {
            assert!(!S::Expired.can_transition_to(to));
            assert!(!S::Cancelled.can_transition_to(to));
        }
        // Never to itself.
        for status in S::ALL {
            assert!(!status.can_transition_to(status), "{status} → {status}");
        }
    }

    /// `effective_band` prefers the explicit band and falls back to the
    /// percentile; with neither it is `None`.
    #[test]
    fn effective_band_prefers_the_explicit_band() {
        let mut r = AssessmentResult::new(AssessmentScale::BehaviouralStyle);
        assert_eq!(r.effective_band(), None);

        r.percentile = Some(95.0);
        assert_eq!(r.effective_band(), Some(ScoreBand::High));

        r.band = Some(ScoreBand::Average);
        assert_eq!(
            r.effective_band(),
            Some(ScoreBand::Average),
            "an explicitly reported band wins"
        );
    }

    /// Validity requires completion and respects the expiry date.
    #[test]
    fn validity_requires_completion_and_a_live_expiry() {
        let day = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).expect("valid date");
        let mut a = Assessment::new(
            Uuid::new_v4(),
            AssessmentCategory::Psychometric,
            "Hogan HPI",
        );
        a.administered_on = Some(day(2026, 1, 10));

        // Not completed ⇒ not valid.
        assert!(!a.is_valid_on(day(2026, 7, 23)));

        a.status = AssessmentStatus::Completed;
        // Completed with no expiry ⇒ valid indefinitely.
        assert!(a.is_valid_on(day(2030, 1, 1)));

        a.expires_on = Some(day(2027, 1, 10));
        assert!(a.is_valid_on(day(2027, 1, 10)), "valid on the expiry date");
        assert!(!a.is_valid_on(day(2027, 1, 11)), "not valid the day after");

        // An explicitly expired record is never valid.
        a.status = AssessmentStatus::Expired;
        assert!(!a.is_valid_on(day(2026, 7, 23)));
    }

    /// `mean_percentile` averages only percentile-scored results, and is
    /// `None` when none carry a percentile.
    #[test]
    fn mean_percentile_averages_real_scores_only() {
        let mut a = Assessment::new(Uuid::new_v4(), AssessmentCategory::Selection, "Work sample");
        assert_eq!(a.mean_percentile(), None);

        a.results.push(AssessmentResult::percentile(
            AssessmentScale::JobSimulation,
            80.0,
        ));
        a.results.push(AssessmentResult::percentile(
            AssessmentScale::JudgementTest,
            60.0,
        ));
        // A band-only result contributes nothing to the mean.
        let mut band_only = AssessmentResult::new(AssessmentScale::SkillsAssessment);
        band_only.band = Some(ScoreBand::High);
        a.results.push(band_only);

        let mean = a.mean_percentile().expect("two scored results");
        assert!(
            (mean - 70.0).abs() < f64::EPSILON,
            "mean of 80 and 60 is 70, got {mean}"
        );
    }

    /// Masking drops scores, percentiles, narratives, and operator notes
    /// while keeping the band and the fact of the assessment.
    #[test]
    fn masking_keeps_the_band_and_drops_the_profile() {
        let mut a = Assessment::new(
            Uuid::new_v4(),
            AssessmentCategory::Personality,
            "Big Five Inventory",
        );
        a.notes = Some("administered with extra time".to_string());
        a.results.push(AssessmentResult {
            scale: AssessmentScale::IntroversionExtraversion,
            raw_score: Some(41.0),
            max_score: Some(50.0),
            percentile: Some(88.0),
            band: None,
            narrative: Some("strongly extraverted".to_string()),
        });

        let masked = a.masked();
        assert_eq!(masked.instrument, "Big Five Inventory");
        assert_eq!(masked.category, AssessmentCategory::Personality);
        assert!(masked.notes.is_none(), "operator notes are redacted");

        let r = &masked.results[0];
        assert_eq!(r.scale, AssessmentScale::IntroversionExtraversion);
        assert_eq!(r.band, Some(ScoreBand::AboveAverage), "band survives");
        assert!(r.raw_score.is_none() && r.max_score.is_none());
        assert!(r.percentile.is_none(), "the percentile is redacted");
        assert!(r.narrative.is_none(), "the narrative is redacted");
    }

    /// Wire tokens round-trip through the `from_token` parsers and match
    /// the serde representation.
    #[test]
    fn tokens_round_trip() {
        for category in AssessmentCategory::ALL {
            assert_eq!(
                AssessmentCategory::from_token(category.as_str()),
                Some(category)
            );
            let json = serde_json::to_string(&category).expect("serialize");
            assert_eq!(json, format!("\"{category}\""));
        }
        for scale in AssessmentScale::ALL {
            assert_eq!(AssessmentScale::from_token(scale.as_str()), Some(scale));
            let json = serde_json::to_string(&scale).expect("serialize");
            assert_eq!(json, format!("\"{scale}\""));
        }
        for status in AssessmentStatus::ALL {
            assert_eq!(AssessmentStatus::from_token(status.as_str()), Some(status));
            let json = serde_json::to_string(&status).expect("serialize");
            assert_eq!(json, format!("\"{status}\""));
        }
        assert_eq!(AssessmentCategory::from_token("astrology"), None);
        assert_eq!(AssessmentScale::from_token("vibes"), None);
        assert_eq!(AssessmentStatus::from_token("maybe"), None);
    }

    /// A full assessment survives a JSON round-trip unchanged.
    #[test]
    fn assessment_serialization_roundtrip() {
        let mut a = Assessment::new(
            Uuid::new_v4(),
            AssessmentCategory::Aptitude,
            "SHL Verify G+",
        );
        a.provider = Some("SHL".to_string());
        a.status = AssessmentStatus::Completed;
        a.administered_on = NaiveDate::from_ymd_opt(2026, 5, 4);
        a.expires_on = NaiveDate::from_ymd_opt(2028, 5, 4);
        a.results.push(AssessmentResult::percentile(
            AssessmentScale::NumericalReasoning,
            74.5,
        ));

        let json = serde_json::to_string(&a).expect("serialize");
        let back: Assessment = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.id, a.id);
        assert_eq!(back.category, AssessmentCategory::Aptitude);
        assert_eq!(back.status, AssessmentStatus::Completed);
        assert_eq!(back.administered_on, a.administered_on);
        assert_eq!(back.results.len(), 1);
        assert_eq!(back.results[0].scale, AssessmentScale::NumericalReasoning);
        assert_eq!(back.results[0].band, Some(ScoreBand::AboveAverage));
    }
}
