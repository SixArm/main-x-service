#![warn(clippy::pedantic)]

//! Example showing custom matching configuration

use worker_matcher::{
    Gender, MatchConfig, MatchingEngine, NicknameTable, SimilarityAlgorithm, Worker,
};

/// The two sample workers compared by every configuration below:
/// "Robert Jones" against "Bob Jones" (a nickname), same DOB and gender.
fn build_sample_pair() -> (Worker, Worker) {
    let robert = Worker::builder()
        .given_name("Robert")
        .family_name("Jones")
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(1975, 6, 20).unwrap())
        .gender(Gender::Male)
        .build();

    let bob = Worker::builder()
        .given_name("Bob") // Nickname
        .family_name("Jones")
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(1975, 6, 20).unwrap())
        .gender(Gender::Male)
        .build();

    (robert, bob)
}

/// Custom configuration that prioritises demographic data over national
/// identifiers (all identifier weights reduced; name + DOB weights raised).
fn build_custom_config() -> MatchConfig {
    MatchConfig {
        match_threshold: 0.80,
        uk_nhs_number_weight: 0.10, // Reduced weight
        fr_nir_weight: 0.10,
        es_tsi_weight: 0.10,
        ie_ihi_weight: 0.10,
        uk_hc_number_weight: 0.10,
        us_ssn_weight: 0.10,
        au_ihi_weight: 0.10,
        de_kvnr_weight: 0.10,
        it_cf_weight: 0.10,
        nl_bsn_weight: 0.10,
        se_personnummer_weight: 0.10,
        uk_chi_number_weight: 0.10,
        be_nn_weight: 0.10,
        bg_egn_weight: 0.10,
        cz_rc_weight: 0.10,
        dk_cpr_weight: 0.10,
        ee_ik_weight: 0.10,
        es_dni_weight: 0.10,
        fi_hetu_weight: 0.10,
        hr_oib_weight: 0.10,
        is_kt_weight: 0.10,
        lt_ak_weight: 0.10,
        lv_pk_weight: 0.10,
        mt_id_weight: 0.10,
        no_fnr_weight: 0.10,
        pl_pesel_weight: 0.10,
        ro_cnp_weight: 0.10,
        si_emso_weight: 0.10,
        sk_rc_weight: 0.10,
        uk_nino_weight: 0.10,
        gr_dss_weight: 0.10,
        li_id_weight: 0.10,
        nl_id_weight: 0.10,
        pl_nip_weight: 0.10,
        pt_nif_weight: 0.10,
        br_cpf_weight: 0.10,
        cn_rrn_weight: 0.10,
        in_aadhaar_weight: 0.10,
        jp_my_number_weight: 0.10,
        mx_curp_weight: 0.10,
        nz_nhi_weight: 0.10,
        za_id_weight: 0.10,
        passport_book_weight: 0.10,
        given_name_weight: 0.20,    // Increased weight
        family_name_weight: 0.25,   // Increased weight
        date_of_birth_weight: 0.25, // Increased weight
        gender_weight: 0.10,
        blood_type_weight: 0.10,
        multiple_birth_weight: 0.05,
        address_weight: 0.05,
        birth_place_weight: 0.05,
        death_date_weight: 0.10,
        death_place_weight: 0.05,
        phone_weight: 0.05,
        email_weight: 0.05,
        use_phonetic_matching: true,
        name_algorithm: SimilarityAlgorithm::JaroWinkler,
        strict_mode: false,
        nickname_table: NicknameTable::english(),
        gmail_dot_folding: false,
        phone_default_country: Some("GB".into()),
    }
}

