//! Phonetic name matching via [Soundex](https://en.wikipedia.org/wiki/Soundex).
//!
//! Soundex reduces a word to a 4-character code (a leading letter plus three
//! digits) that is stable across many spelling variants and typos, so that
//! "Springfield" and "Springfeild" share a code. The matcher uses this as a
//! small *bonus*, not a standalone component: when two names share a Soundex
//! code and the base score is below 0.95, [`compute_match`](crate::matching::scoring::compute_match)
//! adds +0.05.
//!
//! This is a simplified Soundex: it treats vowels and `H`/`W`/`Y` as
//! separators (resetting the run of repeated digits) rather than skipping
//! them transparently, so some codes differ from the strict American
//! Soundex algorithm (see the `Ashcraft` test). It is intentionally
//! consistent rather than canonical.
//!
//! # Examples
//!
//! ```
//! use thing_service::matching::phonetic::{soundex, soundex_match};
//!
//! assert_eq!(soundex("Robert"), "R163");
//! assert_eq!(soundex("Rupert"), "R163");
//! assert!(soundex_match("Robert", "Rupert"));
//! ```

/// Compute the Soundex code for a string.
///
/// Returns a 4-character code: the first alphabetic letter (uppercased)
/// followed by up to three digits encoding the remaining consonants.
/// Non-alphabetic input returns `"0000"`. The code is always exactly four
/// characters, right-padded with zeros.
///
/// # Examples
///
/// ```
/// use thing_service::matching::phonetic::soundex;
///
/// assert_eq!(soundex("Washington"), "W252");
/// assert_eq!(soundex(""), "0000");
/// assert_eq!(soundex("A"), "A000");
/// ```
pub fn soundex(s: &str) -> String {
    // Work in uppercase and keep only letters; digits/punctuation/spaces
    // are dropped entirely.
    let s = s.trim().to_uppercase();
    let chars: Vec<char> = s.chars().filter(|c| c.is_ascii_alphabetic()).collect();

    // No letters → the conventional all-zero code.
    if chars.is_empty() {
        return "0000".to_string();
    }

    // The first letter is carried verbatim and is the first character of the
    // code.
    let first = chars[0];
    let mut code = String::from(first);

    // Standard Soundex consonant-to-digit mapping; vowels and H/W/Y map to
    // '0', which acts as a separator below.
    let to_digit = |c: char| -> char {
        match c {
            'B' | 'F' | 'P' | 'V' => '1',
            'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => '2',
            'D' | 'T' => '3',
            'L' => '4',
            'M' | 'N' => '5',
            'R' => '6',
            _ => '0',
        }
    };

    // Track the previous digit so adjacent duplicates collapse to one.
    let mut last_digit = to_digit(first);

    for &c in &chars[1..] {
        // Stop once we have the leading letter plus three digits.
        if code.len() >= 4 {
            break;
        }
        let digit = to_digit(c);
        // Append only non-zero digits that differ from the previous one;
        // this drops vowels (digit '0') and runs of the same consonant class.
        if digit != '0' && digit != last_digit {
            code.push(digit);
        }
        // Note: last_digit updates even for '0', so a vowel between two equal
        // consonant digits acts as a separator that lets the second through.
        last_digit = digit;
    }

    // Right-pad short codes to the fixed 4-character width.
    while code.len() < 4 {
        code.push('0');
    }

    code
}

/// Check if two strings have the same Soundex code.
///
/// A convenience wrapper over [`soundex`]; `true` means the two strings are
/// phonetically equivalent under this implementation.
///
/// # Examples
///
/// ```
/// use thing_service::matching::phonetic::soundex_match;
///
/// assert!(soundex_match("Springfield", "Springfeild"));
/// assert!(!soundex_match("Robert", "Smith"));
/// ```
pub fn soundex_match(a: &str, b: &str) -> bool {
    soundex(a) == soundex(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Canonical "Robert" → R163.
    #[test]
    fn test_soundex_robert() {
        assert_eq!(soundex("Robert"), "R163");
    }

    /// "Rupert" collides with "Robert" (same code) — the point of Soundex.
    #[test]
    fn test_soundex_rupert() {
        assert_eq!(soundex("Rupert"), "R163");
    }

    /// `soundex_match` reports the Robert/Rupert collision.
    #[test]
    fn test_soundex_match_similar_names() {
        assert!(soundex_match("Robert", "Rupert"));
    }

    /// Unrelated names have distinct codes.
    #[test]
    fn test_soundex_no_match() {
        assert!(!soundex_match("Robert", "Smith"));
    }

    /// Pins this implementation's `H`-as-separator behaviour (A226, not A261).
    #[test]
    fn test_soundex_ashcraft() {
        // A=first letter, s->2, h->0(skip), c->2(same as s, skip), r->6, a->0(skip), f->1, t->3
        // With adjacent-digit suppression: A261 per standard, but our simple
        // implementation treats h as a separator, giving A226.
        assert_eq!(soundex("Ashcraft"), "A226");
    }

    /// Empty input yields the all-zero code.
    #[test]
    fn test_soundex_empty() {
        assert_eq!(soundex(""), "0000");
    }

    /// A single letter is padded with zeros.
    #[test]
    fn test_soundex_single_char() {
        assert_eq!(soundex("A"), "A000");
    }

    /// The code is case-insensitive.
    #[test]
    fn test_soundex_case_insensitive() {
        assert_eq!(soundex("smith"), soundex("SMITH"));
    }

    /// A longer name exercises the digit-collapsing logic.
    #[test]
    fn test_soundex_washington() {
        assert_eq!(soundex("Washington"), "W252");
    }

    /// Common typo pairs share a code (the property the bonus relies on).
    #[test]
    fn test_soundex_typo_pairs() {
        assert!(soundex_match("Springfield", "Springfeild"));
        assert!(soundex_match("Steven", "Stevn"));
    }
}
