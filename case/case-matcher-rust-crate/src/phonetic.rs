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
///
/// Implements classic American (Russell) Soundex: retain the first
/// letter, map the rest to digit groups via [`digit`], drop zeros
/// (vowels + H/W/Y), collapse adjacent equal digits, then truncate or
/// zero-pad to exactly four characters.
///
/// `s` is the term to encode. Returns `Some(code)` for any input with at
/// least one ASCII letter, or `None` when there is no letter to seed the
/// leading character.
#[must_use]
pub fn soundex(s: &str) -> Option<String> {
    // Keep only letters, uppercased, so digits/punctuation/case do not
    // affect the encoding.
    let mut chars = s
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_uppercase());
    // The first letter is retained verbatim and seeds `prev`, so a digit
    // equal to the first letter's group is not re-emitted (the standard
    // "first letter" collapse rule).
    let first = chars.next()?;
    let mut code = String::with_capacity(4);
    code.push(first);
    let mut prev = digit(first);
    for c in chars {
        let d = digit(c);
        if d == '0' {
            // H and W keep `prev` so they don't collapse runs they sit between.
            // Vowels and Y (also group 0) reset `prev`, allowing an
            // otherwise-identical consonant on each side to be emitted twice.
            if c != 'H' && c != 'W' {
                prev = '0';
            }
            continue;
        }
        // Emit only when the group differs from the previous one, so runs
        // of same-group consonants collapse to a single digit.
        if d != prev {
            code.push(d);
            // Early exit once the 4-char code is full.
            if code.len() == 4 {
                return Some(code);
            }
        }
        prev = d;
    }
    // Short codes are right-padded with zeros to the fixed width of 4.
    while code.len() < 4 {
        code.push('0');
    }
    Some(code)
}

/// Map a single uppercase letter to its Soundex digit group.
///
/// The six consonant groups encode similar-sounding letters to the same
/// digit; vowels, `H`, `W`, and `Y` (and any non-mapped char) return
/// `'0'`, which the encoder treats as "drop". `c` is the uppercase letter
/// to classify. Returns the group digit `'0'..='6'` as a `char`.
fn digit(c: char) -> char {
    match c {
        'B' | 'F' | 'P' | 'V' => '1',                         // labials
        'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => '2', // gutturals/sibilants
        'D' | 'T' => '3',                                     // dentals
        'L' => '4',                                           // L
        'M' | 'N' => '5',                                     // nasals
        'R' => '6',                                           // R
        _ => '0',                                             // vowels, H, W, Y, other
    }
}

/// True when both inputs produce the same Soundex code.
///
/// The gate used by the title component's phonetic bonus. `a` and `b` are
/// the two strings to compare. Returns `true` only when both encode (each
/// has at least one letter) and their codes are identical; any side that
/// fails to encode yields `false`.
#[must_use]
pub fn same(a: &str, b: &str) -> bool {
    match (soundex(a), soundex(b)) {
        (Some(x), Some(y)) => x == y,
        // If either side has no letters, there is no phonetic match.
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pins the canonical textbook Soundex codes (Robert/Rupert → R163,
    // Smith/Smyth → S530, plus the H/W edge cases Ashcraft & Tymczak).
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

    // Pins that input with no letters (empty / digits+punct) → `None`.
    #[test]
    fn empty_input_returns_none() {
        assert!(soundex("").is_none());
        assert!(soundex("123!").is_none());
    }

    // Pins zero-padding: "Lee" yields only an initial, padded to L000.
    #[test]
    fn pads_short_codes() {
        assert_eq!(soundex("Lee").as_deref(), Some("L000"));
    }

    // Pins `same`: true for phonetic pairs, false for dissimilar names,
    // and false when the *initial letter* differs (Catherine/Katheryn).
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

    // Pins the fixed width invariant: any alphabetic input encodes to
    // exactly 4 chars (short padded, long truncated).
    #[test]
    fn code_is_always_four_chars_for_alphabetic_input() {
        for s in ["a", "Lee", "Washington", "Supercalifragilistic"] {
            let code = soundex(s).expect("alphabetic input encodes");
            assert_eq!(code.len(), 4, "{s:?} → {code}");
        }
    }

    // Pins that non-letters are skipped: "S-m-i-t-h" and "123Smith!"
    // both encode like "Smith".
    #[test]
    fn ignores_non_alphabetic_characters() {
        // Digits / punctuation are skipped; the leading letter is the
        // first ASCII-alphabetic char.
        assert_eq!(soundex("S-m-i-t-h").as_deref(), soundex("Smith").as_deref());
        assert_eq!(soundex("123Smith!").as_deref(), Some("S530"));
    }

    // Pins case-insensitivity: SMITH and smith encode identically.
    #[test]
    fn case_insensitive() {
        assert_eq!(soundex("SMITH").as_deref(), soundex("smith").as_deref());
    }

    // Pins `same` returns false when either side fails to encode.
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
