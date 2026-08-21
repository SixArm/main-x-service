# National personal identifiers

| National personal identifier endonym kebab case | National personal identifier exonym kebab case | Country name endonym | Country name exonym English | ISO 3166-1 code | Personal identifier name endonym | Personal identifier exonym English |
| --- | --- | --- | --- | --- | --- | --- |
| alba-community-health-index | scotland-community-health-index | Alba | Scotland | GB-SCT | Community Health Index (CHI) | Community Health Index (CHI) |
| australia-individual-healthcare-identifier | australia-individual-healthcare-identifier | Australia | Australia | AU | Individual Healthcare Identifier (IHI) | Individual Healthcare Identifier (IHI) |
| cymru-rhif-y-gwasanaeth-iechyd-gwladol | wales-national-health-service-number | Cymru | Wales | GB-CYM | Rhif y Gwasanaeth Iechyd Gwladol (Rhif GIG) | Rhif y Gwasanaeth Iechyd Gwladol (Rhif GIG) |
| deutschland-krankenversichertennummer | germany-health-insurance-number | Deutschland | Germany | DE | Krankenversichertennummer (KVNR) | Krankenversichertennummer (KVNR) |
| eire-aitheantoir-indibhidiuil-slainte | ireland-individual-health-identifier | Éire | Ireland | IE | Aitheantóir Indibhidiúil Sláinte (AIS) | Individual Health Identifier (IHI) |
| england-national-health-service-number | england-national-health-service-number | England | England | GB-ENG | National Health Service (NHS) Number | National Health Service (NHS) Number |
| espana-tarjeta-sanitaria-individual | spain-individual-health-card | España | Spain | ES | Tarjeta Sanitaria Individual (TSI) | Tarjeta Sanitaria Individual (TSI) |
| france-numero-didentification-au-repertoire | france-social-security-number | France | France | FR | Numéro d'Identification au Répertoire (NIR) | Numéro d'Identification au Répertoire (NIR) |
| italia-codice-fiscale | italy-fiscal-code | Italia | Italy | IT | Codice Fiscale (CF) | Codice Fiscale (CF) |
| nederland-burgerservicenummer | netherlands-citizen-service-number | Nederland | Netherlands | NL | Burgerservicenummer (BSN) | Burgerservicenummer (BSN) |
| northern-ireland-health-and-care-number | northern-ireland-health-and-care-number | Northern Ireland | Northern Ireland | GB-NIR | Health and Care (H&C) Number | Health and Care (H&C) Number |
| sverige-personnummer | sweden-personal-identity-number | Sverige | Sweden | SE | Personnummer | Personnummer |
| united-kingdom-national-health-service-number | united-kingdom-national-health-service-number | United Kingdom | United Kingdom | GB | National Health Service (NHS) Number | National Health Service (NHS) Number |
| united-states-social-security-number | united-states-social-security-number | United States | United States | US | Social Security Number (SSN) | Social Security Number (SSN) |

## 42-scheme parser reference

Each FR / parser / format summary is canonical for the wire-level behaviour. Per-parser unit tests in `src/identifiers.rs::tests` pin every behaviour-defining number (lengths, weights, check rules, sentinel rejections).

