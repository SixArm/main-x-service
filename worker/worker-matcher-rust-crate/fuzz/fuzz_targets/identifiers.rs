//! T-35 fuzz target: every national-identifier parser + passport-format
//! validator in `src/identifiers.rs` over arbitrary UTF-8.
//!
//! These are the crate's most string-parsing-heavy attack surface — 42
//! per-scheme personal-identifier parsers plus 9 per-country passport
//! validators, each running its own regex/format/check-digit logic on
//! caller-supplied text. This feeds every one of them the same arbitrary
//! input and asserts only the never-panic invariant
//! (`agents/share/security.md` invariant 2); it does not assert anything
//! about cross-scheme equality, since these are pure `&str -> Option<String>`
//! parsers with no comparison surface of their own — cross-scheme
//! false-equality is a matcher-level property (already pinned by the
//! `match_workers` fuzz target and the crate's proptest suite), not a
//! property of an individual parser. Ported from person-matcher's
//! identical, already-resolved T-35.

#![no_main]

use libfuzzer_sys::fuzz_target;
use worker_matcher::identifiers;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    // 42 personal-identifier parsers (one per scheme; never cross-matched).
    let _ = identifiers::parse_uk_nhs_number(s);
    let _ = identifiers::parse_fr_nir(s);
    let _ = identifiers::parse_es_tsi(s);
    let _ = identifiers::parse_ie_ihi(s);
    let _ = identifiers::parse_uk_hc_number(s);
    let _ = identifiers::parse_us_ssn(s);
    let _ = identifiers::parse_de_kvnr(s);
    let _ = identifiers::parse_it_cf(s);
    let _ = identifiers::parse_nl_bsn(s);
    let _ = identifiers::parse_se_personnummer(s);
    let _ = identifiers::parse_au_ihi(s);
    let _ = identifiers::parse_uk_chi_number(s);
    let _ = identifiers::parse_be_nn(s);
    let _ = identifiers::parse_bg_egn(s);
    let _ = identifiers::parse_cz_rc(s);
    let _ = identifiers::parse_dk_cpr(s);
    let _ = identifiers::parse_ee_ik(s);
    let _ = identifiers::parse_es_dni(s);
    let _ = identifiers::parse_fi_hetu(s);
    let _ = identifiers::parse_hr_oib(s);
    let _ = identifiers::parse_is_kt(s);
    let _ = identifiers::parse_lt_ak(s);
    let _ = identifiers::parse_lv_pk(s);
    let _ = identifiers::parse_mt_id(s);
    let _ = identifiers::parse_no_fnr(s);
    let _ = identifiers::parse_pl_pesel(s);
    let _ = identifiers::parse_ro_cnp(s);
    let _ = identifiers::parse_si_emso(s);
    let _ = identifiers::parse_sk_rc(s);
    let _ = identifiers::parse_uk_nino(s);
    let _ = identifiers::parse_gr_dss(s);
    let _ = identifiers::parse_li_id(s);
    let _ = identifiers::parse_nl_id(s);
    let _ = identifiers::parse_pl_nip(s);
    let _ = identifiers::parse_pt_nif(s);
    let _ = identifiers::parse_br_cpf(s);
    let _ = identifiers::parse_cn_rrn(s);
    let _ = identifiers::parse_in_aadhaar(s);
    let _ = identifiers::parse_jp_my_number(s);
    let _ = identifiers::parse_mx_curp(s);
    let _ = identifiers::parse_nz_nhi(s);
    let _ = identifiers::parse_za_id(s);

    // 9 per-country passport-format validators (feed `PassportBook`).
    let _ = identifiers::parse_cy_passport(s);
    let _ = identifiers::parse_cz_passport(s);
    let _ = identifiers::parse_li_passport(s);
    let _ = identifiers::parse_lt_passport(s);
    let _ = identifiers::parse_mt_passport(s);
    let _ = identifiers::parse_nl_passport(s);
    let _ = identifiers::parse_pt_passport(s);
    let _ = identifiers::parse_ro_passport(s);
    let _ = identifiers::parse_sk_passport(s);
});
