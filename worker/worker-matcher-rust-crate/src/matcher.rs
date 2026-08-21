//! Worker matcher engine: deterministic and probabilistic algorithms.
//!
//! This is the orchestration layer of the crate. It pulls together the data
//! types from [`crate::models`], the text transformations from
//! [`crate::normalizer`], and the similarity primitives from
//! [`crate::scorer`] to produce a single answer about whether two worker
//! records refer to the same individual.
//!
//! ## Two strategies, one engine
//!
//! - [`MatchingEngine::deterministic_match`] — fast, binary, defensible.
//!   Returns `true` iff either both NHS numbers parse to the same value or
//!   the normalised name + DOB + gender all match exactly.
//! - [`MatchingEngine::match_workers`] — weighted probabilistic scoring,
//!   returning a [`MatchResult`] with per-field [`MatchBreakdown`].
//!
//! ## Example
//!
//! ```
//! use worker_matcher::{MatchingEngine, Worker};
//! use chrono::NaiveDate;
//!
//! let a = Worker::builder()
//!     .given_name("John")
//!     .family_name("Smith")
//!     .date_of_birth(NaiveDate::from_ymd_opt(1980, 5, 15).unwrap())
//!     .build();
//!
//! let b = Worker::builder()
//!     .given_name("Jon")          // typo
//!     .family_name("Smith")
//!     .date_of_birth(NaiveDate::from_ymd_opt(1980, 5, 15).unwrap())
//!     .build();
//!
//! let engine = MatchingEngine::default_config();
//! let result = engine.match_workers(&a, &b);
//! assert!(result.is_match);
//! ```

use crate::identifiers;
use crate::models::{Address, PassportBook, Worker};
use crate::nicknames::NicknameTable;
use crate::normalizer::Normalizer;
use crate::scorer::{Scorer, SimilarityAlgorithm};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

/// Tunable configuration for the matching engine.
///
/// All weights are dimensionless and contribute to a renormalised weighted
/// sum — they do not need to add to `1.0`. The matching pipeline divides the
/// weighted sum by the sum of *participating* weights so that missing fields
/// neither contribute nor penalise. The score is then compared against
/// [`MatchConfig::match_threshold`] to produce the `is_match` boolean.
///
/// Two presets cover most needs:
///
/// - [`MatchConfig::strict`]  — `match_threshold = 0.95`, `strict_mode = true`.
/// - [`MatchConfig::lenient`] — `match_threshold = 0.75`, phonetic on.
///
/// # Example
///
/// ```
/// use worker_matcher::{MatchConfig, SimilarityAlgorithm};
///
/// let custom = MatchConfig {
///     match_threshold: 0.80,
///     uk_nhs_number_weight: 0.30,
///     fr_nir_weight: 0.30,
///     es_tsi_weight: 0.30,
///     ie_ihi_weight: 0.30,
///     uk_hc_number_weight: 0.30,
///     us_ssn_weight: 0.30,
///     au_ihi_weight: 0.30,
///     de_kvnr_weight: 0.30,
///     it_cf_weight: 0.30,
///     nl_bsn_weight: 0.30,
///     se_personnummer_weight: 0.30,
///     uk_chi_number_weight: 0.30,
///     be_nn_weight: 0.30,
///     bg_egn_weight: 0.30,
///     cz_rc_weight: 0.30,
///     dk_cpr_weight: 0.30,
///     ee_ik_weight: 0.30,
///     es_dni_weight: 0.30,
///     fi_hetu_weight: 0.30,
///     hr_oib_weight: 0.30,
///     is_kt_weight: 0.30,
///     lt_ak_weight: 0.30,
///     lv_pk_weight: 0.30,
///     mt_id_weight: 0.30,
///     no_fnr_weight: 0.30,
///     pl_pesel_weight: 0.30,
///     ro_cnp_weight: 0.30,
///     si_emso_weight: 0.30,
///     sk_rc_weight: 0.30,
///     uk_nino_weight: 0.30,
///     gr_dss_weight: 0.30,
///     li_id_weight: 0.30,
///     nl_id_weight: 0.30,
///     pl_nip_weight: 0.30,
///     pt_nif_weight: 0.30,
///     br_cpf_weight: 0.30,
///     cn_rrn_weight: 0.30,
///     in_aadhaar_weight: 0.30,
///     jp_my_number_weight: 0.30,
///     mx_curp_weight: 0.30,
///     nz_nhi_weight: 0.30,
///     za_id_weight: 0.30,
///     passport_book_weight: 0.30,
///     given_name_weight: 0.15,
///     family_name_weight: 0.20,
///     date_of_birth_weight: 0.15,
///     gender_weight: 0.05,
///     blood_type_weight: 0.05,
///     multiple_birth_weight: 0.05,
///     address_weight: 0.025,
///     birth_place_weight: 0.05,
///     death_date_weight: 0.10,
///     death_place_weight: 0.05,
///     phone_weight: 0.025,
///     email_weight: 0.05,
///     use_phonetic_matching: true,
///     name_algorithm: SimilarityAlgorithm::JaroWinkler,
///     strict_mode: false,
///     nickname_table: worker_matcher::NicknameTable::empty(),
///     gmail_dot_folding: false,
///     phone_default_country: Some("GB".into()),
/// };
/// assert_eq!(custom.match_threshold, 0.80);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MatchConfig {
    /// Threshold score for considering two workers a match (`0.0..=1.0`).
    pub match_threshold: f64,

    /// Weight for UK NHS Number match (only contributes if both parse).
    pub uk_nhs_number_weight: f64,

    /// Weight for France NIR match (only contributes if both parse).
    pub fr_nir_weight: f64,

    /// Weight for España TSI / CIP-SNS match (only contributes if both parse).
    pub es_tsi_weight: f64,

    /// Weight for Éire IHI match (only contributes if both parse).
    pub ie_ihi_weight: f64,

    /// Weight for United Kingdom Northern Ireland H&C Number match (only contributes if both parse).
    pub uk_hc_number_weight: f64,

    /// Weight for United States Social Security Number match (only contributes if both parse).
    pub us_ssn_weight: f64,

    /// Weight for Australia IHI match (only contributes if both parse).
    pub au_ihi_weight: f64,

    /// Weight for Germany KVNR match (only contributes if both parse).
    pub de_kvnr_weight: f64,

    /// Weight for Italy *Codice Fiscale* match (only contributes if both parse).
    pub it_cf_weight: f64,

    /// Weight for Netherlands BSN match (only contributes if both parse).
    pub nl_bsn_weight: f64,

    /// Weight for Sweden *Personnummer* match (only contributes if both parse).
    pub se_personnummer_weight: f64,

    /// Weight for United Kingdom (Scotland) CHI Number match (only contributes if both parse).
    pub uk_chi_number_weight: f64,

    /// Weight for Belgium National Number match (only contributes if both parse).
    pub be_nn_weight: f64,
    /// Weight for Bulgaria EGN match (only contributes if both parse).
    pub bg_egn_weight: f64,
    /// Weight for Czech *Rodné číslo* match (only contributes if both parse).
    pub cz_rc_weight: f64,
    /// Weight for Denmark CPR match (only contributes if both parse).
    pub dk_cpr_weight: f64,
    /// Weight for Estonia *Isikukood* match (only contributes if both parse).
    pub ee_ik_weight: f64,
    /// Weight for Spain DNI / NIE match (only contributes if both parse).
    pub es_dni_weight: f64,
    /// Weight for Finland HETU match (only contributes if both parse).
    pub fi_hetu_weight: f64,
    /// Weight for Croatia OIB match (only contributes if both parse).
    pub hr_oib_weight: f64,
    /// Weight for Iceland *Kennitala* match (only contributes if both parse).
    pub is_kt_weight: f64,
    /// Weight for Lithuania *Asmens kodas* match (only contributes if both parse).
    pub lt_ak_weight: f64,
    /// Weight for Latvia *Personas kods* match (only contributes if both parse).
    pub lv_pk_weight: f64,
    /// Weight for Malta National ID match (only contributes if both parse).
    pub mt_id_weight: f64,
    /// Weight for Norway *Fødselsnummer* match (only contributes if both parse).
    pub no_fnr_weight: f64,
    /// Weight for Poland PESEL match (only contributes if both parse).
    pub pl_pesel_weight: f64,
    /// Weight for Romania CNP match (only contributes if both parse).
    pub ro_cnp_weight: f64,
    /// Weight for Slovenia EMŠO match (only contributes if both parse).
    pub si_emso_weight: f64,
    /// Weight for Slovakia *Rodné číslo* match (only contributes if both parse).
    pub sk_rc_weight: f64,
    /// Weight for UK NINO match (only contributes if both parse).
    pub uk_nino_weight: f64,
    /// Weight for Greece DSS investor-share match (only contributes if both parse).
    pub gr_dss_weight: f64,
    /// Weight for Liechtenstein National ID match (only contributes if both parse).
    pub li_id_weight: f64,
    /// Weight for Netherlands National ID match (only contributes if both parse).
    pub nl_id_weight: f64,
    /// Weight for Poland NIP match (only contributes if both parse).
    pub pl_nip_weight: f64,
    /// Weight for Portugal NIF match (only contributes if both parse).
    pub pt_nif_weight: f64,
    /// Weight for Brazil CPF match (only contributes if both parse).
    pub br_cpf_weight: f64,
    /// Weight for China Resident Identity Card match (only contributes if both parse).
    pub cn_rrn_weight: f64,
    /// Weight for India Aadhaar match (only contributes if both parse).
    pub in_aadhaar_weight: f64,
    /// Weight for Japan My Number match (only contributes if both parse).
    pub jp_my_number_weight: f64,
    /// Weight for Mexico CURP match (only contributes if both parse).
    pub mx_curp_weight: f64,
    /// Weight for New Zealand NHI match (only contributes if both parse).
    pub nz_nhi_weight: f64,
    /// Weight for South Africa ID Number match (only contributes if both parse).
    pub za_id_weight: f64,

    /// Weight for passport-book match (contributes when both workers
    /// have at least one [`crate::PassportBook`] recorded). See spec
    /// §6.4a / FR-51.
    pub passport_book_weight: f64,

    /// Weight for given-name similarity.
    pub given_name_weight: f64,

    /// Weight for family-name similarity.
    pub family_name_weight: f64,

    /// Weight for date-of-birth exact match.
    pub date_of_birth_weight: f64,

    /// Weight for gender exact match.
    pub gender_weight: f64,

    /// Weight for ABO+RhD blood-type exact match (see [`crate::BloodType`]).
    /// Defaults to `0.05` — blood type is a weak positive signal but a
    /// strong negative signal (disagreement is reliable evidence of
    /// non-match because blood type doesn't change over a lifetime).
    pub blood_type_weight: f64,

    /// Weight for multiple-birth indicator exact match (FHIR
    /// `Patient.multipleBirth`). Defaults to `0.05` — a weak positive
    /// signal in general, but a strong negative signal for
    /// distinguishing identical twins who otherwise share name, DOB,
    /// and address.
    pub multiple_birth_weight: f64,

    /// Weight for address similarity.
    pub address_weight: f64,

    /// Weight for place-of-birth match (FHIR `Patient.birthPlace`).
    /// Defaults to `0.05` — stable for life so disagreement is
    /// informative, but agreement alone is weak because many people
    /// are born in the same place. Scored against the `city` and
    /// `country` sub-fields of [`crate::Address`].
    pub birth_place_weight: f64,

    /// Weight for date-of-death match (FHIR
    /// `Patient.deceasedDateTime`). Defaults to `0.10` — exact
    /// agreement on a recorded death date is strong evidence the
    /// records refer to the same worker, scored with the same DOB
    /// transposition heuristic as [`Self::date_of_birth_weight`].
    pub death_date_weight: f64,

    /// Weight for place-of-death match. Defaults to `0.05` — analogous
    /// to [`Self::birth_place_weight`]: stable per record (someone only
    /// dies once) so disagreement is informative, but agreement alone
    /// is weak. Scored against the `city` and `country` sub-fields of
    /// [`crate::Address`].
    pub death_place_weight: f64,

    /// Weight for phone-number exact match (after normalisation).
    pub phone_weight: f64,

    /// Weight for email-address exact match (after normalisation via
    /// [`crate::Normalizer::normalize_email`]).
    pub email_weight: f64,

    /// Whether to add a phonetic-name bonus when both names sound alike.
    pub use_phonetic_matching: bool,

    /// Similarity algorithm to use when comparing given and family names.
    pub name_algorithm: SimilarityAlgorithm,

    /// Reserved flag for stricter deterministic enforcement. See spec OQ-5.
    pub strict_mode: bool,

    /// Fold Gmail-specific localpart dots and `+tag` suffixes during
    /// email normalisation. When `true`, addresses on `gmail.com` /
    /// `googlemail.com` have every `.` in the localpart removed and any
    /// `+anything` suffix dropped before comparison, mirroring Gmail's
    /// documented routing rules. Defaults to `false` so non-Gmail
    /// addresses are unaffected and the canonical form is unsurprising.
    pub gmail_dot_folding: bool,

    /// Optional table of nickname equivalence classes consulted by name
    /// scoring.
    ///
    /// When a name pair is equivalent under this table — e.g.
    /// `("Michael", "Mike")` — the matcher **lifts the given-name (and
    /// family-name) similarity score to at least `0.9`**, ensuring the
    /// table-driven equivalence is not undone by a low Jaro-Winkler /
    /// Levenshtein score on dissimilar forms. The boost never lowers a
    /// score.
    ///
    /// Defaults to [`NicknameTable::empty`] so existing behaviour is
    /// preserved. Opt in with [`NicknameTable::english`] (a built-in
    /// English-language dictionary) or build a custom table via
    /// [`NicknameTable::with_class`].
    pub nickname_table: NicknameTable,

    /// ISO 3166-1 alpha-2 country code applied to phone numbers that lack
    /// an explicit international marker (`+CC` or `00CC`).
    ///
    /// When `Some(cc)`, the matcher converts each phone to E.164 form via
    /// [`crate::Normalizer::normalize_phone_e164`] using `cc` as the
    /// fallback jurisdiction. Numbers from different countries with
    /// overlapping national digits will no longer collide. When `None`,
    /// only inputs carrying an explicit international marker reach E.164;
    /// every other comparison falls back to the legacy
    /// [`crate::Normalizer::normalize_phone`] form.
    ///
    /// Defaults to `Some("GB")` to preserve the crate's historical
    /// UK-centric behaviour. Set to the worker population's predominant
    /// jurisdiction in production deployments.
    pub phone_default_country: Option<String>,
}

