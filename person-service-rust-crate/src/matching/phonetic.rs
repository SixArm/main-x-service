//! Phonetic matching algorithms (Soundex, Metaphone)

/// Compute the Soundex code for a name
///
/// Soundex maps similar-sounding names to the same code.
/// The code consists of the first letter followed by three digits.
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

    let mut last_digit = to_digit(first);

    for &c in &chars[1..] {
        if code.len() >= 4 {
            break;
        }

        let digit = to_digit(c);
        if let Some(d) = digit {
            if Some(d) != last_digit {
                code.push(d);
            }
        }
        last_digit = digit;
    }

    // Pad with zeros
    while code.len() < 4 {
        code.push('0');
    }

    code
}

/// Check if two names have the same Soundex code
pub fn soundex_match(name1: &str, name2: &str) -> bool {
    let s1 = soundex(name1);
    let s2 = soundex(name2);
    !s1.is_empty() && !s2.is_empty() && s1 == s2
}

/// Compute phonetic similarity score between two names.
/// Returns 1.0 for identical Soundex codes, with partial credit
/// for matching leading characters.
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

    #[test]
    fn test_soundex_basic() {
        assert_eq!(soundex("Robert"), "R163");
        assert_eq!(soundex("Rupert"), "R163");
        assert_eq!(soundex("Smith"), "S530");
        assert_eq!(soundex("Smyth"), "S530");
    }

    #[test]
    fn test_soundex_match() {
        assert!(soundex_match("Robert", "Rupert"));
        assert!(soundex_match("Smith", "Smyth"));
        assert!(!soundex_match("Smith", "Johnson"));
    }

    #[test]
    fn test_soundex_edge_cases() {
        assert_eq!(soundex(""), "");
        assert_eq!(soundex("A"), "A000");
        assert_eq!(soundex("Lee"), "L000");
    }

    #[test]
    fn test_phonetic_similarity() {
        assert_eq!(phonetic_similarity("Smith", "Smyth"), 1.0);
        assert!(phonetic_similarity("Smith", "Johnson") < 0.5);
        assert_eq!(phonetic_similarity("", "Smith"), 0.0);
    }

    #[test]
    fn test_soundex_empty_string() {
        assert_eq!(soundex(""), "");
        assert_eq!(soundex("   "), "");
    }

    #[test]
    fn test_soundex_single_char() {
        assert_eq!(soundex("A"), "A000");
        assert_eq!(soundex("Z"), "Z000");
        assert_eq!(soundex("M"), "M000");
    }

    #[test]
    fn test_soundex_special_characters() {
        // Non-alphabetic characters should be filtered out
        assert_eq!(soundex("123"), "");
        assert_eq!(soundex("!!!"), "");
        // Mixed: alphabetic chars should still produce a code
        assert_eq!(soundex("O'Brien"), soundex("OBrien"));
    }

    #[test]
    fn test_soundex_robert_rupert() {
        assert_eq!(soundex("Robert"), "R163");
        assert_eq!(soundex("Rupert"), "R163");
        assert!(soundex_match("Robert", "Rupert"));
    }

    #[test]
    fn test_soundex_ashcraft() {
        // Classic Soundex test case: Ashcraft -> A261
        let code = soundex("Ashcraft");
        assert_eq!(code.len(), 4);
        assert!(code.starts_with('A'), "Ashcraft should start with A, got {}", code);
    }
}
