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

/// Encode `s` into a 4-character Soundex code (one initial letter plus
/// three digits). Non-alphabetic characters are ignored, so the first
/// retained letter is the first ASCII letter encountered.
///
/// Returns `None` for inputs with no ASCII-alphabetic characters (there
/// is then no leading letter to anchor the code).
#[must_use]
pub fn soundex(s: &str) -> Option<String> {
    // Keep only letters, upper-cased, so the algorithm is case- and
    // punctuation-insensitive.
    let mut chars = s
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_uppercase());
    // The first letter is preserved literally as the code's prefix.
    let first = chars.next()?;
    let mut code = String::with_capacity(4);
    code.push(first);
    // Seed `prev` with the first letter's digit so a second letter in
    // the same group as the first collapses against it.
    let mut prev = digit(first);
    for c in chars {
        let d = digit(c);
        if d == '0' {
            // Vowels and H/W/Y map to '0'. H and W keep `prev` so they
            // don't collapse runs they sit between (Ashcraft → A261);
            // vowels reset `prev` so a group can legitimately repeat
            // across a vowel (e.g. "Tymczak").
            if c != 'H' && c != 'W' {
                prev = '0';
            }
            continue;
        }
        // Emit a digit only when it differs from the previous one,
        // collapsing adjacent same-group consonants into one.
        if d != prev {
            code.push(d);
            // Classic Soundex is fixed-width: stop at four characters.
            if code.len() == 4 {
                return Some(code);
            }
        }
        prev = d;
    }
    // Right-pad short codes with zeros to the fixed width.
    while code.len() < 4 {
        code.push('0');
    }
    Some(code)
}

/// Map a single upper-case letter to its Soundex group digit. Letters
/// that carry no group (vowels and H/W/Y) map to `'0'`, which the
/// encoder treats as "skip".
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

/// True when both inputs produce the same Soundex code. `false` if
/// either input has no encodable letters — an absent code is never
/// treated as a phonetic match.
#[must_use]
pub fn same(a: &str, b: &str) -> bool {
    match (soundex(a), soundex(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// Unit tests for the Soundex encoder and its `same` helper.
#[cfg(test)]
mod tests {
    use super::*;

    /// Pins canonical reference codes (incl. the textbook Ashcraft /
    /// Tymczak edge cases that exercise the H/W and vowel rules).
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

    /// Pins that input with no letters has no anchor letter → `None`.
    #[test]
    fn empty_input_returns_none() {
        assert!(soundex("").is_none());
        assert!(soundex("123!").is_none());
    }

    /// Pins zero-padding: a code shorter than four chars is right-filled.
    #[test]
    fn pads_short_codes() {
        assert_eq!(soundex("Lee").as_deref(), Some("L000"));
    }

    /// Pins `same`: phonetically-equal names match, distinct ones don't,
    /// and a differing initial letter defeats the match (Soundex keeps
    /// the leading letter literally).
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

    /// Pins the fixed-width invariant: any alphabetic input encodes to
    /// exactly four characters (truncated or zero-padded as needed).
    #[test]
    fn code_is_always_four_chars_for_alphabetic_input() {
        for s in ["a", "Lee", "Washington", "Supercalifragilistic"] {
            let code = soundex(s).expect("alphabetic input encodes");
            assert_eq!(code.len(), 4, "{s:?} → {code}");
        }
    }

    /// Pins that digits/punctuation are skipped, so a leading number or
    /// embedded hyphens don't change the encoding.
    #[test]
    fn ignores_non_alphabetic_characters() {
        // Digits / punctuation are skipped; the leading letter is the
        // first ASCII-alphabetic char.
        assert_eq!(soundex("S-m-i-t-h").as_deref(), soundex("Smith").as_deref());
        assert_eq!(soundex("123Smith!").as_deref(), Some("S530"));
    }

    /// Pins case-insensitivity: upper and lower case encode identically.
    #[test]
    fn case_insensitive() {
        assert_eq!(soundex("SMITH").as_deref(), soundex("smith").as_deref());
    }

    /// Pins that `same` is `false` whenever either side fails to encode
    /// (no letters), rather than treating two `None`s as equal.
    #[test]
    fn same_is_false_when_either_side_has_no_letters() {
        assert!(!same("Smith", ""));
        assert!(!same("", "Smith"));
        assert!(!same("123", "456"));
    }

    /// Pins run-collapsing: adjacent consonants in the same group emit a
    /// single digit (Jackson's "cks" run → one 2).
    #[test]
    fn adjacent_same_group_consonants_collapse() {
        // c, k, s all map to digit 2, so the "cks" run collapses to a
        // single 2: Jackson → J250.
        assert_eq!(soundex("Jackson").as_deref(), Some("J250"));
    }
}