impl Default for MatchConfig {
    /// Production-ready defaults tuned per spec §13.1.
    ///
    /// ```
    /// use worker_matcher::{MatchConfig, SimilarityAlgorithm};
    /// let c = MatchConfig::default();
    /// assert!((c.match_threshold - 0.85).abs() < 1e-9);
    /// assert!(c.use_phonetic_matching);
    /// assert!(matches!(c.name_algorithm, SimilarityAlgorithm::Combined));
    /// ```
    fn default() -> Self {
        Self {
            match_threshold: 0.85,
            uk_nhs_number_weight: 0.30,
            fr_nir_weight: 0.30,
            es_tsi_weight: 0.30,
            ie_ihi_weight: 0.30,
            uk_hc_number_weight: 0.30,
            us_ssn_weight: 0.30,
            au_ihi_weight: 0.30,
            de_kvnr_weight: 0.30,
            it_cf_weight: 0.30,
            nl_bsn_weight: 0.30,
            se_personnummer_weight: 0.30,
            uk_chi_number_weight: 0.30,
            be_nn_weight: 0.30,
            bg_egn_weight: 0.30,
            cz_rc_weight: 0.30,
            dk_cpr_weight: 0.30,
            ee_ik_weight: 0.30,
            es_dni_weight: 0.30,
            fi_hetu_weight: 0.30,
            hr_oib_weight: 0.30,
            is_kt_weight: 0.30,
            lt_ak_weight: 0.30,
            lv_pk_weight: 0.30,
            mt_id_weight: 0.30,
            no_fnr_weight: 0.30,
            pl_pesel_weight: 0.30,
            ro_cnp_weight: 0.30,
            si_emso_weight: 0.30,
            sk_rc_weight: 0.30,
            uk_nino_weight: 0.30,
            gr_dss_weight: 0.30,
            li_id_weight: 0.30,
            nl_id_weight: 0.30,
            pl_nip_weight: 0.30,
            pt_nif_weight: 0.30,
            br_cpf_weight: 0.30,
            cn_rrn_weight: 0.30,
            in_aadhaar_weight: 0.30,
            jp_my_number_weight: 0.30,
            mx_curp_weight: 0.30,
            nz_nhi_weight: 0.30,
            za_id_weight: 0.30,
            passport_book_weight: 0.30,
            given_name_weight: 0.15,
            family_name_weight: 0.20,
            date_of_birth_weight: 0.20,
            gender_weight: 0.05,
            blood_type_weight: 0.05,
            multiple_birth_weight: 0.05,
            address_weight: 0.05,
            birth_place_weight: 0.05,
            death_date_weight: 0.10,
            death_place_weight: 0.05,
            phone_weight: 0.05,
            email_weight: 0.05,
            use_phonetic_matching: true,
            name_algorithm: SimilarityAlgorithm::Combined,
            strict_mode: false,
            nickname_table: NicknameTable::empty(),
            gmail_dot_folding: false,
            phone_default_country: Some("GB".to_string()),
        }
    }
}

impl MatchConfig {
    /// A stricter preset: `match_threshold = 0.95`, `strict_mode = true`.
    ///
    /// Use when a clinician must rely on the answer and false positives are
    /// more dangerous than false negatives.
    ///
    /// ```
    /// use worker_matcher::MatchConfig;
    /// let c = MatchConfig::strict();
    /// assert!((c.match_threshold - 0.95).abs() < 1e-9);
    /// assert!(c.strict_mode);
    /// ```
    #[must_use]
    pub fn strict() -> Self {
        Self {
            match_threshold: 0.95,
            strict_mode: true,
            ..Default::default()
        }
    }

    /// A more forgiving preset: `match_threshold = 0.75`, phonetic matching on.
    ///
    /// Use when triaging large candidate sets where false negatives are
    /// worse than false positives.
    ///
    /// ```
    /// use worker_matcher::MatchConfig;
    /// let c = MatchConfig::lenient();
    /// assert!((c.match_threshold - 0.75).abs() < 1e-9);
    /// assert!(c.use_phonetic_matching);
    /// ```
    #[must_use]
    pub fn lenient() -> Self {
        Self {
            match_threshold: 0.75,
            use_phonetic_matching: true,
            ..Default::default()
        }
    }
}

/// Qualitative confidence band derived from the probabilistic
/// [`MatchResult::score`].
///
/// The bands are fixed across all `MatchConfig` presets — they do **not**
/// follow `match_threshold`. They are intended for triage UIs and audit
/// logs where a coarse High/Medium/Low summary is more useful than the
/// raw float. The `is_match` boolean remains the authoritative go/no-go
/// signal because it incorporates the configured threshold.
///
/// Boundaries (per spec §12.5):
///
/// | Score range | Band |
/// |---|---|
/// | `score >= 0.90` | `High` |
/// | `0.75 <= score < 0.90` | `Medium` |
/// | `score < 0.75` | `Low` |
///
/// # Examples
///
/// ```
/// use worker_matcher::Confidence;
///
/// assert_eq!(Confidence::from_score(0.99), Confidence::High);
/// assert_eq!(Confidence::from_score(0.90), Confidence::High);   // inclusive
/// assert_eq!(Confidence::from_score(0.85), Confidence::Medium);
/// assert_eq!(Confidence::from_score(0.75), Confidence::Medium); // inclusive
/// assert_eq!(Confidence::from_score(0.50), Confidence::Low);
/// assert_eq!(Confidence::from_score(0.00), Confidence::Low);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    /// Score is at or above `0.90`. Strong match; safe to act on with
    /// minimal review.
    High,
    /// Score is in `0.75..0.90`. Medium-confidence match; the per-field
    /// [`MatchBreakdown`] should be inspected before clinical use.
    Medium,
    /// Score is below `0.75`. Treat as a candidate at best; require
    /// additional evidence before treating as the same worker.
    Low,
}

impl Confidence {
    /// Bucket a probabilistic score into one of the three bands.
    ///
    /// The function is total over `f64`: NaN inputs degrade to `Low`,
    /// negative scores degrade to `Low`, scores above `1.0` are treated
    /// as `High`. In practice the matcher only ever produces values in
    /// `[0.0, 1.0]`, so callers shouldn't encounter the degenerate
    /// inputs.
    ///
    /// ```
    /// use worker_matcher::Confidence;
    ///
    /// assert_eq!(Confidence::from_score(f64::NAN), Confidence::Low);
    /// assert_eq!(Confidence::from_score(-0.5),     Confidence::Low);
    /// assert_eq!(Confidence::from_score(2.0),      Confidence::High);
    /// ```
    #[must_use]
    pub fn from_score(score: f64) -> Self {
        if score >= 0.90 {
            Confidence::High
        } else if score >= 0.75 {
            Confidence::Medium
        } else {
            Confidence::Low
        }
    }
}

/// Outcome of a probabilistic worker match.
///
/// Contains the overall renormalised `score`, the threshold-derived
/// `is_match` boolean, a coarse [`Confidence`] band, and a per-field
/// [`MatchBreakdown`] for audit.
///
/// `MatchResult` implements `Serialize + Deserialize` so it can be persisted
/// or returned over an API.
///
/// ```
/// use worker_matcher::{Confidence, MatchingEngine, Worker};
///
/// let p = Worker::builder().given_name("Ada").family_name("Lovelace").build();
/// let q = p.clone();
/// let result = MatchingEngine::default_config().match_workers(&p, &q);
/// assert_eq!(result.confidence, Confidence::High);
///
/// // Round-trip through JSON.
/// let json = serde_json::to_string(&result).unwrap();
/// let back: worker_matcher::MatchResult = serde_json::from_str(&json).unwrap();
/// assert!((result.score - back.score).abs() < 1e-12);
/// assert_eq!(result.is_match, back.is_match);
/// assert_eq!(result.confidence, back.confidence);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    /// Overall match score in `[0.0, 1.0]`.
    pub score: f64,

    /// `true` if `score >= MatchConfig::match_threshold`.
    pub is_match: bool,

    /// Coarse confidence band derived from `score` per spec §12.5.
    /// Defaults to [`Confidence::Low`] on legacy JSON payloads that
    /// pre-date the field.
    #[serde(default = "default_confidence")]
    pub confidence: Confidence,

    /// Per-field score contributions for explainability.
    pub breakdown: MatchBreakdown,
}

/// Backstop for legacy `MatchResult` JSON payloads that lack the
/// `confidence` field. Returns `Confidence::Low` so a deserialised
/// payload that pre-dates the field is unambiguously flagged as
/// "needs re-scoring".
fn default_confidence() -> Confidence {
    Confidence::Low
}

