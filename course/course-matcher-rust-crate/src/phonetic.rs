//! Soundex phonetic encoder — 4-character code (first letter +
//! 3 digits) used as a +0.05 bonus on the `name_score` when codes
//! match and the score hasn't already cleared the High band.
//!
//! The implementation is the classic American Soundex (Russell):
//! first letter retained, vowels + H/W/Y dropped (except as initial),
//! consonant runs collapsed, mapped by group, then truncated /
//! zero-padded to four characters. Examples:
//!
//! | input  | code |
//! |--------|------|
//! | smith  | S530 |
//! | smyth  | S530 |
//! | robert | R163 |
//! | rupert | R163 |

/// Encode `s` into a 4-character Soundex code.
///
/// Used by the name component as a phonetic fall-back: two course
/// titles that *sound* alike (e.g. `Smyth` / `Smith`) earn a small
/// bonus even when their literal spelling diverges. Why retain the
/// leading letter rather than encode it: that is the classic Russell
/// Soundex contract, and it keeps codes for distinct initials apart
/// (so `Catherine` / `Katheryn` deliberately do not collide).
///
/// # Returns
///
/// `Some(code)` where `code` is the 4-character Soundex string
/// (first letter + three digits, zero-padded). Returns `None` for
/// inputs with no ASCII-alphabetic characters — there is no leading
/// letter to anchor the code, so the pair cannot be compared
/// phonetically.
///
/// # Examples
///
/// ```
/// use course_matcher::phonetic::soundex;
///
/// assert_eq!(soundex("Smith").as_deref(), Some("S530"));
/// assert_eq!(soundex("123").as_deref(), None);
/// ```
#[must_use]
pub fn soundex(s: &str) -> Option<String> {
    // Keep only ASCII letters (digits / punctuation carry no phonetic
    // signal) and upper-case them so the mapping table is case-blind.
    let mut chars = s
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_uppercase());
    // The first letter is retained verbatim as the code's anchor; an
    // empty filtered iterator means no letters, so bail with `None`.
    let first = chars.next()?;
    let mut code = String::with_capacity(4);
    code.push(first);
    // Seed `prev` with the first letter's group so an immediately
    // following same-group consonant collapses against it.
    let mut prev = digit(first);
    for c in chars {
        let d = digit(c);
        if d == '0' {
            // Vowels and H/W/Y map to '0'. H and W are "transparent":
            // they keep `prev` unchanged so two same-group consonants
            // separated only by H/W still collapse (Ashcraft → A261).
            // Vowels reset `prev` so consonants either side do NOT
            // collapse (they are genuinely separated by a vowel).
            if c != 'H' && c != 'W' {
                prev = '0';
            }
            continue;
        }
        // Emit a digit only when it differs from the previous group —
        // this collapses runs of the same-group consonants into one.
        if d != prev {
            code.push(d);
            // The code is capped at four characters; stop early once full.
            if code.len() == 4 {
                return Some(code);
            }
        }
        prev = d;
    }
    // Short codes (few coding consonants) are right-padded with zeros
    // to the fixed 4-character width (Lee → L000).
    while code.len() < 4 {
        code.push('0');
    }
    Some(code)
}

/// Map a single upper-case letter to its Soundex group digit.
///
/// The six groups encode places of articulation so that
/// similar-sounding consonants share a digit. Vowels and the
/// "transparent" letters (A, E, I, O, U, H, W, Y) map to `'0'`, which
/// the caller treats specially (drop, but H/W do not reset the run).
///
/// `c` is assumed already upper-cased by the caller; any non-mapped
/// character (including lower-case, which never reaches here) falls
/// through to `'0'`.
fn digit(c: char) -> char {
    match c {
        'B' | 'F' | 'P' | 'V' => '1',
        'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => '2',
        'D' | 'T' => '3',
        'L' => '4',
        'M' | 'N' => '5',
        'R' => '6',
        _ => '0',
    }
}

