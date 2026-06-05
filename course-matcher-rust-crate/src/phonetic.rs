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
pub fn soundex(s: &str) -> Option<String> {
    let mut chars = s
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase());
    let first = chars.next()?;
    let mut code = String::with_capacity(4);
    code.push(first);
    let mut prev = digit(first);
    for c in chars {
        let d = digit(c);
        if d == '0' {
            // H and W keep `prev` so they don't collapse runs they sit between.
            if c != 'H' && c != 'W' {
                prev = '0';
            }
            continue;
        }
        if d != prev {
            code.push(d);
            if code.len() == 4 {
                return Some(code);
            }
        }
        prev = d;
    }
    while code.len() < 4 {
        code.push('0');
    }
    Some(code)
}

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
pub fn same(a: &str, b: &str) -> bool {
    match (soundex(a), soundex(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn empty_input_returns_none() {
        assert!(soundex("").is_none());
        assert!(soundex("123!").is_none());
    }

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
}