/// Per-field score breakdown returned with every [`MatchResult`].
///
/// Each field is `Option<f64>`:
///
/// - `Some(score)` — the field was scored; the value is in `[0.0, 1.0]`.
/// - `None` — the field was missing on at least one side and so did not
///   participate in the weighted sum.
///
/// The breakdown exists so a clinician or auditor can see *why* a match was
/// flagged. Do not throw it away in downstream services.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchBreakdown {
    /// Score for UK NHS Number equality (`1.0` or `0.0`), or `None` if either side did not parse.
    #[serde(default)]
    pub uk_nhs_number_score: Option<f64>,
    /// Score for France NIR equality (`1.0` or `0.0`), or `None` if either side did not parse.
    #[serde(default)]
    pub fr_nir_score: Option<f64>,
    /// Score for España TSI / CIP-SNS equality (`1.0` or `0.0`), or `None` if either side did not parse.
    #[serde(default)]
    pub es_tsi_score: Option<f64>,
    /// Score for Éire IHI equality (`1.0` or `0.0`), or `None` if either side did not parse.
    #[serde(default)]
    pub ie_ihi_score: Option<f64>,
    /// Score for United Kingdom Northern Ireland H&C Number equality (`1.0` or `0.0`), or `None` if either side did not parse.
    #[serde(default)]
    pub uk_hc_number_score: Option<f64>,
    /// Score for United States Social Security Number equality (`1.0` or `0.0`), or `None` if either side did not parse.
    #[serde(default)]
    pub us_ssn_score: Option<f64>,
    /// Score for Australia IHI equality (`1.0` or `0.0`), or `None` if either side did not parse.
    #[serde(default)]
    pub au_ihi_score: Option<f64>,
    /// Score for Germany KVNR equality (`1.0` or `0.0`), or `None` if either side did not parse.
    #[serde(default)]
    pub de_kvnr_score: Option<f64>,
    /// Score for Italy *Codice Fiscale* equality (`1.0` or `0.0`), or `None` if either side did not parse.
    #[serde(default)]
    pub it_cf_score: Option<f64>,
    /// Score for Netherlands BSN equality (`1.0` or `0.0`), or `None` if either side did not parse.
    #[serde(default)]
    pub nl_bsn_score: Option<f64>,
    /// Score for Sweden *Personnummer* equality (`1.0` or `0.0`), or `None` if either side did not parse.
    #[serde(default)]
    pub se_personnummer_score: Option<f64>,
    /// Score for United Kingdom (Scotland) CHI Number equality (`1.0` or `0.0`), or `None` if either side did not parse.
    #[serde(default)]
    pub uk_chi_number_score: Option<f64>,
    /// Score for Belgium National Number equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub be_nn_score: Option<f64>,
    /// Score for Bulgaria EGN equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub bg_egn_score: Option<f64>,
    /// Score for Czech *Rodné číslo* equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub cz_rc_score: Option<f64>,
    /// Score for Denmark CPR equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub dk_cpr_score: Option<f64>,
    /// Score for Estonia *Isikukood* equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub ee_ik_score: Option<f64>,
    /// Score for Spain DNI / NIE equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub es_dni_score: Option<f64>,
    /// Score for Finland HETU equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub fi_hetu_score: Option<f64>,
    /// Score for Croatia OIB equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub hr_oib_score: Option<f64>,
    /// Score for Iceland *Kennitala* equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub is_kt_score: Option<f64>,
    /// Score for Lithuania *Asmens kodas* equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub lt_ak_score: Option<f64>,
    /// Score for Latvia *Personas kods* equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub lv_pk_score: Option<f64>,
    /// Score for Malta National ID equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub mt_id_score: Option<f64>,
    /// Score for Norway *Fødselsnummer* equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub no_fnr_score: Option<f64>,
    /// Score for Poland PESEL equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub pl_pesel_score: Option<f64>,
    /// Score for Romania CNP equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub ro_cnp_score: Option<f64>,
    /// Score for Slovenia EMŠO equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub si_emso_score: Option<f64>,
    /// Score for Slovakia *Rodné číslo* equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub sk_rc_score: Option<f64>,
    /// Score for UK NINO equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub uk_nino_score: Option<f64>,
    /// Score for Greece DSS investor-share equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub gr_dss_score: Option<f64>,
    /// Score for Liechtenstein National ID equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub li_id_score: Option<f64>,
    /// Score for Netherlands National ID equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub nl_id_score: Option<f64>,
    /// Score for Poland NIP equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub pl_nip_score: Option<f64>,
    /// Score for Portugal NIF equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub pt_nif_score: Option<f64>,
    /// Score for Brazil CPF equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub br_cpf_score: Option<f64>,
    /// Score for China Resident Identity Card equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub cn_rrn_score: Option<f64>,
    /// Score for India Aadhaar equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub in_aadhaar_score: Option<f64>,
    /// Score for Japan My Number equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub jp_my_number_score: Option<f64>,
    /// Score for Mexico CURP equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub mx_curp_score: Option<f64>,
    /// Score for New Zealand NHI equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub nz_nhi_score: Option<f64>,
    /// Score for South Africa ID Number equality (`1.0` or `0.0`), or `None`.
    #[serde(default)]
    pub za_id_score: Option<f64>,
    /// Score for passport-book match: `Some(1.0)` when at least one
    /// `(country, number)` pair is shared across the two workers'
    /// passport lists, `Some(0.0)` when both have at least one book
    /// but no pair is shared, `None` when either side has no books.
    /// See spec §6.4a / FR-51.
    #[serde(default)]
    pub passport_book_score: Option<f64>,
    /// Score for given-name similarity using the configured algorithm.
    pub given_name_score: Option<f64>,
    /// Score for family-name similarity using the configured algorithm.
    pub family_name_score: Option<f64>,
    /// Score for date-of-birth equality (`1.0` or `0.0`).
    pub date_of_birth_score: Option<f64>,
    /// Score for gender equality (`1.0` or `0.0`).
    pub gender_score: Option<f64>,
    /// Score for ABO+RhD blood-type equality (`1.0` or `0.0`), or
    /// `None` if either side is missing. See [`crate::BloodType`].
    #[serde(default)]
    pub blood_type_score: Option<f64>,
    /// Score for multiple-birth indicator equality (`1.0` for matching
    /// birth order, `0.0` for different birth orders within the same
    /// multiple-birth set), or `None` if either side is missing.
    #[serde(default)]
    pub multiple_birth_score: Option<f64>,
    /// Score for address similarity (weighted blend of postcode, city, line 1).
    pub address_score: Option<f64>,
    /// Score for place-of-birth similarity (city Jaro-Winkler blended
    /// with country exact match), or `None` if either side is absent
    /// or carries no city / country sub-fields.
    #[serde(default)]
    pub birth_place_score: Option<f64>,
    /// Score for date-of-death equality, using the same DOB
    /// transposition heuristic as [`Self::date_of_birth_score`]:
    /// `1.0` on exact equality, `0.5` on day/month swap, `0.0`
    /// otherwise. `None` if either side has no recorded death date.
    #[serde(default)]
    pub death_date_score: Option<f64>,
    /// Score for place-of-death similarity (analogous to
    /// [`Self::birth_place_score`]: city Jaro-Winkler blended with
    /// country exact match), or `None` if either side is absent or
    /// carries no city / country sub-fields.
    #[serde(default)]
    pub death_place_score: Option<f64>,
    /// Score for normalised phone-number equality (`1.0` or `0.0`).
    pub phone_score: Option<f64>,
    /// Score for normalised email-address equality (`1.0` or `0.0`).
    /// `None` if either side is absent or fails to parse as an email.
    #[serde(default)]
    pub email_score: Option<f64>,
    /// Mean Soundex match across given and family names (`0.0`, `0.5`, or `1.0`).
    pub phonetic_name_score: Option<f64>,
}

/// Worker matcher engine.
///
/// The engine is **immutable after construction** and cheap to clone (it
/// owns only a [`MatchConfig`]). Construct one and call its methods from any
/// thread.
///
/// ```
/// use worker_matcher::{MatchConfig, MatchingEngine};
///
/// let engine_a = MatchingEngine::default_config();
/// let engine_b = MatchingEngine::new(MatchConfig::strict());
/// # let _ = (engine_a, engine_b);
/// ```
pub struct MatchingEngine {
    config: MatchConfig,
}

impl MatchingEngine {
    /// Construct an engine with the given configuration.
    ///
    /// ```
    /// use worker_matcher::{MatchConfig, MatchingEngine};
    /// let engine = MatchingEngine::new(MatchConfig::lenient());
    /// # let _ = engine;
    /// ```
    #[must_use]
    pub fn new(config: MatchConfig) -> Self {
        Self { config }
    }