- **FR-12 UK United Kingdom National Health Service Number** — `parse_united_kingdom_national_health_service_number`, delegated to the `united-kingdom-national-health-service-number` crate (aliased upstream `nhs-number`); 10-digit canonical.
- **FR-25 FR NIR** — `parse_fr_nir`; 15 chars, Modulus-97 check key, Corsica department `"2A"` / `"2B"` remapping.
- **FR-26 ES TSI / CIP-SNS** — `parse_es_tsi`; length 10..=20, alphanumeric, format-only.
- **FR-27 IE IHI** — `parse_ie_ihi`; 7 digits after stripping non-digit characters.
- **FR-28 UK (NI) H&C Number** — `parse_uk_hc_number`; same algorithm as UK United Kingdom National Health Service Number; scheme-local.
- **FR-32 US SSN** — `parse_us_ssn`; 9 digits; reject area `000` / `666` / `900..=999`, group `00`, serial `0000`.
- **FR-39 AU IHI** — `parse_au_ihi`; 16 digits, Luhn (ISO/IEC 7812-1).
- **FR-40 DE KVNR** — `parse_de_kvnr`; 1 ASCII letter + 9 ASCII digits, Mod-10 via letter-ordinal expansion (`A=01..Z=26`).
- **FR-41 IT *Codice Fiscale*** — `parse_it_cf`; 16 ASCII alphanumerics, Mod-26 via odd/even position tables.
- **FR-42 NL BSN** — `parse_nl_bsn`; 9 digits, 11-test (`9·d₁ + 8·d₂ + … + 2·d₈ − d₉ ≡ 0 mod 11`); reject the all-zero string.
- **FR-43 SE *Personnummer*** — `parse_se_personnummer`; 10 or 12 digits, Luhn over the 10-digit form; input length preserved (10-digit and 12-digit forms do NOT cross-match).
- **FR-44 UK Scotland CHI Number** — `parse_uk_chi_number`; 10 digits, Mod-11 (same algorithm as United Kingdom National Health Service Number); computed check of 10 rejected; scheme-local.
- **FR-54 BE National Number** — `parse_be_nn`; 11 digits; check is `97 − (first-9 mod 97)`, with leading `"2"` prefix for births in 2000+; parser accepts either form.
- **FR-55 BG EGN** — `parse_bg_egn`; 10 digits; weights `[2,4,8,5,10,9,7,3,6]` mod 11; mod = 10 ⇒ check = 0.
- **FR-56 CZ *Rodné číslo*** — `parse_cz_rc`; 9 digits as-is, or 10 digits where the full number is divisible by 11.
- **FR-57 DK CPR** — `parse_dk_cpr`; 10 digits, format-only (the historical Mod-11 check was abandoned in 2007).
- **FR-58 EE *Isikukood*** — `parse_ee_ik`; 11 digits, cascading Mod-11 (pass-1 weights `[1..9, 1]`; pass-2 `[3..9, 1, 2, 3]`; mod = 10 in pass-2 ⇒ check = 0).
- **FR-59 ES DNI / NIE** — `parse_es_dni`; 8 digits + control letter from `"TRWAGMYFPDXBNJZSQVHLCKE"` indexed by `n mod 23`; NIE prefixes `X` / `Y` / `Z` map to leading digits `0` / `1` / `2`.
- **FR-60 FI HETU** — `parse_fi_hetu`; `DDMMYY` + century sign + 3 digits + Mod-31 check character from `"0123456789ABCDEFHJKLMNPRSTUVWXY"`.
- **FR-61 HR OIB** — `parse_hr_oib`; 11 digits, ISO 7064 MOD 11,10.
- **FR-62 IS *Kennitala*** — `parse_is_kt`; 10 digits, Mod-11 weights `[3,2,7,6,5,4,3,2]`; mod = 10 ⇒ invalid.
- **FR-63 LT *Asmens kodas*** — `parse_lt_ak`; 11 digits, same cascading Mod-11 as Estonia.
- **FR-64 LV *Personas kods*** — `parse_lv_pk`; 11 digits; weights `[1,6,3,7,9,10,5,8,4,2]`; `check = ((1101 − Σ) mod 11) mod 10`.
- **FR-65 MT National ID** — `parse_mt_id`; 7 digits + letter in `{M, G, A, P, L, H, B, Z}` (format-only — letter encodes geographic provenance).
- **FR-66 NO *Fødselsnummer*** — `parse_no_fnr`; 11 digits, two Mod-11 check digits; weights `[3,7,6,1,8,9,4,5,2]` and `[5,4,3,2,7,6,5,4,3,2]`; mod = 10 ⇒ invalid.
- **FR-67 PL PESEL** — `parse_pl_pesel`; 11 digits, weighted Mod-10 weights `[1,3,7,9,1,3,7,9,1,3]`.
- **FR-68 RO CNP** — `parse_ro_cnp`; 13 digits, Mod-11 weights `[2,7,9,1,4,6,3,5,8,2,7,9]`; mod = 10 ⇒ check = 1.
- **FR-69 SI EMŠO** — `parse_si_emso`; 13 digits, Mod-11 weights `[7,6,5,4,3,2,7,6,5,4,3,2]`; mod = 0 ⇒ check = 0, else `11 − mod`; check = 10 ⇒ invalid.
- **FR-70 SK *Rodné číslo*** — `parse_sk_rc`; same algorithm as Czech RČ.
- **FR-71 UK NINO** — `parse_uk_nino`; format `AA999999A`; banned 1st prefix letters `D F I Q U V`; banned 2nd prefix letters `D F I O Q U V`; banned admin prefixes `OO CR FY MW NC PP PZ TN`; suffix MUST be one of `A B C D`. Format-only.
- **FR-72 GR DSS** — `parse_gr_dss`; 10 digits, format-only.
- **FR-73 LI National ID** — `parse_li_id`; 2 letters + 8 or 9 digits; format-only; renewal-varying — for cross-renewal matching prefer `PassportBook` with `country = "LI"`.
- **FR-74 NL National ID** — `parse_nl_id`; 9 chars: positions 1–2 uppercase letters except `O`; 3–8 alphanumeric except `O`; 9 a digit.
- **FR-75 PL NIP** — `parse_pl_nip`; 10 digits, weights `[6,5,7,2,3,4,5,6,7]` mod 11; mod = 10 ⇒ invalid; else 10th digit MUST equal the remainder.
- **FR-76 PT NIF** — `parse_pt_nif`; 9 digits, weights `[9,8,7,6,5,4,3,2]` over the first 8; `r = Σ mod 11`; check = `0` if `r < 2` else `11 − r`.
- **FR-85 BR CPF** — `parse_br_cpf`; 11 digits; reject all-equal sequences (sentinel data); two Mod-11 weighted check digits at positions 9 and 10 using weights `[10,9,8,7,6,5,4,3,2]` and `[11,10,9,8,7,6,5,4,3,2]`; `check = 0` if `r < 2` else `11 − r`.
- **FR-86 CN RRN** — `parse_cn_rrn`; 18 chars: 17 digits + check character (digit or `X` / `x`); positions 6..14 MUST be a valid `YYYYMMDD` date; weights `W = [7,9,10,5,8,4,2,1,6,3,7,9,10,5,8,4,2]`; check from `['1','0','X','9','8','7','6','5','4','3','2']`. Pre-1999 15-digit form not accepted.
- **FR-87 IN Aadhaar** — `parse_in_aadhaar`; 12 digits; reject all-equal sequences and UIDAI-reserved `0` / `1` prefixes; Verhoeff check digit.
- **FR-88 JP My Number** — `parse_jp_my_number`; 12 digits; weights `[6,5,4,3,2,7,6,5,4,3,2]` over the first 11; `r = Σ mod 11`; check = `0` if `r < 2` else `11 − r`.
- **FR-89 MX CURP** — `parse_mx_curp`; 18 uppercased chars; structural shape `LLLL DDDDDD S LL LLL X D`; positions 4..10 MUST be a valid `YYMMDD` (century: `YY ≤ 29 → 20YY` else `19YY`); Mod-10 weighted check with `A..N = 10..23, Ñ = 24, O..Z = 25..36`.
- **FR-90 NZ NHI** — `parse_nz_nhi`; original 7-char form: 3 letters (`A..Z` except `I` / `O`) + 4 digits; letter values `A=1..H=8, J=9..N=13, P=14..Z=24`; weights `[7,6,5,4,3,2]`; mod = 0 ⇒ check = 0, mod = 1 ⇒ invalid, else check = `11 − mod`. 2019 alphanumeric form not supported by the parser.
- **FR-91 ZA ID Number** — `parse_za_id`; 13 digits; positions 0..6 MUST be a valid `YYMMDD` (century: `YY ≤ 29 → 20YY` else `19YY`); Luhn over all 13 digits.

## Passport-number format validators (FR-77)

Pure format validators in the `identifiers` module — no `Person` field; passport data flows through `Person::passport_books`:

- `parse_cy_passport` — `E` + 6 digits, or `K` + 8 digits.
- `parse_cz_passport` — 8 to 12 digits.
- `parse_li_passport` — 1 letter + 5 digits.
- `parse_lt_passport` — 8 digits.
- `parse_mt_passport` — 7 digits.
- `parse_nl_passport` — same shape as the NL ID card.
- `parse_pt_passport` — 1 letter + 6 digits.
- `parse_ro_passport` — 2 letters + 6 digits.
- `parse_sk_passport` — 2 letters + 7 digits.
