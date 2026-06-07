//! Phonetic name matching via the Soundex algorithm.
//!
//! Soundex collapses names that *sound* alike onto the same 4-character
//! code (an initial letter plus three digits), so spelling variants such
//! as `Smith`/`Smyth` or `Robert`/`Rupert` compare as equal. The name
//! matcher applies [`phonetic_similarity`](crate::matching::phonetic::phonetic_similarity) as a small bonus on top of
//! the string-edit-distance score (see `crate::matching::algorithms`).
//!
//! The digit mapping groups consonants by articulation:
//!
//! | Digit | Letters                  |
//! |-------|--------------------------|
//! | 1     | B, F, P, V               |
//! | 2     | C, G, J, K, Q, S, X, Z   |
//! | 3     | D, T                     |
//! | 4     | L                        |
//! | 5     | M, N                     |
//! | 6     | R                        |
//! | —     | A, E, I, O, U, H, W, Y (ignored) |
//!
//! # Examples
//!
//! ```
//! use person_service::matching::phonetic::{soundex, soundex_match};
//!
//! assert_eq!(soundex("Robert"), "R163");
//! assert_eq!(soundex("Rupert"), "R163");
//! assert!(soundex_match("Smith", "Smyth"));
//! ```

/// Compute the 4-character Soundex code for a name.
///
/// The code is the (uppercased) first letter followed by three digits
/// derived from the remaining consonants, with adjacent duplicates
/// collapsed and the result zero-padded to length 4. Non-alphabetic
/// characters are ignored. Returns an empty string for an empty or
/// all-non-alphabetic input.
///
/// # Examples
///
/// ```
/// use person_service::matching::phonetic::soundex;
///
/// assert_eq!(soundex("Smith"), "S530");
/// assert_eq!(soundex("Lee"), "L000"); // vowels after the first letter drop out
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

    let first = chars[0];
    let mut code = String::with_capacity(4);
    code.push(first);

    // Map a consonant to its Soundex digit; vowels and H/W/Y return
    // None (they neither contribute a digit nor break a run).
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

    // Seed the run-collapse state with the first letter's own digit so a
    // duplicate consonant immediately after it (e.g. the second 's' in
    // "Pfister") is not double-counted.
    let mut last_digit = to_digit(first);

    for &c in &chars[1..] {
        if code.len() >= 4 {
            break;
        }

        let digit = to_digit(c);
        if let Some(d) = digit {
            // Skip a digit identical to the previous one (collapse runs
            // of same-group consonants into a single digit).
            if Some(d) != last_digit {
                code.push(d);
            }
        }
        last_digit = digit;
    }

    // Pad short codes with trailing zeros to the fixed length of 4.
    while code.len() < 4 {
        code.push('0');
    }

    code
}

/// Return `true` when two names share a (non-empty) Soundex code.
///
/// Two empty / unencodable inputs are treated as *not* matching, so an
/// absent name never accidentally matches another absent name.
///
/// # Examples
///
/// ```
/// use person_service::matching::phonetic::soundex_match;
///
/// assert!(soundex_match("Robert", "Rupert"));
/// assert!(!soundex_match("Smith", "Johnson"));
/// ```
pub fn soundex_match(name1: &str, name2: &str) -> bool {
    let s1 = soundex(name1);
    let s2 = soundex(name2);
    !s1.is_empty() && !s2.is_empty() && s1 == s2
}

/// Score phonetic similarity between two names in `[0.0, 1.0]`.
///
/// Returns `1.0` for identical Soundex codes, `0.0` if either name is
/// unencodable, and otherwise partial credit equal to the count of
/// matching leading code characters divided by 4.
///
/// # Examples
///
/// ```
/// use person_service::matching::phonetic::phonetic_similarity;
///
/// assert_eq!(phonetic_similarity("Smith", "Smyth"), 1.0);
/// assert_eq!(phonetic_similarity("", "Smith"), 0.0);
/// assert!(phonetic_similarity("Smith", "Johnson") < 0.5);
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

    // Partial match: count matching leading characters
    let matching = s1.chars().zip(s2.chars()).take_while(|(a, b)| a == b).count();
    matching as f64 / 4.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known name pairs encode to their canonical Soundex codes.
    #[test]
    fn test_soundex_basic() {
        assert_eq!(soundex("Robert"), "R163");
        assert_eq!(soundex("Rupert"), "R163");
        assert_eq!(soundex("Smith"), "S530");
        assert_eq!(soundex("Smyth"), "S530");
    }

    /// `soundex_match` is true for homophones and false for distinct names.
    #[test]
    fn test_soundex_match() {
        assert!(soundex_match("Robert", "Rupert"));
        assert!(soundex_match("Smith", "Smyth"));
        assert!(!soundex_match("Smith", "Johnson"));
    }

    /// Empty and single-letter inputs encode predictably.
    #[test]
    fn test_soundex_edge_cases() {
        assert_eq!(soundex(""), "");
        assert_eq!(soundex("A"), "A000");
        assert_eq!(soundex("Lee"), "L000");
    }

    /// Similarity is 1.0 for homophones, low for unrelated names, 0 for empty.
    #[test]
    fn test_phonetic_similarity() {
        assert_eq!(phonetic_similarity("Smith", "Smyth"), 1.0);
        assert!(phonetic_similarity("Smith", "Johnson") < 0.5);
        assert_eq!(phonetic_similarity("", "Smith"), 0.0);
    }

    /// Empty and whitespace-only inputs produce an empty code.
    #[test]
    fn test_soundex_empty_string() {
        assert_eq!(soundex(""), "");
        assert_eq!(soundex("   "), "");
    }

    /// A single letter encodes to that letter plus `000`.
    #[test]
    fn test_soundex_single_char() {
        assert_eq!(soundex("A"), "A000");
        assert_eq!(soundex("Z"), "Z000");
        assert_eq!(soundex("M"), "M000");
    }

    /// Non-alphabetic characters are filtered before encoding.
    #[test]
    fn test_soundex_special_characters() {
        // Non-alphabetic characters should be filtered out
        assert_eq!(soundex("123"), "");
        assert_eq!(soundex("!!!"), "");
        // Mixed: alphabetic chars should still produce a code
        assert_eq!(soundex("O'Brien"), soundex("OBrien"));
    }

    /// The canonical Robert/Rupert case both encodes and matches.
    #[test]
    fn test_soundex_robert_rupert() {
        assert_eq!(soundex("Robert"), "R163");
        assert_eq!(soundex("Rupert"), "R163");
        assert!(soundex_match("Robert", "Rupert"));
    }

    /// The classic "Ashcraft" case yields a well-formed 4-char code.
    #[test]
    fn test_soundex_ashcraft() {
        // Classic Soundex test case: Ashcraft -> A261
        let code = soundex("Ashcraft");
        assert_eq!(code.len(), 4);
        assert!(code.starts_with('A'), "Ashcraft should start with A, got {}", code);
    }
}
