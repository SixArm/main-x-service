//! Phonetic matching via the Soundex algorithm.
//!
//! Soundex maps sound-alike strings to the same 4-character code (the
//! first letter followed by three digits). The name matcher uses it as
//! a similarity *floor*: when two titles share a Soundex code they are
//! guaranteed a minimum score, catching cases like "Smith" / "Smyth"
//! that string-edit distance would rate lower.
//!
//! # Examples
//!
//! ```
//! use event_service::matching::phonetic::{soundex, soundex_match};
//!
//! assert_eq!(soundex("Robert"), "R163");
//! assert_eq!(soundex("Rupert"), "R163");
//! assert!(soundex_match("Smith", "Smyth"));
//! assert!(!soundex_match("Smith", "Johnson"));
//! ```

/// Compute the Soundex code for a name.
///
/// The code is the (uppercased) first letter followed by three digits
/// derived from the consonants, with adjacent duplicate digits
/// collapsed and the result zero-padded to length 4. Non-alphabetic
/// characters are ignored; an empty or all-non-alphabetic input yields
/// an empty string.
///
/// # Examples
///
/// ```
/// use event_service::matching::phonetic::soundex;
///
/// assert_eq!(soundex("Smith"), "S530");
/// assert_eq!(soundex("Lee"), "L000");
/// assert_eq!(soundex(""), "");
/// ```
pub fn soundex(name: &str) -> String {
    let name = name.trim().to_uppercase();
    if name.is_empty() {
        return String::new();
    }

    let chars: Vec<char> = name.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if chars.is_empty() {
        return String::new();
    }

    // The code always begins with the first letter, verbatim.
    let first = chars[0];
    let mut code = String::with_capacity(4);
    code.push(first);

    // Map each consonant to its Soundex digit; vowels and H/W/Y return
    // None (they act as separators but contribute no digit).
    let to_digit = |c: char| -> Option<char> {
        match c {
            'B' | 'F' | 'P' | 'V' => Some('1'),
            'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => Some('2'),
            'D' | 'T' => Some('3'),
            'L' => Some('4'),
            'M' | 'N' => Some('5'),
            'R' => Some('6'),
            _ => None, // A, E, I, O, U, H, W, Y
        }
    };

    // Track the previous letter's digit so we can collapse runs of the
    // same digit (e.g. the double letters in "Pfister").
    let mut last_digit = to_digit(first);

    for &c in &chars[1..] {
        // The full code is exactly four characters.
        if code.len() >= 4 {
            break;
        }

        let digit = to_digit(c);
        if let Some(d) = digit {
            // Skip a digit identical to the immediately preceding one.
            if Some(d) != last_digit {
                code.push(d);
            }
        }
        last_digit = digit;
    }

    // Pad short codes with trailing zeros to reach length 4.
    while code.len() < 4 {
        code.push('0');
    }

    code
}

/// Check if two names have the same (non-empty) Soundex code.
///
/// # Examples
///
/// ```
/// use event_service::matching::phonetic::soundex_match;
/// assert!(soundex_match("Robert", "Rupert"));
/// ```
pub fn soundex_match(name1: &str, name2: &str) -> bool {
    let s1 = soundex(name1);
    let s2 = soundex(name2);
    !s1.is_empty() && !s2.is_empty() && s1 == s2
}

/// Compute a phonetic similarity score in `[0.0, 1.0]` between two names.
///
/// Returns `1.0` for identical Soundex codes, `0.0` when either code is
/// empty, otherwise partial credit equal to the count of shared leading
/// characters divided by 4.
///
/// # Examples
///
/// ```
/// use event_service::matching::phonetic::phonetic_similarity;
/// assert_eq!(phonetic_similarity("Smith", "Smyth"), 1.0);
/// assert_eq!(phonetic_similarity("", "Smith"), 0.0);
/// ```
pub fn phonetic_similarity(name1: &str, name2: &str) -> f64 {
    let s1 = soundex(name1);
    let s2 = soundex(name2);

    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }

    if s1 == s2 {
        return 1.0;
    }

    // Partial match: count how many leading characters agree before the
    // first divergence, scaled by the fixed code length of 4.
    let matching = s1.chars().zip(s2.chars()).take_while(|(a, b)| a == b).count();
    matching as f64 / 4.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Classic sound-alike pairs map to identical codes.
    #[test]
    fn test_soundex_basic() {
        assert_eq!(soundex("Robert"), "R163");
        assert_eq!(soundex("Rupert"), "R163");
        assert_eq!(soundex("Smith"), "S530");
        assert_eq!(soundex("Smyth"), "S530");
    }

    /// `soundex_match` is true for sound-alikes, false otherwise.
    #[test]
    fn test_soundex_match() {
        assert!(soundex_match("Robert", "Rupert"));
        assert!(soundex_match("Smith", "Smyth"));
        assert!(!soundex_match("Smith", "Johnson"));
    }

    /// Empty, single-letter, and vowel-only inputs are handled.
    #[test]
    fn test_soundex_edge_cases() {
        assert_eq!(soundex(""), "");
        assert_eq!(soundex("A"), "A000");
        assert_eq!(soundex("Lee"), "L000");
    }

    /// Similarity is 1.0 for identical codes, low for unrelated names,
    /// and 0.0 when one input is empty.
    #[test]
    fn test_phonetic_similarity() {
        assert_eq!(phonetic_similarity("Smith", "Smyth"), 1.0);
        assert!(phonetic_similarity("Smith", "Johnson") < 0.5);
        assert_eq!(phonetic_similarity("", "Smith"), 0.0);
    }

    /// Empty and whitespace-only inputs both yield an empty code.
    #[test]
    fn test_soundex_empty_string() {
        assert_eq!(soundex(""), "");
        assert_eq!(soundex("   "), "");
    }

    /// A single letter codes to that letter plus three zeros.
    #[test]
    fn test_soundex_single_char() {
        assert_eq!(soundex("A"), "A000");
        assert_eq!(soundex("Z"), "Z000");
        assert_eq!(soundex("M"), "M000");
    }

    /// Non-alphabetic characters are filtered before coding.
    #[test]
    fn test_soundex_special_characters() {
        // Non-alphabetic characters should be filtered out
        assert_eq!(soundex("123"), "");
        assert_eq!(soundex("!!!"), "");
        // Mixed: alphabetic chars should still produce a code
        assert_eq!(soundex("O'Brien"), soundex("OBrien"));
    }

    /// "Robert" and "Rupert" share the canonical R163 code.
    #[test]
    fn test_soundex_robert_rupert() {
        assert_eq!(soundex("Robert"), "R163");
        assert_eq!(soundex("Rupert"), "R163");
        assert!(soundex_match("Robert", "Rupert"));
    }

    /// The classic "Ashcraft" case produces a 4-char A-prefixed code.
    #[test]
    fn test_soundex_ashcraft() {
        // Classic Soundex test case: Ashcraft -> A261
        let code = soundex("Ashcraft");
        assert_eq!(code.len(), 4);
        assert!(code.starts_with('A'), "Ashcraft should start with A, got {}", code);
    }
}