/// True when both inputs produce the same Soundex code.
///
/// This is the predicate the name component actually consults: it is
/// the gate for the phonetic bonus. Returns `false` when *either*
/// side fails to encode (no ASCII letters) — an un-encodable string
/// has no phonetic identity to share, so it can never "sound the same".
///
/// # Examples
///
/// ```
/// use course_matcher::phonetic::same;
///
/// assert!(same("Smith", "Smyth"));
/// assert!(!same("Smith", "Jones"));
/// ```
#[must_use]
pub fn same(a: &str, b: &str) -> bool {
    match (soundex(a), soundex(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// Unit tests for the Soundex encoder and the `same` predicate.
#[cfg(test)]
mod tests {
    use super::*;

    // Pins the canonical Russell Soundex reference codes — these exact
    // values appear in every Soundex specification and guard against a
    // regression in the mapping table or run-collapse logic.
    #[test]
    fn classic_examples() {
        assert_eq!(soundex("Robert").as_deref(), Some("R163"));
        assert_eq!(soundex("Rupert").as_deref(), Some("R163"));
        assert_eq!(soundex("Rubin").as_deref(), Some("R150"));
        assert_eq!(soundex("Smith").as_deref(), Some("S530"));
        assert_eq!(soundex("Smyth").as_deref(), Some("S530"));
        assert_eq!(soundex("Ashcraft").as_deref(), Some("A261"));
        assert_eq!(soundex("Tymczak").as_deref(), Some("T522"));
    }

    // Pins that inputs with no ASCII letters encode to `None` rather
    // than an empty or padded string — the leading-letter anchor is
    // mandatory.
    #[test]
    fn empty_input_returns_none() {
        assert!(soundex("").is_none());
        assert!(soundex("123!").is_none());
    }

    // Pins the zero-padding rule: a name with no coding consonants
    // after the initial still produces a full 4-character code.
    #[test]
    fn pads_short_codes() {
        assert_eq!(soundex("Lee").as_deref(), Some("L000"));
    }

    #[test]
    fn same_helper_matches_phonetic_pairs() {
        // Same initial letter is part of the Soundex contract — codes
        // for `Catherine` / `Katheryn` differ on the leading letter
        // even though the rest of the encoding agrees.
        assert!(same("Smith", "Smyth"));
        assert!(same("Robert", "Rupert"));
        assert!(!same("Smith", "Jones"));
        assert!(!same("Catherine", "Katheryn"));
    }

    // Pins the fixed-width invariant: any alphabetic input — however
    // short or long — yields exactly four characters (truncated or padded).
    #[test]
    fn code_is_always_four_chars_for_alphabetic_input() {
        for s in ["a", "Lee", "Washington", "Supercalifragilistic"] {
            let code = soundex(s).expect("alphabetic input encodes");
            assert_eq!(code.len(), 4, "{s:?} → {code}");
        }
    }

    #[test]
    fn ignores_non_alphabetic_characters() {
        // Digits / punctuation are skipped; the leading letter is the
        // first ASCII-alphabetic char.
        assert_eq!(soundex("S-m-i-t-h").as_deref(), soundex("Smith").as_deref());
        assert_eq!(soundex("123Smith!").as_deref(), Some("S530"));
    }

    // Pins case-blindness: the upper-casing step makes encoding
    // independent of the input's letter case.
    #[test]
    fn case_insensitive() {
        assert_eq!(soundex("SMITH").as_deref(), soundex("smith").as_deref());
    }

    // Pins the `same` guard: a side that cannot encode (no letters)
    // makes the pair unequal regardless of the other side.
    #[test]
    fn same_is_false_when_either_side_has_no_letters() {
        assert!(!same("Smith", ""));
        assert!(!same("", "Smith"));
        assert!(!same("123", "456"));
    }

    #[test]
    fn adjacent_same_group_consonants_collapse() {
        // c, k, s all map to digit 2, so the "cks" run collapses to a
        // single 2: Jackson → J250.
        assert_eq!(soundex("Jackson").as_deref(), Some("J250"));
    }
}