    /// Construct an engine with [`MatchConfig::default`].
    ///
    /// ```
    /// use worker_matcher::MatchingEngine;
    /// let engine = MatchingEngine::default_config();
    /// # let _ = engine;
    /// ```
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(MatchConfig::default())
    }

    /// Compare two workers probabilistically and return a [`MatchResult`].
    ///
    /// The score is the weight-renormalised sum of every component that
    /// scored on both records. Missing fields are skipped, not penalised.
    ///
    /// ```
    /// use worker_matcher::{MatchingEngine, Worker};
    /// use chrono::NaiveDate;
    ///
    /// let p = Worker::builder()
    ///     .given_name("Carys")
    ///     .family_name("Pritchard")
    ///     .date_of_birth(NaiveDate::from_ymd_opt(1985, 1, 1).unwrap())
    ///     .build();
    ///
    /// let result = MatchingEngine::default_config().match_workers(&p, &p);
    /// assert!(result.is_match);
    /// assert!(result.score > 0.99);
    /// ```
    #[must_use]
    pub fn match_workers(&self, worker1: &Worker, worker2: &Worker) -> MatchResult {
        let breakdown = self.calculate_breakdown(worker1, worker2);
        let score = self.calculate_weighted_score(&breakdown);
        let above_threshold = score >= self.config.match_threshold;
        // Under strict mode, `is_match` ALSO requires a deterministic
        // match (any identifier scheme agrees, or the full demographic
        // tuple agrees). This narrows the false-positive surface in
        // clinical workflows where the threshold alone is not enough.
        // See spec FR-47 / §13.2 / OQ-5.
        let is_match = if self.config.strict_mode {
            above_threshold && self.deterministic_match(worker1, worker2)
        } else {
            above_threshold
        };
        let confidence = Confidence::from_score(score);

        MatchResult {
            score,
            is_match,
            confidence,
            breakdown,
        }
    }

    /// Score a single query against many candidates in parallel-friendly
    /// fashion. Returns one [`MatchResult`] per candidate, in the same
    /// order as the input slice.
    ///
    /// This is the building block for a Main Worker Index workflow:
    /// given a new record and the existing population, produce a fully
    /// audited score for each potential link. The engine is immutable
    /// and `Send + Sync`, so call-sites that want parallel evaluation
    /// can wrap the call in `rayon::par_iter` or similar without
    /// further changes to this crate.
    ///
    /// For sparse / large populations consider blocking — pre-filter
    /// `candidates` with a cheap predicate (e.g. matching family-name
    /// Soundex or postcode prefix) before passing the survivors to this
    /// function. Blocking is a consumer concern and is intentionally
    /// not baked into the API; the crate stays a pure scoring library.
    ///
    /// # Examples
    ///
    /// ```
    /// use worker_matcher::{MatchingEngine, Worker};
    ///
    /// let query = Worker::builder()
    ///     .given_name("Ada")
    ///     .family_name("Lovelace")
    ///     .build();
    /// let candidates = vec![
    ///     Worker::builder().given_name("Ada").family_name("Lovelace").build(),
    ///     Worker::builder().given_name("Alan").family_name("Turing").build(),
    ///     Worker::builder().given_name("Grace").family_name("Hopper").build(),
    /// ];
    ///
    /// let engine = MatchingEngine::default_config();
    /// let results = engine.match_one_to_many(&query, &candidates);
    ///
    /// assert_eq!(results.len(), 3);
    /// assert!(results[0].is_match);
    /// assert!(!results[1].is_match);
    /// ```
    ///
    /// Empty candidates yield an empty result:
    ///
    /// ```
    /// # use worker_matcher::{MatchingEngine, Worker};
    /// let q = Worker::builder().given_name("Solo").build();
    /// let r = MatchingEngine::default_config().match_one_to_many(&q, &[]);
    /// assert!(r.is_empty());
    /// ```
    #[must_use]
    pub fn match_one_to_many(&self, query: &Worker, candidates: &[Worker]) -> Vec<MatchResult> {
        candidates
            .iter()
            .map(|c| self.match_workers(query, c))
            .collect()
    }

    /// Score and rank: return `(original_index, MatchResult)` tuples
    /// sorted by descending score. Ties are broken by ascending original
    /// index, so the result is deterministic.
    ///
    /// Convenience wrapper around [`MatchingEngine::match_one_to_many`]
    /// for the common "give me the best matches first" workflow.
    /// Consumers that need a filtered view (e.g. only `is_match == true`)
    /// can drop entries off the front, while consumers that need to
    /// pair results with external metadata can use the preserved
    /// original index.
    ///
    /// # Examples
    ///
    /// ```
    /// use worker_matcher::{MatchingEngine, Worker};
    ///
    /// let query = Worker::builder().given_name("Ada").family_name("Lovelace").build();
    /// let candidates = vec![
    ///     Worker::builder().given_name("Grace").family_name("Hopper").build(),   // index 0
    ///     Worker::builder().given_name("Ada").family_name("Lovelace").build(),   // index 1 — best match
    ///     Worker::builder().given_name("Alan").family_name("Turing").build(),    // index 2
    /// ];
    ///
    /// let ranked = MatchingEngine::default_config().rank_one_to_many(&query, &candidates);
    /// assert_eq!(ranked.len(), 3);
    /// assert_eq!(ranked[0].0, 1);                  // best match's original index
    /// assert!(ranked[0].1.score >= ranked[1].1.score);
    /// assert!(ranked[1].1.score >= ranked[2].1.score);
    /// ```
    #[must_use]
    pub fn rank_one_to_many(
        &self,
        query: &Worker,
        candidates: &[Worker],
    ) -> Vec<(usize, MatchResult)> {
        let mut indexed: Vec<(usize, MatchResult)> = self
            .match_one_to_many(query, candidates)
            .into_iter()
            .enumerate()
            .collect();
        indexed.sort_by(|a, b| {
            b.1.score
                .partial_cmp(&a.1.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        indexed
    }

    /// Compare two workers deterministically and return a single boolean.
    ///
    /// Returns `true` iff **any** of the following hold:
    ///
    /// - Both sides parse and are equal on the **same** national-identifier
    ///   scheme, for any of the **42 supported schemes** (one field per
    ///   scheme on [`crate::Worker`]; catalogue in
    ///   `agents/national-person-identifiers.md`) — e.g. both UK NHS
    ///   Numbers, both France NIRs, both US Social Security Numbers, and
    ///   so on through the full list. See
    ///   [`agents/matching-algorithm.md`](https://github.com/sixarm/worker-matcher-rust-crate/blob/main/agents/matching-algorithm.md)
    ///   for the exhaustive per-scheme branch table.
    /// - The workers share at least one `(country, number)` passport-book
    ///   pair after canonicalisation (see [`crate::PassportBook`]).
    /// - Normalised given name matches, **and** normalised family name
    ///   matches, **and** date of birth matches, **and** gender matches (or
    ///   is missing on at least one side).
    ///
    /// National identifiers from different schemes never cross-match: an
    /// NHS Number is only ever compared against another NHS Number, never
    /// against an H&C Number that happens to share the same 10 digits.
    ///
    /// ```
    /// use worker_matcher::{MatchingEngine, Worker};
    ///
    /// // Same NHS number, different formatting → match.
    /// let a = Worker::builder().uk_nhs_number("943 476 5919").build();
    /// let b = Worker::builder().uk_nhs_number("9434765919").build();
    /// assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    /// ```
    #[must_use]
    pub fn deterministic_match(&self, worker1: &Worker, worker2: &Worker) -> bool {
        if Self::deterministic_identifier_match(worker1, worker2) {
            return true;
        }
        if passport_books_share_pair(&worker1.passport_books, &worker2.passport_books) {
            return true;
        }

        // SEC-M2: a name normalising to empty (blank / punctuation-only) is
        // not identity evidence — require a non-empty normal form so two
        // records sharing only a blank name do not agree here.
        let name_match = match (&worker1.given_name, &worker2.given_name) {
            (Some(f1), Some(f2)) => {
                let (n1, n2) = (
                    Normalizer::normalize_name(f1),
                    Normalizer::normalize_name(f2),
                );
                !n1.is_empty() && n1 == n2
            }
            _ => false,
        } && match (&worker1.family_name, &worker2.family_name) {
            (Some(l1), Some(l2)) => {
                let (n1, n2) = (
                    Normalizer::normalize_name(l1),
                    Normalizer::normalize_name(l2),
                );
                !n1.is_empty() && n1 == n2
            }
            _ => false,
        };

        let dob_match = match (worker1.date_of_birth, worker2.date_of_birth) {
            (Some(d1), Some(d2)) => d1 == d2,
            _ => false,
        };

        let gender_match = match (worker1.gender, worker2.gender) {
            (Some(g1), Some(g2)) => g1 == g2,
            _ => true,
        };

        name_match && dob_match && gender_match
    }

    /// `true` iff any supported national-identifier scheme matches between
    /// the two workers. National identifiers never cross-match across
    /// schemes: each `(field1, field2, parser)` triple is compared only
    /// against its own scheme. The first matching scheme short-circuits.
    fn deterministic_identifier_match(worker1: &Worker, worker2: &Worker) -> bool {
        // Each entry: the same scheme field on each worker plus the parser
        // that canonicalises it. The parser is the source of truth on
        // validity; mismatched or unparseable values simply do not match.
        let schemes: &[SchemePair<'_>] = &[
            (
                &worker1.uk_nhs_number,
                &worker2.uk_nhs_number,
                identifiers::parse_uk_nhs_number,
            ),
            (&worker1.fr_nir, &worker2.fr_nir, identifiers::parse_fr_nir),
            (&worker1.es_tsi, &worker2.es_tsi, identifiers::parse_es_tsi),
            (&worker1.ie_ihi, &worker2.ie_ihi, identifiers::parse_ie_ihi),
            (
                &worker1.uk_hc_number,
                &worker2.uk_hc_number,
                identifiers::parse_uk_hc_number,
            ),
            (&worker1.us_ssn, &worker2.us_ssn, identifiers::parse_us_ssn),
            (&worker1.au_ihi, &worker2.au_ihi, identifiers::parse_au_ihi),
            (
                &worker1.de_kvnr,
                &worker2.de_kvnr,
                identifiers::parse_de_kvnr,
            ),
            (&worker1.it_cf, &worker2.it_cf, identifiers::parse_it_cf),
            (&worker1.nl_bsn, &worker2.nl_bsn, identifiers::parse_nl_bsn),
            (
                &worker1.se_personnummer,
                &worker2.se_personnummer,
                identifiers::parse_se_personnummer,
            ),
            (
                &worker1.uk_chi_number,
                &worker2.uk_chi_number,
                identifiers::parse_uk_chi_number,
            ),
            (&worker1.be_nn, &worker2.be_nn, identifiers::parse_be_nn),
            (&worker1.bg_egn, &worker2.bg_egn, identifiers::parse_bg_egn),
            (&worker1.cz_rc, &worker2.cz_rc, identifiers::parse_cz_rc),
            (&worker1.dk_cpr, &worker2.dk_cpr, identifiers::parse_dk_cpr),
            (&worker1.ee_ik, &worker2.ee_ik, identifiers::parse_ee_ik),
            (&worker1.es_dni, &worker2.es_dni, identifiers::parse_es_dni),
            (
                &worker1.fi_hetu,
                &worker2.fi_hetu,
                identifiers::parse_fi_hetu,
            ),
            (&worker1.hr_oib, &worker2.hr_oib, identifiers::parse_hr_oib),
            (&worker1.is_kt, &worker2.is_kt, identifiers::parse_is_kt),
            (&worker1.lt_ak, &worker2.lt_ak, identifiers::parse_lt_ak),
            (&worker1.lv_pk, &worker2.lv_pk, identifiers::parse_lv_pk),
            (&worker1.mt_id, &worker2.mt_id, identifiers::parse_mt_id),
            (&worker1.no_fnr, &worker2.no_fnr, identifiers::parse_no_fnr),
            (
                &worker1.pl_pesel,
                &worker2.pl_pesel,
                identifiers::parse_pl_pesel,
            ),
            (&worker1.ro_cnp, &worker2.ro_cnp, identifiers::parse_ro_cnp),
            (
                &worker1.si_emso,
                &worker2.si_emso,
                identifiers::parse_si_emso,
            ),
            (&worker1.sk_rc, &worker2.sk_rc, identifiers::parse_sk_rc),
            (
                &worker1.uk_nino,
                &worker2.uk_nino,
                identifiers::parse_uk_nino,
            ),
            (&worker1.gr_dss, &worker2.gr_dss, identifiers::parse_gr_dss),
            (&worker1.li_id, &worker2.li_id, identifiers::parse_li_id),
            (&worker1.nl_id, &worker2.nl_id, identifiers::parse_nl_id),
            (&worker1.pl_nip, &worker2.pl_nip, identifiers::parse_pl_nip),
            (&worker1.pt_nif, &worker2.pt_nif, identifiers::parse_pt_nif),
            (&worker1.br_cpf, &worker2.br_cpf, identifiers::parse_br_cpf),
            (&worker1.cn_rrn, &worker2.cn_rrn, identifiers::parse_cn_rrn),
            (
                &worker1.in_aadhaar,
                &worker2.in_aadhaar,
                identifiers::parse_in_aadhaar,
            ),
            (
                &worker1.jp_my_number,
                &worker2.jp_my_number,
                identifiers::parse_jp_my_number,
            ),
            (
                &worker1.mx_curp,
                &worker2.mx_curp,
                identifiers::parse_mx_curp,
            ),
            (&worker1.nz_nhi, &worker2.nz_nhi, identifiers::parse_nz_nhi),
            (&worker1.za_id, &worker2.za_id, identifiers::parse_za_id),
        ];
        schemes
            .iter()
            .any(|(a, b, parser)| identifier_equal(a.as_ref(), b.as_ref(), parser))
    }

    /// Build the full per-field [`MatchBreakdown`]. The national-identifier
    /// scores are computed by [`Self::identifier_breakdown`]; this method adds
    /// the passport, name, demographic, and contact scores on top via struct
    /// update so the field enumeration stays auditable but no single function
    /// grows unwieldy.
    fn calculate_breakdown(&self, worker1: &Worker, worker2: &Worker) -> MatchBreakdown {
        MatchBreakdown {
            passport_book_score: score_passport_books(
                &worker1.passport_books,
                &worker2.passport_books,
            ),
            given_name_score: self.score_given_name(worker1, worker2),
            family_name_score: self.score_family_name(worker1, worker2),
            date_of_birth_score: Self::score_date_of_birth(worker1, worker2),
            gender_score: Self::score_gender(worker1, worker2),
            blood_type_score: Self::score_blood_type(worker1, worker2),
            multiple_birth_score: Self::score_multiple_birth(worker1, worker2),
            address_score: Self::score_address(worker1, worker2),
            birth_place_score: Self::score_birth_place(worker1, worker2),
            death_date_score: Self::score_death_date(worker1, worker2),
            death_place_score: Self::score_death_place(worker1, worker2),
            phone_score: self.score_phone(worker1, worker2),
            email_score: self.score_email(worker1, worker2),
            phonetic_name_score: if self.config.use_phonetic_matching {
                Self::score_phonetic_names(worker1, worker2)
            } else {
                None
            },
            ..Self::identifier_breakdown(worker1, worker2)
        }
    }

    /// Score every national-identifier scheme into a [`MatchBreakdown`] whose
    /// non-identifier fields are left at their `Default` (`None`). Scheme
    /// scores never cross-match: each field is parsed only with its own
    /// scheme's parser.
    fn identifier_breakdown(worker1: &Worker, worker2: &Worker) -> MatchBreakdown {
        MatchBreakdown {
            uk_nhs_number_score: identifier_score(
                worker1.uk_nhs_number.as_ref(),
                worker2.uk_nhs_number.as_ref(),
                identifiers::parse_uk_nhs_number,
            ),
            fr_nir_score: identifier_score(
                worker1.fr_nir.as_ref(),
                worker2.fr_nir.as_ref(),
                identifiers::parse_fr_nir,
            ),
            es_tsi_score: identifier_score(
                worker1.es_tsi.as_ref(),
                worker2.es_tsi.as_ref(),
                identifiers::parse_es_tsi,
            ),
            ie_ihi_score: identifier_score(
                worker1.ie_ihi.as_ref(),
                worker2.ie_ihi.as_ref(),
                identifiers::parse_ie_ihi,
            ),
            uk_hc_number_score: identifier_score(
                worker1.uk_hc_number.as_ref(),
                worker2.uk_hc_number.as_ref(),
                identifiers::parse_uk_hc_number,
            ),
            us_ssn_score: identifier_score(
                worker1.us_ssn.as_ref(),
                worker2.us_ssn.as_ref(),
                identifiers::parse_us_ssn,
            ),
            au_ihi_score: identifier_score(
                worker1.au_ihi.as_ref(),
                worker2.au_ihi.as_ref(),
                identifiers::parse_au_ihi,
            ),
            de_kvnr_score: identifier_score(
                worker1.de_kvnr.as_ref(),
                worker2.de_kvnr.as_ref(),
                identifiers::parse_de_kvnr,
            ),
            it_cf_score: identifier_score(
                worker1.it_cf.as_ref(),
                worker2.it_cf.as_ref(),
                identifiers::parse_it_cf,
            ),
            nl_bsn_score: identifier_score(
                worker1.nl_bsn.as_ref(),
                worker2.nl_bsn.as_ref(),
                identifiers::parse_nl_bsn,
            ),
            se_personnummer_score: identifier_score(
                worker1.se_personnummer.as_ref(),
                worker2.se_personnummer.as_ref(),
                identifiers::parse_se_personnummer,
            ),
            uk_chi_number_score: identifier_score(
                worker1.uk_chi_number.as_ref(),
                worker2.uk_chi_number.as_ref(),
                identifiers::parse_uk_chi_number,
            ),
            be_nn_score: identifier_score(
                worker1.be_nn.as_ref(),
                worker2.be_nn.as_ref(),
                identifiers::parse_be_nn,
            ),
            bg_egn_score: identifier_score(
                worker1.bg_egn.as_ref(),
                worker2.bg_egn.as_ref(),
                identifiers::parse_bg_egn,
            ),
            cz_rc_score: identifier_score(
                worker1.cz_rc.as_ref(),
                worker2.cz_rc.as_ref(),
                identifiers::parse_cz_rc,
            ),
            dk_cpr_score: identifier_score(
                worker1.dk_cpr.as_ref(),
                worker2.dk_cpr.as_ref(),
                identifiers::parse_dk_cpr,
            ),
            ee_ik_score: identifier_score(
                worker1.ee_ik.as_ref(),
                worker2.ee_ik.as_ref(),
                identifiers::parse_ee_ik,
            ),
            es_dni_score: identifier_score(
                worker1.es_dni.as_ref(),
                worker2.es_dni.as_ref(),
                identifiers::parse_es_dni,
            ),
            fi_hetu_score: identifier_score(
                worker1.fi_hetu.as_ref(),
                worker2.fi_hetu.as_ref(),
                identifiers::parse_fi_hetu,
            ),
            ..Self::identifier_breakdown_2(worker1, worker2)
        }
    }

    /// Continuation of [`Self::identifier_breakdown`]: scores the remaining
    /// national-identifier schemes (Croatia onward) and leaves all other
    /// fields at their `Default` (`None`).
    fn identifier_breakdown_2(worker1: &Worker, worker2: &Worker) -> MatchBreakdown {
        MatchBreakdown {
            hr_oib_score: identifier_score(
                worker1.hr_oib.as_ref(),
                worker2.hr_oib.as_ref(),
                identifiers::parse_hr_oib,
            ),
            is_kt_score: identifier_score(
                worker1.is_kt.as_ref(),
                worker2.is_kt.as_ref(),
                identifiers::parse_is_kt,
            ),
            lt_ak_score: identifier_score(
                worker1.lt_ak.as_ref(),
                worker2.lt_ak.as_ref(),
                identifiers::parse_lt_ak,
            ),
            lv_pk_score: identifier_score(
                worker1.lv_pk.as_ref(),
                worker2.lv_pk.as_ref(),
                identifiers::parse_lv_pk,
            ),
            mt_id_score: identifier_score(
                worker1.mt_id.as_ref(),
                worker2.mt_id.as_ref(),
                identifiers::parse_mt_id,
            ),
            no_fnr_score: identifier_score(
                worker1.no_fnr.as_ref(),
                worker2.no_fnr.as_ref(),
                identifiers::parse_no_fnr,
            ),
            pl_pesel_score: identifier_score(
                worker1.pl_pesel.as_ref(),
                worker2.pl_pesel.as_ref(),
                identifiers::parse_pl_pesel,
            ),
            ..Self::identifier_breakdown_3(worker1, worker2)
        }
    }

    /// Continuation of [`Self::identifier_breakdown_2`]: scores the final
    /// national-identifier schemes (Romania onward) and leaves all other
    /// fields at their `Default` (`None`).
    fn identifier_breakdown_3(worker1: &Worker, worker2: &Worker) -> MatchBreakdown {
        MatchBreakdown {
            ro_cnp_score: identifier_score(
                worker1.ro_cnp.as_ref(),
                worker2.ro_cnp.as_ref(),
                identifiers::parse_ro_cnp,
            ),
            si_emso_score: identifier_score(
                worker1.si_emso.as_ref(),
                worker2.si_emso.as_ref(),
                identifiers::parse_si_emso,
            ),
            sk_rc_score: identifier_score(
                worker1.sk_rc.as_ref(),
                worker2.sk_rc.as_ref(),
                identifiers::parse_sk_rc,
            ),
            uk_nino_score: identifier_score(
                worker1.uk_nino.as_ref(),
                worker2.uk_nino.as_ref(),
                identifiers::parse_uk_nino,
            ),
            gr_dss_score: identifier_score(
                worker1.gr_dss.as_ref(),
                worker2.gr_dss.as_ref(),
                identifiers::parse_gr_dss,
            ),
            li_id_score: identifier_score(
                worker1.li_id.as_ref(),
                worker2.li_id.as_ref(),
                identifiers::parse_li_id,
            ),
            nl_id_score: identifier_score(
                worker1.nl_id.as_ref(),
                worker2.nl_id.as_ref(),
                identifiers::parse_nl_id,
            ),
            pl_nip_score: identifier_score(
                worker1.pl_nip.as_ref(),
                worker2.pl_nip.as_ref(),
                identifiers::parse_pl_nip,
            ),
            pt_nif_score: identifier_score(
                worker1.pt_nif.as_ref(),
                worker2.pt_nif.as_ref(),
                identifiers::parse_pt_nif,
            ),
            br_cpf_score: identifier_score(
                worker1.br_cpf.as_ref(),
                worker2.br_cpf.as_ref(),
                identifiers::parse_br_cpf,
            ),
            cn_rrn_score: identifier_score(
                worker1.cn_rrn.as_ref(),
                worker2.cn_rrn.as_ref(),
                identifiers::parse_cn_rrn,
            ),
            in_aadhaar_score: identifier_score(
                worker1.in_aadhaar.as_ref(),
                worker2.in_aadhaar.as_ref(),
                identifiers::parse_in_aadhaar,
            ),
            jp_my_number_score: identifier_score(
                worker1.jp_my_number.as_ref(),
                worker2.jp_my_number.as_ref(),
                identifiers::parse_jp_my_number,
            ),
            mx_curp_score: identifier_score(
                worker1.mx_curp.as_ref(),
                worker2.mx_curp.as_ref(),
                identifiers::parse_mx_curp,
            ),
            nz_nhi_score: identifier_score(
                worker1.nz_nhi.as_ref(),
                worker2.nz_nhi.as_ref(),
                identifiers::parse_nz_nhi,
            ),
            za_id_score: identifier_score(
                worker1.za_id.as_ref(),
                worker2.za_id.as_ref(),
                identifiers::parse_za_id,
            ),
            ..MatchBreakdown::default()
        }
    }

    /// Weight-renormalised probabilistic score in `[0.0, 1.0]`.
    ///
    /// Every component that scored on both records contributes
    /// `score × weight` to the numerator and `weight` to the denominator;
    /// the phonetic-name component is a bonus that only ever raises the
    /// score. The per-field weighting is gathered by two cohesive helpers
    /// ([`Self::accumulate_identifier_weights`] and
    /// [`Self::accumulate_field_weights`]) so the field enumeration stays
    /// auditable without any single function growing unwieldy.
    fn calculate_weighted_score(&self, breakdown: &MatchBreakdown) -> f64 {
        let (mut weighted_sum, mut total_weight) = (0.0_f64, 0.0_f64);
        self.accumulate_identifier_weights(breakdown, &mut weighted_sum, &mut total_weight);
        self.accumulate_field_weights(breakdown, &mut weighted_sum, &mut total_weight);

        // Phonetic match is a bonus only — never lowers the score.
        if let Some(score) = breakdown.phonetic_name_score
            && score > 0.9
        {
            weighted_sum += score * 0.05;
            total_weight += 0.05;
        }

        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }

    /// Fold every national-identifier component of `breakdown` into the
    /// running `(weighted_sum, total_weight)` accumulators.
    fn accumulate_identifier_weights(
        &self,
        breakdown: &MatchBreakdown,
        weighted_sum: &mut f64,
        total_weight: &mut f64,
    ) {
        let mut add = |score: Option<f64>, weight: f64| {
            if let Some(score) = score {
                *weighted_sum += score * weight;
                *total_weight += weight;
            }
        };
        let c = &self.config;
        add(breakdown.uk_nhs_number_score, c.uk_nhs_number_weight);
        add(breakdown.fr_nir_score, c.fr_nir_weight);
        add(breakdown.es_tsi_score, c.es_tsi_weight);
        add(breakdown.ie_ihi_score, c.ie_ihi_weight);
        add(breakdown.uk_hc_number_score, c.uk_hc_number_weight);
        add(breakdown.us_ssn_score, c.us_ssn_weight);
        add(breakdown.au_ihi_score, c.au_ihi_weight);
        add(breakdown.de_kvnr_score, c.de_kvnr_weight);
        add(breakdown.it_cf_score, c.it_cf_weight);
        add(breakdown.nl_bsn_score, c.nl_bsn_weight);
        add(breakdown.se_personnummer_score, c.se_personnummer_weight);
        add(breakdown.uk_chi_number_score, c.uk_chi_number_weight);
        add(breakdown.be_nn_score, c.be_nn_weight);
        add(breakdown.bg_egn_score, c.bg_egn_weight);
        add(breakdown.cz_rc_score, c.cz_rc_weight);
        add(breakdown.dk_cpr_score, c.dk_cpr_weight);
        add(breakdown.ee_ik_score, c.ee_ik_weight);
        add(breakdown.es_dni_score, c.es_dni_weight);
        add(breakdown.fi_hetu_score, c.fi_hetu_weight);
        add(breakdown.hr_oib_score, c.hr_oib_weight);
        add(breakdown.is_kt_score, c.is_kt_weight);
        add(breakdown.lt_ak_score, c.lt_ak_weight);
        add(breakdown.lv_pk_score, c.lv_pk_weight);
        add(breakdown.mt_id_score, c.mt_id_weight);
        add(breakdown.no_fnr_score, c.no_fnr_weight);
        add(breakdown.pl_pesel_score, c.pl_pesel_weight);
        add(breakdown.ro_cnp_score, c.ro_cnp_weight);
        add(breakdown.si_emso_score, c.si_emso_weight);
        add(breakdown.sk_rc_score, c.sk_rc_weight);
        add(breakdown.uk_nino_score, c.uk_nino_weight);
        add(breakdown.gr_dss_score, c.gr_dss_weight);
        add(breakdown.li_id_score, c.li_id_weight);
        add(breakdown.nl_id_score, c.nl_id_weight);
        add(breakdown.pl_nip_score, c.pl_nip_weight);
        add(breakdown.pt_nif_score, c.pt_nif_weight);
        add(breakdown.br_cpf_score, c.br_cpf_weight);
        add(breakdown.cn_rrn_score, c.cn_rrn_weight);
        add(breakdown.in_aadhaar_score, c.in_aadhaar_weight);
        add(breakdown.jp_my_number_score, c.jp_my_number_weight);
        add(breakdown.mx_curp_score, c.mx_curp_weight);
        add(breakdown.nz_nhi_score, c.nz_nhi_weight);
        add(breakdown.za_id_score, c.za_id_weight);
    }

    /// Fold the passport, name, demographic, and contact components of
    /// `breakdown` (everything except the phonetic bonus) into the running
    /// `(weighted_sum, total_weight)` accumulators.
    fn accumulate_field_weights(
        &self,
        breakdown: &MatchBreakdown,
        weighted_sum: &mut f64,
        total_weight: &mut f64,
    ) {
        let mut add = |score: Option<f64>, weight: f64| {
            if let Some(score) = score {
                *weighted_sum += score * weight;
                *total_weight += weight;
            }
        };
        let c = &self.config;
        add(breakdown.passport_book_score, c.passport_book_weight);
        add(breakdown.given_name_score, c.given_name_weight);
        add(breakdown.family_name_score, c.family_name_weight);
        add(breakdown.date_of_birth_score, c.date_of_birth_weight);
        add(breakdown.gender_score, c.gender_weight);
        add(breakdown.blood_type_score, c.blood_type_weight);
        add(breakdown.multiple_birth_score, c.multiple_birth_weight);
        add(breakdown.address_score, c.address_weight);
        add(breakdown.birth_place_score, c.birth_place_weight);
        add(breakdown.death_date_score, c.death_date_weight);
        add(breakdown.death_place_score, c.death_place_weight);
        add(breakdown.phone_score, c.phone_weight);
        add(breakdown.email_score, c.email_weight);
    }

    fn score_given_name(&self, worker1: &Worker, worker2: &Worker) -> Option<f64> {
        let (g1, g2) = match (&worker1.given_name, &worker2.given_name) {
            (Some(a), Some(b)) => (a.as_str(), b.as_str()),
            _ => return None,
        };
        let given = self.score_name(g1, g2);
        // Middle-name contribution: when both workers have a middle
        // name, blend 5% of a middle-name similarity into the
        // given-name component. The same `score_name` helper is used,
        // so the configured similarity algorithm and nickname boost
        // apply to middle names as well. Per spec FR-49 / §12.2;
        // resolves OQ-1.
        let blended = match (&worker1.middle_name, &worker2.middle_name) {
            (Some(m1), Some(m2)) => {
                let middle = self.score_name(m1, m2);
                0.95 * given + 0.05 * middle
            }
            _ => given,
        };
        Some(blended)
    }

    fn score_family_name(&self, worker1: &Worker, worker2: &Worker) -> Option<f64> {
        match (&worker1.family_name, &worker2.family_name) {
            (Some(name1), Some(name2)) => Some(self.score_name(name1, name2)),
            _ => None,
        }
    }

    fn score_name(&self, name1: &str, name2: &str) -> f64 {
        let norm1 = Normalizer::normalize_name(name1);
        let norm2 = Normalizer::normalize_name(name2);
        let base = match self.config.name_algorithm {
            SimilarityAlgorithm::JaroWinkler => Scorer::jaro_winkler_similarity(&norm1, &norm2),
            SimilarityAlgorithm::Levenshtein => Scorer::levenshtein_similarity(&norm1, &norm2),
            SimilarityAlgorithm::Exact => Scorer::exact_match(&norm1, &norm2),
            SimilarityAlgorithm::Combined => Scorer::combined_similarity(&norm1, &norm2),
        };
        // Nickname boost: when both normalised forms appear in the same
        // equivalence class of the configured table, lift the score so
        // table-driven equivalence is not undone by a low Jaro-Winkler /
        // Levenshtein result on dissimilar surface forms. The boost only
        // ever raises the score; it never lowers it.
        if !self.config.nickname_table.is_empty()
            && self.config.nickname_table.are_equivalent(&norm1, &norm2)
        {
            base.max(0.9)
        } else {
            base
        }
    }

    fn score_date_of_birth(worker1: &Worker, worker2: &Worker) -> Option<f64> {
        match (worker1.date_of_birth, worker2.date_of_birth) {
            (Some(dob1), Some(dob2)) => Some(score_dob_pair(dob1, dob2)),
            _ => None,
        }
    }

    fn score_gender(worker1: &Worker, worker2: &Worker) -> Option<f64> {
        match (worker1.gender, worker2.gender) {
            (Some(g1), Some(g2)) => Some(if g1 == g2 { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    fn score_blood_type(worker1: &Worker, worker2: &Worker) -> Option<f64> {
        match (worker1.blood_type, worker2.blood_type) {
            (Some(b1), Some(b2)) => Some(if b1 == b2 { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    fn score_multiple_birth(worker1: &Worker, worker2: &Worker) -> Option<f64> {
        match (worker1.multiple_birth, worker2.multiple_birth) {
            (Some(m1), Some(m2)) => Some(f64::from(m1 == m2)),
            _ => None,
        }
    }

    fn score_address(worker1: &Worker, worker2: &Worker) -> Option<f64> {
        // Consider the current address plus any historical addresses
        // recorded on each worker. The reported `address_score` is the
        // best (highest) score across the cartesian product of the two
        // address lists. This catches the "worker moved house"
        // failure mode where the current addresses no longer agree
        // but a prior address on one side still matches the other
        // side's current. Per spec §12.4 / FR-48; resolves OQ-3.
        let all_p1: Vec<&Address> = worker1
            .address
            .as_ref()
            .into_iter()
            .chain(worker1.previous_addresses.iter())
            .collect();
        let all_p2: Vec<&Address> = worker2
            .address
            .as_ref()
            .into_iter()
            .chain(worker2.previous_addresses.iter())
            .collect();
        if all_p1.is_empty() || all_p2.is_empty() {
            return None;
        }
        let mut best = f64::NEG_INFINITY;
        for a1 in &all_p1 {
            for a2 in &all_p2 {
                let s = Self::compare_addresses(a1, a2);
                if s > best {
                    best = s;
                }
            }
        }
        Some(best)
    }

    /// Compare two place-of-birth `Address` values.
    ///
    /// Delegates to [`score_named_place`], which scores city via
    /// Jaro-Winkler and country via exact equality on the normalised
    /// string, blending them as `0.7 × city + 0.3 × country`.
    fn score_birth_place(worker1: &Worker, worker2: &Worker) -> Option<f64> {
        score_named_place(worker1.birth_place.as_ref()?, worker2.birth_place.as_ref()?)
    }

    /// Compare two place-of-death `Address` values.
    ///
    /// Symmetrical to [`Self::score_birth_place`] — death-place data
    /// in practice carries `city` and `country` only; we reuse the
    /// shared [`score_named_place`] helper.
    fn score_death_place(worker1: &Worker, worker2: &Worker) -> Option<f64> {
        score_named_place(worker1.death_place.as_ref()?, worker2.death_place.as_ref()?)
    }

    /// Compare two recorded dates of death using the same DOB
    /// transposition heuristic as [`Self::score_date_of_birth`]
    /// — exact equality scores `1.0`, day/month swap scores `0.5`,
    /// otherwise `0.0`. Day-month swaps are just as common a
    /// data-entry mistake on death certificates as on birth records.
    fn score_death_date(worker1: &Worker, worker2: &Worker) -> Option<f64> {
        Some(score_dob_pair(worker1.death_date?, worker2.death_date?))
    }

    fn compare_addresses(addr1: &Address, addr2: &Address) -> f64 {
        // Each sub-component contributes its own raw score in `[0.0, 1.0]`
        // and a weight. The final sub-score is the weight-renormalised
        // average — `Σ(score × weight) / Σ(weight)` — over the
        // sub-components that actually fired. This matches the
        // probabilistic outer pipeline (§12.3) and resolves §22 OQ-4:
        // postcode dominates (`0.5`), then city (`0.3`), then line 1
        // (`0.2`), regardless of how many components are populated.
        let mut weighted_sum = 0.0_f64;
        let mut total_weight = 0.0_f64;

        if let (Some(pc1), Some(pc2)) = (&addr1.postcode, &addr2.postcode) {
            let norm1 = Normalizer::normalize_postcode(pc1);
            let norm2 = Normalizer::normalize_postcode(pc2);
            weighted_sum += f64::from(norm1 == norm2) * 0.5;
            total_weight += 0.5;
        }

        if let (Some(city1), Some(city2)) = (&addr1.city, &addr2.city) {
            let norm1 = Normalizer::normalize_name(city1);
            let norm2 = Normalizer::normalize_name(city2);
            weighted_sum += Scorer::jaro_winkler_similarity(&norm1, &norm2) * 0.3;
            total_weight += 0.3;
        }

        if let (Some(line1), Some(line2)) = (&addr1.line1, &addr2.line1) {
            let parsed1 = Normalizer::parse_address_line(line1);
            let parsed2 = Normalizer::parse_address_line(line2);
            let street_sim = Scorer::jaro_winkler_similarity(&parsed1.street, &parsed2.street);
            let house_score = match (&parsed1.house_number, &parsed2.house_number) {
                (Some(a), Some(b)) => Some(f64::from(a == b)),
                _ => None,
            };
            let line1_score = match house_score {
                Some(h) => 0.6 * street_sim + 0.4 * h,
                None => street_sim,
            };
            weighted_sum += line1_score * 0.2;
            total_weight += 0.2;
        }

        if total_weight == 0.0 {
            0.5
        } else {
            weighted_sum / total_weight
        }
    }

    fn score_phone(&self, worker1: &Worker, worker2: &Worker) -> Option<f64> {
        let phone1 = worker1.phone.as_ref().or(worker1.mobile.as_ref())?;
        let phone2 = worker2.phone.as_ref().or(worker2.mobile.as_ref())?;

        let default = self.config.phone_default_country.as_deref();
        let e164_1 = Normalizer::normalize_phone_e164(phone1, default);
        let e164_2 = Normalizer::normalize_phone_e164(phone2, default);

        // Prefer the international-aware comparison: a French and a UK
        // number that share the same national-significant digits must not
        // collide. Fall back to the legacy national-significant form when
        // either side cannot be parsed to E.164 — preserving prior
        // behaviour for inputs the country table does not cover.
        if let (Some(a), Some(b)) = (&e164_1, &e164_2) {
            return Some(f64::from(a == b));
        }

        let norm1 = Normalizer::normalize_phone(phone1);
        let norm2 = Normalizer::normalize_phone(phone2);
        Some(f64::from(norm1 == norm2))
    }

    fn score_email(&self, worker1: &Worker, worker2: &Worker) -> Option<f64> {
        let email1 = worker1.email.as_ref()?;
        let email2 = worker2.email.as_ref()?;
        let fold = self.config.gmail_dot_folding;
        let canonical1 = Normalizer::normalize_email(email1, fold)?;
        let canonical2 = Normalizer::normalize_email(email2, fold)?;
        Some(f64::from(canonical1 == canonical2))
    }

    fn score_phonetic_names(worker1: &Worker, worker2: &Worker) -> Option<f64> {
        let p1_given_name = worker1.given_name.as_ref()?;
        let p1_given_name_phonetic = Normalizer::phonetic_code(p1_given_name);
        let p1_family_name = worker1.family_name.as_ref()?;
        let p1_family_name_phonetic = Normalizer::phonetic_code(p1_family_name);

        let p2_given_name = worker2.given_name.as_ref()?;
        let p2_given_name_phonetic = Normalizer::phonetic_code(p2_given_name);
        let p2_family_name = worker2.family_name.as_ref()?;
        let p2_family_name_phonetic = Normalizer::phonetic_code(p2_family_name);

        let given_name_match = f64::from(p1_given_name_phonetic == p2_given_name_phonetic);
        let family_name_match = f64::from(p1_family_name_phonetic == p2_family_name_phonetic);
        Some(f64::midpoint(given_name_match, family_name_match))
    }
}

/// One national-identifier scheme to compare between two workers: the same
/// scheme field on each worker plus the parser that canonicalises raw values.
type SchemePair<'a> = (
    &'a Option<String>,
    &'a Option<String>,
    fn(&str) -> Option<String>,
);

/// Return `true` iff both raw identifier strings parse via `parser` to the
/// same canonical form. Both `None` or a parse failure on either side
/// yields `false` — this helper backs deterministic identifier matching.
fn identifier_equal<F>(a: Option<&String>, b: Option<&String>, parser: F) -> bool
where
    F: Fn(&str) -> Option<String>,
{
    match (a, b) {
        (Some(x), Some(y)) => match (parser(x), parser(y)) {
            (Some(cx), Some(cy)) => cx == cy,
            _ => false,
        },
        _ => false,
    }
}

/// Return `Some(1.0)` if both inputs parse and are equal, `Some(0.0)` if
/// both parse but differ, or `None` if either input is absent or fails to
/// parse — mirroring the existing per-field scoring contract.
fn identifier_score<F>(a: Option<&String>, b: Option<&String>, parser: F) -> Option<f64>
where
    F: Fn(&str) -> Option<String>,
{
    if let (Some(x), Some(y)) = (a, b)
        && let (Some(cx), Some(cy)) = (parser(x), parser(y))
    {
        return Some(f64::from(cx == cy));
    }
    None
}

/// `true` iff the two passport-book lists share at least one
/// `(country, number)` pair after the canonicalisation performed by
/// [`crate::PassportBook::new`].
///
/// A single shared pair is sufficient: a worker with passports from
/// multiple countries matches another worker who carries any one of
/// those `(country, number)` pairs, regardless of issue date. This
/// captures the multi-country and time-varying nature of passport
/// book numbers (see [`crate::PassportBook`] for the design
/// rationale).
fn passport_books_share_pair(a: &[PassportBook], b: &[PassportBook]) -> bool {
    for ba in a {
        for bb in b {
            // SEC-M2: a blank country or number is not identity evidence.
            // `PassportBook` is deserialized with public fields, so a
            // crafted `{"country":"","number":""}` on both records would
            // otherwise short-circuit two *different* people to a 1.0 match.
            if ba.country.trim().is_empty() || ba.number.trim().is_empty() {
                continue;
            }
            if ba.country == bb.country && ba.number == bb.number {
                return true;
            }
        }
    }
    false
}

/// Score a pair of passport-book lists for the probabilistic
/// breakdown. Returns:
///
/// - `None` if either side has no books at all (the field is
///   irrelevant for this pair).
/// - `Some(1.0)` if at least one `(country, number)` pair is shared.
/// - `Some(0.0)` if both sides carry books but none share a pair.
fn score_passport_books(a: &[PassportBook], b: &[PassportBook]) -> Option<f64> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    Some(f64::from(passport_books_share_pair(a, b)))
}

/// Score a pair of `NaiveDate` values for the date-of-birth component.
///
/// - `1.0` when the dates are exactly equal.
/// - `0.5` when **swapping the day and month on one side** yields the
///   other side, and the years agree. This catches the DD/MM ↔ MM/DD
///   data-entry bug that is common in cross-border or multi-system data
///   flows; `1995-01-10` vs `1995-10-01` is the canonical example.
/// - `0.0` otherwise.
///
/// The transposition path is conservative by design: it requires the
/// years to match, and it relies on `NaiveDate::from_ymd_opt` to validate
/// the swapped form (so it cannot fire on a day greater than 12, on a
/// month longer than the original day's day-count, or across years).
/// Compare two named-place [`Address`] values (typically `city` and
/// `country` populated). Used by both birth-place and death-place
/// scoring — Jaro-Winkler on the normalised city blended with exact
/// equality on the normalised country: `0.7 × city + 0.3 × country`
/// when both are present, otherwise whichever single signal is
/// available. Returns `None` if neither sub-field is populated on
/// both sides.
fn score_named_place(a: &Address, b: &Address) -> Option<f64> {
    let city = match (&a.city, &b.city) {
        (Some(c1), Some(c2)) => Some(Scorer::jaro_winkler_similarity(
            &Normalizer::normalize_name(c1),
            &Normalizer::normalize_name(c2),
        )),
        _ => None,
    };
    let country = match (&a.country, &b.country) {
        (Some(c1), Some(c2)) => Some(f64::from(
            Normalizer::normalize_name(c1) == Normalizer::normalize_name(c2),
        )),
        _ => None,
    };
    match (city, country) {
        (Some(c), Some(co)) => Some(0.7 * c + 0.3 * co),
        (Some(c), None) => Some(c),
        (None, Some(co)) => Some(co),
        (None, None) => None,
    }
}

/// Score a pair of dates of birth.
///
/// Returns `1.0` for an exact match, `0.5` when the day and month are
/// transposed within the same year (a common data-entry error), and `0.0`
/// otherwise.
fn score_dob_pair(dob1: NaiveDate, dob2: NaiveDate) -> f64 {
    if dob1 == dob2 {
        return 1.0;
    }
    if dob1.year() == dob2.year()
        && let Some(swapped) = NaiveDate::from_ymd_opt(dob1.year(), dob1.day(), dob1.month())
        && swapped == dob2
    {
        return 0.5;
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Gender;

    fn dob(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
    }

    /// Assert two `f64` values are equal within floating-point tolerance.
    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < f64::EPSILON, "{a} != {b}");
    }

    // ---------- MatchConfig presets ----------

    #[test]
    fn config_default_values() {
        let c = MatchConfig::default();
        assert!((c.match_threshold - 0.85).abs() < 1e-9);
        assert!((c.uk_nhs_number_weight - 0.30).abs() < 1e-9);
        assert!(c.use_phonetic_matching);
        assert!(!c.strict_mode);
    }

    #[test]
    fn config_strict_raises_threshold_and_sets_flag() {
        let c = MatchConfig::strict();
        assert!((c.match_threshold - 0.95).abs() < 1e-9);
        assert!(c.strict_mode);
    }

    // ---------- MatchConfig serde ----------

    #[test]
    fn config_default_round_trips_through_json() {
        let cfg = MatchConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialise");
        let back: MatchConfig = serde_json::from_str(&json).expect("deserialise");
        assert!((cfg.match_threshold - back.match_threshold).abs() < 1e-12);
        assert!((cfg.uk_nhs_number_weight - back.uk_nhs_number_weight).abs() < 1e-12);
        assert_eq!(cfg.use_phonetic_matching, back.use_phonetic_matching);
        assert!(matches!(back.name_algorithm, SimilarityAlgorithm::Combined));
        assert_eq!(cfg.strict_mode, back.strict_mode);
        assert_eq!(cfg.nickname_table, back.nickname_table);
        assert_eq!(cfg.gmail_dot_folding, back.gmail_dot_folding);
        assert_eq!(cfg.phone_default_country, back.phone_default_country);
    }

    #[test]
    fn config_strict_round_trips_through_json() {
        let cfg = MatchConfig::strict();
        let json = serde_json::to_string(&cfg).expect("serialise");
        let back: MatchConfig = serde_json::from_str(&json).expect("deserialise");
        assert!((back.match_threshold - 0.95).abs() < 1e-12);
        assert!(back.strict_mode);
    }

    #[test]
    fn config_lenient_round_trips_through_json() {
        let cfg = MatchConfig::lenient();
        let json = serde_json::to_string(&cfg).expect("serialise");
        let back: MatchConfig = serde_json::from_str(&json).expect("deserialise");
        assert!((back.match_threshold - 0.75).abs() < 1e-12);
    }

    #[test]
    fn config_partial_json_fills_missing_fields_from_default() {
        // `#[serde(default)]` on the struct: a JSON document carrying
        // only the fields the caller cares about deserialises with the
        // rest filled in from `MatchConfig::default()`.
        let partial = r#"{"match_threshold": 0.80, "gmail_dot_folding": true}"#;
        let cfg: MatchConfig = serde_json::from_str(partial).expect("partial json");
        assert!((cfg.match_threshold - 0.80).abs() < 1e-12);
        assert!(cfg.gmail_dot_folding);
        // Other fields come from default().
        assert!((cfg.uk_nhs_number_weight - 0.30).abs() < 1e-12);
        assert!(matches!(cfg.name_algorithm, SimilarityAlgorithm::Combined));
        assert_eq!(cfg.phone_default_country.as_deref(), Some("GB"));
    }

    #[test]
    fn similarity_algorithm_round_trips_through_json() {
        for alg in [
            SimilarityAlgorithm::JaroWinkler,
            SimilarityAlgorithm::Levenshtein,
            SimilarityAlgorithm::Exact,
            SimilarityAlgorithm::Combined,
        ] {
            let json = serde_json::to_string(&alg).expect("serialise");
            let back: SimilarityAlgorithm = serde_json::from_str(&json).expect("deserialise");
            assert_eq!(alg, back);
        }
    }

    #[test]
    fn config_lenient_lowers_threshold() {
        let c = MatchConfig::lenient();
        assert!((c.match_threshold - 0.75).abs() < 1e-9);
        assert!(c.use_phonetic_matching);
    }

    // ---------- probabilistic match ----------

    #[test]
    fn exact_clone_is_a_match() {
        let p = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .uk_nhs_number("9434765919")
            .build();
        let result = MatchingEngine::default_config().match_workers(&p, &p.clone());
        assert!(result.is_match);
        assert!(result.score > 0.95);
    }

    #[test]
    fn fuzzy_given_name_still_matches() {
        let a = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        let b = Worker::builder()
            .given_name("Jon")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        let r = MatchingEngine::default_config().match_workers(&a, &b);
        assert!(r.is_match);
        assert!(r.score > 0.85);
    }

    #[test]
    fn completely_different_patients_do_not_match() {
        let a = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        let b = Worker::builder()
            .given_name("Jane")
            .family_name("Doe")
            .date_of_birth(dob(1990, 3, 20))
            .gender(Gender::Female)
            .build();
        let r = MatchingEngine::default_config().match_workers(&a, &b);
        assert!(!r.is_match);
        assert!(r.score < 0.5);
    }

    #[test]
    fn no_overlapping_fields_returns_zero_score() {
        // Neither side has any scoreable field on both records.
        let a = Worker::builder().given_name("Solo").build();
        let b = Worker::builder().family_name("Only").build();
        let r = MatchingEngine::default_config().match_workers(&a, &b);
        approx(r.score, 0.0);
        assert!(!r.is_match);
    }

    #[test]
    fn unparseable_uk_nhs_number_is_none_not_zero() {
        let a = Worker::builder()
            .uk_nhs_number("not-a-number")
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .build();
        let b = Worker::builder()
            .uk_nhs_number("also-not-a-number")
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .build();
        let r = MatchingEngine::default_config().match_workers(&a, &b);
        assert_eq!(
            r.breakdown.uk_nhs_number_score, None,
            "unparseable NHS numbers should not produce a 0.0 penalty"
        );
        assert!(r.is_match, "should still match on demographics");
    }

    #[test]
    fn missing_field_yields_none_in_breakdown() {
        let a = Worker::builder().given_name("Ada").build();
        let b = Worker::builder()
            .given_name("Ada")
            .family_name("Lovelace")
            .build();
        let r = MatchingEngine::default_config().match_workers(&a, &b);
        assert!(r.breakdown.given_name_score.is_some());
        assert!(r.breakdown.family_name_score.is_none());
    }

    #[test]
    fn phonetic_match_is_a_bonus_not_a_penalty() {
        // Identical names should not be hurt when phonetic matching is on.
        let p = Worker::builder()
            .given_name("Stephen")
            .family_name("Jones")
            .build();
        let with_phon = MatchingEngine::new(MatchConfig {
            use_phonetic_matching: true,
            ..MatchConfig::default()
        })
        .match_workers(&p, &p.clone());
        let without_phon = MatchingEngine::new(MatchConfig {
            use_phonetic_matching: false,
            ..MatchConfig::default()
        })
        .match_workers(&p, &p.clone());
        assert!(with_phon.score >= without_phon.score);
    }

    #[test]
    fn phonetic_score_disabled_when_config_off() {
        let p = Worker::builder()
            .given_name("Steven")
            .family_name("Smith")
            .build();
        let q = Worker::builder()
            .given_name("Stephen")
            .family_name("Smyth")
            .build();
        let r = MatchingEngine::new(MatchConfig {
            use_phonetic_matching: false,
            ..MatchConfig::default()
        })
        .match_workers(&p, &q);
        assert_eq!(r.breakdown.phonetic_name_score, None);
    }

    #[test]
    fn address_with_no_subfields_is_neutral_half() {
        let a = Address::new();
        let b = Address::new();
        let score = MatchingEngine::compare_addresses(&a, &b);
        assert!(
            (score - 0.5).abs() < 1e-9,
            "empty addresses must be neutral (0.5), got {score}"
        );
    }

    #[test]
    fn address_postcode_dominates() {
        let mut a = Address::new();
        a.postcode = Some("CF10 1AA".into());
        let mut b = Address::new();
        b.postcode = Some("CF10 1AA".into());
        let s = MatchingEngine::compare_addresses(&a, &b);
        assert!(s > 0.0);
    }

    // ---------- deterministic match ----------

    #[test]
    fn deterministic_uk_nhs_match_overrides_demographics() {
        let a = Worker::builder()
            .uk_nhs_number("943 476 5919")
            .given_name("Bob")
            .build();
        let b = Worker::builder()
            .uk_nhs_number("9434765919")
            .given_name("Alice") // intentionally different
            .build();
        assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_demographics_match_when_all_align() {
        let p = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        assert!(MatchingEngine::default_config().deterministic_match(&p, &p.clone()));
    }

    /// SEC-M2: two DIFFERENT workers sharing only a blank / punctuation name
    /// (which normalises to empty) plus a DOB must NOT deterministically
    /// match — an empty normalised name is not identity evidence.
    #[test]
    fn deterministic_demographics_reject_blank_names() {
        let a = Worker::builder()
            .given_name("   ")
            .family_name("--")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        let b = Worker::builder()
            .given_name(".")
            .family_name("  ")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    /// SEC-M2: a blank passport book (empty country + number) — reachable
    /// only by deserializing a crafted payload, since `PassportBook::new`
    /// rejects it — must NOT short-circuit two different workers to a match.
    #[test]
    fn deterministic_reject_blank_passport_pair() {
        let blank = || PassportBook {
            country: String::new(),
            number: String::new(),
            issued: None,
            expires: None,
        };
        let a = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .passport_books(vec![blank()])
            .build();
        let b = Worker::builder()
            .given_name("Jane")
            .family_name("Doe")
            .date_of_birth(dob(1990, 1, 1))
            .passport_books(vec![blank()])
            .build();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_demographics_tolerates_missing_gender() {
        let a = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .build();
        let b = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_rejects_when_dob_differs() {
        let a = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        let b = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 16)) // off by one day
            .gender(Gender::Male)
            .build();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_rejects_when_gender_differs() {
        let a = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        let b = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Female)
            .build();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    // ---------- strict_mode enforcement ----------

    #[test]
    fn strict_mode_requires_deterministic_for_is_match() {
        // A fuzzy match that lifts the score above the strict threshold
        // (0.95) but is NOT a deterministic match must produce
        // `is_match = false` under strict mode.
        let cfg = MatchConfig {
            // Lower threshold so the fuzzy score clears it.
            match_threshold: 0.85,
            strict_mode: true,
            ..MatchConfig::default()
        };
        let p1 = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        let p2 = Worker::builder()
            .given_name("Jon") // typo — fuzzy match
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        let engine = MatchingEngine::new(cfg);
        let r = engine.match_workers(&p1, &p2);
        assert!(
            r.score >= 0.85,
            "fuzzy score should clear lowered threshold"
        );
        // Normalised given names differ ("john" vs "jon"), so the
        // demographic-tuple deterministic branch fails.
        assert!(!engine.deterministic_match(&p1, &p2));
        assert!(
            !r.is_match,
            "strict mode must reject fuzzy-only matches even above threshold"
        );
    }

    #[test]
    fn strict_mode_accepts_when_deterministic_holds() {
        // An identifier-driven deterministic match also clears any
        // sensible threshold. Strict mode must allow it.
        let cfg = MatchConfig::strict();
        let p1 = Worker::builder()
            .uk_nhs_number("9434765919")
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        let p2 = p1.clone();
        let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
        assert!(r.is_match);
    }

    #[test]
    fn non_strict_mode_accepts_fuzzy_match_above_threshold() {
        // Sanity: in default (non-strict) config, a fuzzy match above
        // the threshold is still accepted. This pins that the strict
        // logic only activates when explicitly opted in.
        let p1 = Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        let p2 = Worker::builder()
            .given_name("Jon")
            .family_name("Smith")
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        let r = MatchingEngine::default_config().match_workers(&p1, &p2);
        assert!(r.is_match);
    }

    // ---------- batch API: match_one_to_many / rank_one_to_many ----------

    #[test]
    fn match_one_to_many_empty_candidates_yields_empty_vec() {
        let engine = MatchingEngine::default_config();
        let q = Worker::builder().given_name("Solo").build();
        assert!(engine.match_one_to_many(&q, &[]).is_empty());
    }

    #[test]
    fn match_one_to_many_preserves_order() {
        let engine = MatchingEngine::default_config();
        let q = Worker::builder()
            .given_name("Ada")
            .family_name("Lovelace")
            .build();
        let candidates = vec![
            Worker::builder()
                .given_name("Grace")
                .family_name("Hopper")
                .build(),
            q.clone(),
            Worker::builder()
                .given_name("Alan")
                .family_name("Turing")
                .build(),
        ];
        let r = engine.match_one_to_many(&q, &candidates);
        assert_eq!(r.len(), 3);
        // Index 1 is the perfect clone → highest score.
        assert!(r[1].score > r[0].score);
        assert!(r[1].score > r[2].score);
        assert!(r[1].is_match);
    }

    #[test]
    fn match_one_to_many_matches_individual_scoring() {
        // Calling match_one_to_many must produce the same per-candidate
        // score as calling match_workers individually.
        let engine = MatchingEngine::default_config();
        let q = Worker::builder()
            .given_name("Ada")
            .family_name("Lovelace")
            .build();
        let candidates = vec![
            Worker::builder()
                .given_name("Ada")
                .family_name("Lovelace")
                .build(),
            Worker::builder()
                .given_name("Alan")
                .family_name("Turing")
                .build(),
        ];
        let batch = engine.match_one_to_many(&q, &candidates);
        for (i, c) in candidates.iter().enumerate() {
            let individual = engine.match_workers(&q, c);
            assert!((batch[i].score - individual.score).abs() < 1e-12);
            assert_eq!(batch[i].is_match, individual.is_match);
            assert_eq!(batch[i].confidence, individual.confidence);
        }
    }

    #[test]
    fn rank_one_to_many_sorts_by_score_descending() {
        let engine = MatchingEngine::default_config();
        let q = Worker::builder()
            .given_name("Ada")
            .family_name("Lovelace")
            .build();
        let candidates = vec![
            Worker::builder()
                .given_name("Grace")
                .family_name("Hopper")
                .build(),
            q.clone(),
            Worker::builder()
                .given_name("Alan")
                .family_name("Turing")
                .build(),
        ];
        let ranked = engine.rank_one_to_many(&q, &candidates);
        assert_eq!(ranked.len(), 3);
        // Best match must be index 1 (the clone).
        assert_eq!(ranked[0].0, 1);
        // Ordering invariant.
        for w in ranked.windows(2) {
            assert!(w[0].1.score >= w[1].1.score);
        }
    }

    #[test]
    fn rank_one_to_many_breaks_ties_by_ascending_original_index() {
        // Two candidates that produce the same score: ranking must keep
        // the earlier original index first for determinism.
        let engine = MatchingEngine::default_config();
        let q = Worker::builder()
            .given_name("Ada")
            .family_name("Lovelace")
            .build();
        let twin = q.clone();
        let candidates = vec![twin.clone(), twin.clone(), twin.clone()];
        let ranked = engine.rank_one_to_many(&q, &candidates);
        assert_eq!(ranked.len(), 3);
        // All three score identically; order MUST be 0, 1, 2.
        assert_eq!(ranked[0].0, 0);
        assert_eq!(ranked[1].0, 1);
        assert_eq!(ranked[2].0, 2);
    }

    #[test]
    fn match_one_to_many_is_deterministic_across_calls() {
        let engine = MatchingEngine::default_config();
        let q = Worker::builder().given_name("X").family_name("Y").build();
        let candidates = vec![
            Worker::builder().given_name("X").family_name("Y").build(),
            Worker::builder().given_name("A").family_name("B").build(),
        ];
        let a = engine.match_one_to_many(&q, &candidates);
        let b = engine.match_one_to_many(&q, &candidates);
        for i in 0..a.len() {
            assert!((a[i].score - b[i].score).abs() < 1e-12);
        }
    }

    // ---------- score_dob_pair (transposition heuristic) ----------

    #[test]
    fn dob_pair_exact_equal_scores_one() {
        approx(score_dob_pair(dob(1995, 1, 10), dob(1995, 1, 10)), 1.0);
    }

    #[test]
    fn dob_pair_day_month_swap_scores_half() {
        // 1995-01-10 vs 1995-10-01 — classic DD/MM ↔ MM/DD bug.
        approx(score_dob_pair(dob(1995, 1, 10), dob(1995, 10, 1)), 0.5);
        // Symmetric.
        approx(score_dob_pair(dob(1995, 10, 1), dob(1995, 1, 10)), 0.5);
    }

    #[test]
    fn dob_pair_swap_requires_year_to_match() {
        // Same day/month layout but different year — not a transposition.
        approx(score_dob_pair(dob(1995, 1, 10), dob(1996, 10, 1)), 0.0);
    }

    #[test]
    fn dob_pair_swap_skipped_when_day_exceeds_12() {
        // 1995-01-25: the swap target (1995, month=25, day=1) is not a
        // valid calendar date, so `NaiveDate::from_ymd_opt` returns None
        // and the heuristic does not fire. Compared against any other
        // valid date — including the same month with a different day —
        // the score must be 0.0.
        approx(score_dob_pair(dob(1995, 1, 25), dob(1995, 1, 26)), 0.0);
        approx(score_dob_pair(dob(1995, 1, 25), dob(1995, 2, 25)), 0.0);
    }

    #[test]
    fn dob_pair_unrelated_dates_score_zero() {
        approx(score_dob_pair(dob(1995, 1, 10), dob(1980, 6, 30)), 0.0);
        approx(score_dob_pair(dob(1995, 1, 10), dob(1995, 1, 11)), 0.0);
    }

    #[test]
    fn dob_pair_day_equals_month_collapses_to_exact() {
        // 1995-05-05: swap is a no-op; exact-match branch wins.
        approx(score_dob_pair(dob(1995, 5, 5), dob(1995, 5, 5)), 1.0);
        // 1995-05-05 vs 1995-05-06: not a transposition; 0.0.
        approx(score_dob_pair(dob(1995, 5, 5), dob(1995, 5, 6)), 0.0);
    }

    #[test]
    fn dob_pair_invalid_swap_target_does_not_panic() {
        // 2003-02-30 is never constructed (chrono rejects it), so the
        // helper only ever receives valid dates. But test a near-edge:
        // swap of Feb 29 (leap) vs swap target.
        approx(score_dob_pair(dob(2000, 2, 29), dob(2000, 2, 29)), 1.0);
        // 2000-02-12 swap = 2000-12-02, valid.
        approx(score_dob_pair(dob(2000, 2, 12), dob(2000, 12, 2)), 0.5);
    }

    #[test]
    fn deterministic_match_still_rejects_transposed_dob() {
        // The transposition heuristic only lifts the probabilistic
        // score; deterministic matching by demographics still requires
        // exact DOB equality.
        let a = Worker::builder()
            .given_name("Thomas")
            .family_name("Price")
            .date_of_birth(dob(1995, 1, 10))
            .gender(Gender::Male)
            .build();
        let b = Worker::builder()
            .given_name("Thomas")
            .family_name("Price")
            .date_of_birth(dob(1995, 10, 1))
            .gender(Gender::Male)
            .build();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn transposed_dob_lifts_probabilistic_score_above_zero() {
        let a = Worker::builder()
            .given_name("Thomas")
            .family_name("Price")
            .date_of_birth(dob(1995, 1, 10))
            .gender(Gender::Male)
            .build();
        let b = Worker::builder()
            .given_name("Thomas")
            .family_name("Price")
            .date_of_birth(dob(1995, 10, 1))
            .gender(Gender::Male)
            .build();
        let r = MatchingEngine::default_config().match_workers(&a, &b);
        assert_eq!(r.breakdown.date_of_birth_score, Some(0.5));
        // Demographics agree on name + gender; DOB contributes 0.5
        // partial credit; overall score must lift above what a 0.0 DOB
        // would have produced.
        assert!(r.score > 0.6);
    }

    // ---------- Confidence ----------

    #[test]
    fn confidence_band_boundaries_are_inclusive_on_the_low_side() {
        assert_eq!(Confidence::from_score(0.90), Confidence::High);
        assert_eq!(Confidence::from_score(0.89), Confidence::Medium);
        assert_eq!(Confidence::from_score(0.75), Confidence::Medium);
        assert_eq!(Confidence::from_score(0.74), Confidence::Low);
    }

    #[test]
    fn confidence_handles_degenerate_inputs_gracefully() {
        assert_eq!(Confidence::from_score(f64::NAN), Confidence::Low);
        assert_eq!(Confidence::from_score(-1.0), Confidence::Low);
        assert_eq!(Confidence::from_score(2.0), Confidence::High);
    }

    #[test]
    fn confidence_is_independent_of_match_threshold() {
        // Strict (threshold 0.95) and lenient (threshold 0.75) produce
        // the same Confidence band for the same score — bands are
        // fixed, only is_match follows the threshold.
        let p = Worker::builder()
            .given_name("Ada")
            .family_name("Lovelace")
            .build();
        let strict = MatchingEngine::new(MatchConfig::strict()).match_workers(&p, &p.clone());
        let lenient = MatchingEngine::new(MatchConfig::lenient()).match_workers(&p, &p.clone());
        assert_eq!(strict.confidence, lenient.confidence);
        // And the exact-clone score should land in High.
        assert_eq!(strict.confidence, Confidence::High);
    }

    #[test]
    fn match_result_carries_confidence() {
        let p = Worker::builder()
            .given_name("Ada")
            .family_name("Lovelace")
            .build();
        let r = MatchingEngine::default_config().match_workers(&p, &p.clone());
        assert_eq!(r.confidence, Confidence::High);
    }

    #[test]
    fn match_result_confidence_round_trips_via_serde() {
        let p = Worker::builder()
            .given_name("Ada")
            .family_name("Lovelace")
            .build();
        let r = MatchingEngine::default_config().match_workers(&p, &p.clone());
        let json = serde_json::to_string(&r).unwrap();
        let back: MatchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r.confidence, back.confidence);
    }

    #[test]
    fn deterministic_rejects_when_names_missing() {
        let a = Worker::builder()
            .date_of_birth(dob(1980, 5, 15))
            .gender(Gender::Male)
            .build();
        let b = a.clone();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    // ---------- score_named_place ----------

    #[test]
    fn score_named_place_both_subfields_blend_seven_three() {
        let a = Address::new().with_city("Paris").with_country("France");
        let b = Address::new().with_city("Paris").with_country("France");
        assert_eq!(score_named_place(&a, &b), Some(1.0));
    }

    #[test]
    fn score_named_place_city_only_matches_returns_city_score() {
        let a = Address::new().with_city("Cardiff");
        let b = Address::new().with_city("Cardiff");
        assert_eq!(score_named_place(&a, &b), Some(1.0));
    }

    #[test]
    fn score_named_place_country_only_matches_returns_country_score() {
        let a = Address::new().with_country("Wales");
        let b = Address::new().with_country("Wales");
        assert_eq!(score_named_place(&a, &b), Some(1.0));
    }

    #[test]
    fn score_named_place_empty_returns_none() {
        let a = Address::new();
        let b = Address::new();
        assert_eq!(score_named_place(&a, &b), None);
    }

    #[test]
    fn score_named_place_city_partial_country_mismatch_blends() {
        let a = Address::new().with_city("Paris").with_country("France");
        let b = Address::new().with_city("Paris").with_country("USA");
        let s = score_named_place(&a, &b).unwrap();
        assert!((s - 0.7).abs() < 1e-9);
    }

    // ---------- death_date / death_place wiring ----------

    #[test]
    fn match_config_default_carries_death_weights() {
        let c = MatchConfig::default();
        assert!((c.death_date_weight - 0.10).abs() < 1e-9);
        assert!((c.death_place_weight - 0.05).abs() < 1e-9);
    }

    #[test]
    fn breakdown_carries_death_date_score_when_both_sides_present() {
        let p1 = Worker::builder()
            .given_name("X")
            .family_name("Y")
            .death_date(dob(2020, 3, 14))
            .build();
        let p2 = Worker::builder()
            .given_name("X")
            .family_name("Y")
            .death_date(dob(2020, 3, 14))
            .build();
        let r = MatchingEngine::default_config().match_workers(&p1, &p2);
        assert_eq!(r.breakdown.death_date_score, Some(1.0));
    }

    // ---------- T-3 / OQ-4 address sub-score arithmetic ----------

    #[test]
    fn address_subscore_exact_postcode_plus_slightly_different_street_clears_seven_tenths() {
        // OQ-4 acceptance: an identical postcode plus a slightly
        // different street MUST score ≥ 0.7. Under the old
        // `sum / count` formula this would have scored ~ 0.33.
        let a = Address::new();
        let a = Address {
            line1: Some("10 High Street".into()),
            postcode: Some("CF10 1AA".into()),
            city: Some("Cardiff".into()),
            ..a
        };
        let b = Address {
            line1: Some("10 High Road".into()),
            postcode: Some("CF10 1AA".into()),
            city: Some("Cardiff".into()),
            ..Address::new()
        };
        let s = MatchingEngine::compare_addresses(&a, &b);
        assert!(
            s >= 0.7,
            "exact postcode + slight street typo should score ≥ 0.7: {s}"
        );
    }

    #[test]
    fn address_subscore_postcode_only_match_returns_one() {
        // Only postcode populated on both sides → weighted average
        // collapses to the postcode score alone.
        let a = Address {
            postcode: Some("CF10 1AA".into()),
            ..Address::new()
        };
        let b = Address {
            postcode: Some("CF10 1AA".into()),
            ..Address::new()
        };
        let s = MatchingEngine::compare_addresses(&a, &b);
        assert!((s - 1.0).abs() < 1e-9, "postcode-only match: {s}");
    }

    #[test]
    fn address_subscore_no_comparable_fields_returns_neutral_half() {
        // Neither side populated on any sub-field — neutral 0.5.
        let s = MatchingEngine::compare_addresses(&Address::new(), &Address::new());
        assert!((s - 0.5).abs() < 1e-9, "neutral fallback: {s}");
    }

    #[test]
    fn address_subscore_postcode_match_plus_street_mismatch_dominated_by_postcode() {
        // Postcode (0.5) match + line1 (0.2) mismatch → 0.5/0.7 ≈ 0.714.
        let a = Address {
            postcode: Some("CF10 1AA".into()),
            line1: Some("Wholly Different".into()),
            ..Address::new()
        };
        let b = Address {
            postcode: Some("CF10 1AA".into()),
            line1: Some("Completely Other".into()),
            ..Address::new()
        };
        let s = MatchingEngine::compare_addresses(&a, &b);
        assert!(s >= 0.5, "postcode should still dominate: {s}");
    }

    #[test]
    fn breakdown_omits_death_place_score_when_one_side_absent() {
        let p1 = Worker::builder()
            .given_name("X")
            .family_name("Y")
            .death_place(Address::new().with_city("Cambridge"))
            .build();
        let p2 = Worker::builder().given_name("X").family_name("Y").build();
        let r = MatchingEngine::default_config().match_workers(&p1, &p2);
        assert_eq!(r.breakdown.death_place_score, None);
    }
}
