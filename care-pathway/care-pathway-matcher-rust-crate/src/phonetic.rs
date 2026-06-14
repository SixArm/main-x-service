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

/// Encode `s` into a 4-character Soundex code. Returns `None` for
/// inputs with no ASCII-alphabetic characters.
#[must_use]
pub fn soundex(s: &str) -> Option<String> {
    let mut chars = s
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_uppercase());
    // The first letter is kept verbatim (Soundex's "S530" style), and its
    // own digit seeds `prev` so a same-group consonant immediately after
    // it collapses rather than being re-emitted.
    let first = chars.next()?;
    let mut code = String::with_capacity(4);
    code.push(first);
    let mut prev = digit(first);
    for c in chars {
        let d = digit(c);
        if d == '0' {
            // Vowels and H/W/Y map to '0'. H and W keep `prev` so they
            // don't collapse runs they sit between.
            if c != 'H' && c != 'W' {
                prev = '0';
            }
            continue;
        }
        // Only emit a digit when it differs from the previous one — this
        // collapses adjacent same-group consonants into a single digit.
        if d != prev {
            code.push(d);
            // Soundex is fixed-length 4: stop as soon as it's full.
            if code.len() == 4 {
                return Some(code);
            }
        }
        prev = d;
    }
    // Zero-pad short codes (e.g. "Lee" → "L000") to the fixed length.
    while code.len() < 4 {
        code.push('0');
    }
    Some(code)
}

/// Map a single uppercase letter to its Soundex group digit; vowels and
/// H/W/Y (and anything non-mapping) return `'0'`. Letters sharing a digit
/// are treated as phonetically equivalent and collapse in the encoder.
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
#[must_use]
pub fn same(a: &str, b: &str) -> bool {
    match (soundex(a), soundex(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Canonical Soundex vectors from the published algorithm, including
    // the well-known Ashcraft/Tymczak cases that exercise H/W handling.
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

    // No ASCII letters at all ⇒ `None` (there is no leading letter to keep).
    #[test]
    fn empty_input_returns_none() {
        assert!(soundex("").is_none());
        assert!(soundex("123!").is_none());
    }

    // Short inputs are zero-padded out to the fixed 4-char width.
    #[test]
    fn pads_short_codes() {
        assert_eq!(soundex("Lee").as_deref(), Some("L000"));
    }

    // `same` is true for phonetic pairs sharing a code, false otherwise,
    // and false when the leading letters differ (a Soundex contract: the
    // first letter is never encoded away).
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

    // Invariant: any input with at least one letter encodes to exactly
    // 4 characters, whether it needed padding or truncation.
    #[test]
    fn code_is_always_four_chars_for_alphabetic_input() {
        for s in ["a", "Lee", "Washington", "Supercalifragilistic"] {
            let code = soundex(s).expect("alphabetic input encodes");
            assert_eq!(code.len(), 4, "{s:?} → {code}");
        }
    }

    // Non-letters are filtered out entirely before encoding.
    #[test]
    fn ignores_non_alphabetic_characters() {
        // Digits / punctuation are skipped; the leading letter is the
        // first ASCII-alphabetic char.
        assert_eq!(soundex("S-m-i-t-h").as_deref(), soundex("Smith").as_deref());
        assert_eq!(soundex("123Smith!").as_deref(), Some("S530"));
    }

    // Encoding is case-insensitive (input is uppercased internally).
    #[test]
    fn case_insensitive() {
        assert_eq!(soundex("SMITH").as_deref(), soundex("smith").as_deref());
    }

    // `same` returns false if either side lacks letters — a `None` code
    // can never "equal" anything (guards against empty-vs-empty matching).
    #[test]
    fn same_is_false_when_either_side_has_no_letters() {
        assert!(!same("Smith", ""));
        assert!(!same("", "Smith"));
        assert!(!same("123", "456"));
    }

    // Pins the run-collapsing rule via a same-group consonant cluster.
    #[test]
    fn adjacent_same_group_consonants_collapse() {
        // c, k, s all map to digit 2, so the "cks" run collapses to a
        // single 2: Jackson → J250.
        assert_eq!(soundex("Jackson").as_deref(), Some("J250"));
    }
}