/// National-identifier-focused configuration (all identifier weights very
/// high; demographic weights minimal; strict, exact-name matching).
fn build_nhs_config() -> MatchConfig {
    MatchConfig {
        match_threshold: 0.90,
        uk_nhs_number_weight: 0.60, // Very high weight
        fr_nir_weight: 0.60,
        es_tsi_weight: 0.60,
        ie_ihi_weight: 0.60,
        uk_hc_number_weight: 0.60,
        us_ssn_weight: 0.60,
        au_ihi_weight: 0.60,
        de_kvnr_weight: 0.60,
        it_cf_weight: 0.60,
        nl_bsn_weight: 0.60,
        se_personnummer_weight: 0.60,
        uk_chi_number_weight: 0.60,
        be_nn_weight: 0.60,
        bg_egn_weight: 0.60,
        cz_rc_weight: 0.60,
        dk_cpr_weight: 0.60,
        ee_ik_weight: 0.60,
        es_dni_weight: 0.60,
        fi_hetu_weight: 0.60,
        hr_oib_weight: 0.60,
        is_kt_weight: 0.60,
        lt_ak_weight: 0.60,
        lv_pk_weight: 0.60,
        mt_id_weight: 0.60,
        no_fnr_weight: 0.60,
        pl_pesel_weight: 0.60,
        ro_cnp_weight: 0.60,
        si_emso_weight: 0.60,
        sk_rc_weight: 0.60,
        uk_nino_weight: 0.60,
        gr_dss_weight: 0.60,
        li_id_weight: 0.60,
        nl_id_weight: 0.60,
        pl_nip_weight: 0.60,
        pt_nif_weight: 0.60,
        br_cpf_weight: 0.60,
        cn_rrn_weight: 0.60,
        in_aadhaar_weight: 0.60,
        jp_my_number_weight: 0.60,
        mx_curp_weight: 0.60,
        nz_nhi_weight: 0.60,
        za_id_weight: 0.60,
        passport_book_weight: 0.60,
        given_name_weight: 0.10,
        family_name_weight: 0.10,
        date_of_birth_weight: 0.10,
        gender_weight: 0.05,
        blood_type_weight: 0.05,
        multiple_birth_weight: 0.05,
        address_weight: 0.025,
        birth_place_weight: 0.025,
        death_date_weight: 0.05,
        death_place_weight: 0.025,
        phone_weight: 0.025,
        email_weight: 0.025,
        use_phonetic_matching: false,
        name_algorithm: SimilarityAlgorithm::Exact,
        strict_mode: true,
        nickname_table: NicknameTable::empty(),
        gmail_dot_folding: false,
        phone_default_country: Some("GB".into()),
    }
}

/// Print the score + match outcome for one named configuration.
fn report(label: &str, engine: &MatchingEngine, worker1: &Worker, worker2: &Worker) {
    let result = engine.match_workers(worker1, worker2);
    println!("{label}");
    println!("  Score: {:.2}%", result.score * 100.0);
    println!("  Match: {}", result.is_match);
    println!();
}

fn main() {
    println!("=== Custom Configuration Example ===\n");

    let (worker1, worker2) = build_sample_pair();

    println!("Worker 1: Robert Jones");
    println!("Worker 2: Bob Jones (nickname)");
    println!();

    report(
        "Default Config:",
        &MatchingEngine::default_config(),
        &worker1,
        &worker2,
    );
    report(
        "Strict Config (threshold: 0.95):",
        &MatchingEngine::new(MatchConfig::strict()),
        &worker1,
        &worker2,
    );
    report(
        "Lenient Config (threshold: 0.75):",
        &MatchingEngine::new(MatchConfig::lenient()),
        &worker1,
        &worker2,
    );
    report(
        "Custom Config (demographics prioritized):",
        &MatchingEngine::new(build_custom_config()),
        &worker1,
        &worker2,
    );

    // National-identifier-focused configuration.
    let nhs_engine = MatchingEngine::new(build_nhs_config());
    let nhs_result = nhs_engine.match_workers(&worker1, &worker2);
    println!("NHS-Focused Config:");
    println!("  Score: {:.2}%", nhs_result.score * 100.0);
    println!("  Match: {}", nhs_result.is_match);
    println!("  Note: No NHS numbers provided, so match relies on names/DOB");
}
