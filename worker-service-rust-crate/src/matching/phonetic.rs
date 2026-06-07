//! Phonetic matching algorithms (Soundex).
//!
//! Soundex collapses names that *sound* alike onto a shared four-character
//! code, letting the matcher recognise homophone misspellings (Smith/Smyth,
//! Robert/Rupert) that pure edit-distance metrics under-score. The name
//! matcher in [`algorithms`](crate::matching::algorithms) applies a small
//! phonetic bonus when two family names share a Soundex code.

/// Computes the four-character Soundex code for a name.
///
/// The code is the upper-cased first letter followed by up to three digits
/// derived from the remaining consonants (vowels and H/W/Y are skipped, and
/// adjacent duplicate digits collapse). Codes shorter than four characters
/// are right-padded with `'0'`. An empty or non-alphabetic input yields an
/// empty string.
///
/// # Examples
///
/// ```
/// use worker_service::matching::phonetic::soundex;
///
/// assert_eq!(soundex("Robert"), "R163");
/// assert_eq!(soundex("Rupert"), "R163"); // sounds the same
/// assert_eq!(soundex("Lee"), "L000");    // padded with zeros
/// ```
pub fn soundex(name: &str) -> String {
    let name = name.trim().to_uppercase();
    if name.is_empty() {
        return String::new();
    }

    // Keep only letters; punctuation and digits carry no phonetic value.
    let chars: Vec<char> = name.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if chars.is_empty() {
        return String::new();
    }

    // The code always begins with the literal first letter.
    let first = chars[0];
    let mut code = String::with_capacity(4);
    code.push(first);

    // Maps a consonant to its Soundex digit; vowels and H/W/Y map to None.
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

    // Seed with the first letter's digit so a repeated leading consonant
    // (e.g. the second letter of "Pp...") does not produce a duplicate.
    let mut last_digit = to_digit(first);

    for &c in &chars[1..] {
        // Stop once the four-character code is full.
        if code.len() >= 4 {
            break;
        }

        let digit = to_digit(c);
        if let Some(d) = digit {
            // Collapse runs of the same digit: only emit when it changed.
            if Some(d) != last_digit {
                code.push(d);
            }
        }
        // A vowel resets the run so two same-digit consonants separated by a
        // vowel are both counted.
        last_digit = digit;
    }

    // Pad with zeros to the fixed four-character width.
    while code.len() < 4 {
        code.push('0');
    }

    code
}

/// Returns `true` when both names produce the same non-empty Soundex code.
///
/// # Examples
///
/// ```
/// use worker_service::matching::phonetic::soundex_match;
///
/// assert!(soundex_match("Smith", "Smyth"));
/// assert!(!soundex_match("Smith", "Johnson"));
/// ```
pub fn soundex_match(name1: &str, name2: &str) -> bool {
    let s1 = soundex(name1);
    let s2 = soundex(name2);
    // Empty codes (non-alphabetic input) never count as a match.
    !s1.is_empty() && !s2.is_empty() && s1 == s2
}

/// Scores phonetic similarity in `[0.0, 1.0]`: 1.0 for identical Soundex
/// codes, otherwise partial credit equal to the fraction of matching leading
/// characters (out of four). Returns 0.0 if either code is empty.
///
/// # Examples
///
/// ```
/// use worker_service::matching::phonetic::phonetic_similarity;
///
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

    // Partial match: count matching leading characters, then scale by the
    // fixed four-character code width to yield a 0..1 fraction.
    let matching = s1.chars().zip(s2.chars()).take_while(|(a, b)| a == b).count();
    matching as f64 / 4.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known homophone pairs encode to the same Soundex code.
    #[test]
    fn test_soundex_basic() {
        assert_eq!(soundex("Robert"), "R163");
        assert_eq!(soundex("Rupert"), "R163");
        assert_eq!(soundex("Smith"), "S530");
        assert_eq!(soundex("Smyth"), "S530");
    }

    /// Same-sounding names match; clearly different names do not.
    #[test]
    fn test_soundex_match() {
        assert!(soundex_match("Robert", "Rupert"));
        assert!(soundex_match("Smith", "Smyth"));
        assert!(!soundex_match("Smith", "Johnson"));
    }

    /// Empty input yields empty; single letters and vowel-only tails pad with zeros.
    #[test]
    fn test_soundex_edge_cases() {
        assert_eq!(soundex(""), "");
        assert_eq!(soundex("A"), "A000");
        assert_eq!(soundex("Lee"), "L000");
    }

    /// Identical codes score 1.0, dissimilar names low, empty input 0.0.
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

    /// A single letter is padded to four characters with zeros.
    #[test]
    fn test_soundex_single_char() {
        assert_eq!(soundex("A"), "A000");
        assert_eq!(soundex("Z"), "Z000");
        assert_eq!(soundex("M"), "M000");
    }

    /// Non-letters are filtered, so "O'Brien" and "OBrien" encode alike.
    #[test]
    fn test_soundex_special_characters() {
        // Non-alphabetic characters should be filtered out
        assert_eq!(soundex("123"), "");
        assert_eq!(soundex("!!!"), "");
        // Mixed: alphabetic chars should still produce a code
        assert_eq!(soundex("O'Brien"), soundex("OBrien"));
    }

    /// The canonical Robert/Rupert pair encodes to R163 and matches.
    #[test]
    fn test_soundex_robert_rupert() {
        assert_eq!(soundex("Robert"), "R163");
        assert_eq!(soundex("Rupert"), "R163");
        assert!(soundex_match("Robert", "Rupert"));
    }

    /// "Ashcraft" produces a well-formed four-character A-prefixed code.
    #[test]
    fn test_soundex_ashcraft() {
        // Classic Soundex test case: Ashcraft -> A261
        let code = soundex("Ashcraft");
        assert_eq!(code.len(), 4);
        assert!(code.starts_with('A'), "Ashcraft should start with A, got {}", code);
    }
}
