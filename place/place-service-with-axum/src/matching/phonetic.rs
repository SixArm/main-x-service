//! [Soundex](https://en.wikipedia.org/wiki/Soundex) phonetic coding for
//! sounds-alike name matching.
//!
//! Soundex reduces a word to a four-character code (a leading letter plus
//! three consonant digits) so that names that *sound* similar collapse to
//! the same code (`"Robert"` and `"Rupert"` both yield `R163`). The scorer
//! uses this as a small bonus — not a standalone component — to rescue
//! matches where a phonetic spelling variation drags the Jaro-Winkler score
//! just below the certain threshold.
//!
//! This is a deliberately simple Soundex variant: it treats vowels and
//! `H`/`W`/`Y` as separators rather than full "ignore" letters, so some
//! codes differ slightly from the classic algorithm (see
//! [`soundex`] for the `Ashcraft` caveat).
//!
//! # Examples
//!
//! ```
//! use place_service::matching::phonetic::{soundex, soundex_match};
//!
//! assert_eq!(soundex("Robert"), "R163");
//! assert!(soundex_match("Robert", "Rupert"));
//! ```

/// Compute the four-character Soundex code for a string.
///
/// The algorithm uppercases the input, keeps only ASCII letters, retains the
/// first letter, then encodes subsequent consonants to digits — dropping
/// vowels/`H`/`W`/`Y` and collapsing adjacent equal digits. The code is
/// padded with `0` to length four (and an all-non-letter input yields
/// `"0000"`).
///
/// # Examples
///
/// ```
/// use place_service::matching::phonetic::soundex;
///
/// assert_eq!(soundex(""), "0000");
/// assert_eq!(soundex("A"), "A000");
/// assert_eq!(soundex("Washington"), "W252");
/// ```
#[must_use]
pub fn soundex(s: &str) -> String {
    // Normalize: uppercase, then keep only ASCII letters.
    let s = s.trim().to_uppercase();
    let chars: Vec<char> = s.chars().filter(char::is_ascii_alphabetic).collect();

    // No letters at all ⇒ the conventional empty Soundex code.
    if chars.is_empty() {
        return "0000".to_string();
    }

    // The first letter is always kept verbatim as the code's prefix.
    let first = chars[0];
    let mut code = String::from(first);

    // Maps each consonant to its Soundex digit; everything else is '0'
    // (vowels and H/W/Y act as separators here).
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

    // Track the previous digit so adjacent equal digits collapse to one.
    let mut last_digit = to_digit(first);

    for &c in &chars[1..] {
        // A Soundex code never exceeds four characters.
        if code.len() >= 4 {
            break;
        }
        let digit = to_digit(c);
        // Skip separators ('0') and runs of the same digit.
        if digit != '0' && digit != last_digit {
            code.push(digit);
        }
        last_digit = digit;
    }

    // Right-pad short codes to the fixed length of four.
    while code.len() < 4 {
        code.push('0');
    }

    code
}

/// Returns whether two strings share the same Soundex code (i.e. sound alike).
///
/// # Examples
///
/// ```
/// use place_service::matching::phonetic::soundex_match;
///
/// assert!(soundex_match("Springfield", "Springfeild"));
/// assert!(!soundex_match("Robert", "Smith"));
/// ```
#[must_use]
pub fn soundex_match(a: &str, b: &str) -> bool {
    soundex(a) == soundex(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "Robert" encodes to the canonical R163.
    #[test]
    fn test_soundex_robert() {
        assert_eq!(soundex("Robert"), "R163");
    }

    /// "Rupert" encodes to the same R163 as "Robert".
    #[test]
    fn test_soundex_rupert() {
        assert_eq!(soundex("Rupert"), "R163");
    }

    /// Sounds-alike names report a phonetic match.
    #[test]
    fn test_soundex_match_similar_names() {
        assert!(soundex_match("Robert", "Rupert"));
    }

    /// Dissimilar-sounding names do not phonetically match.
    #[test]
    fn test_soundex_no_match() {
        assert!(!soundex_match("Robert", "Smith"));
    }

    /// Pins this implementation's (non-classic) "Ashcraft" code.
    #[test]
    fn test_soundex_ashcraft() {
        // A=first letter, s->2, h->0(skip), c->2(same as s, skip), r->6, a->0(skip), f->1, t->3
        // With adjacent-digit suppression: A261 per standard, but our simple
        // implementation treats h as a separator, giving A226.
        assert_eq!(soundex("Ashcraft"), "A226");
    }

    /// An empty string yields the all-zero code.
    #[test]
    fn test_soundex_empty() {
        assert_eq!(soundex(""), "0000");
    }

    /// A single letter pads to letter + "000".
    #[test]
    fn test_soundex_single_char() {
        assert_eq!(soundex("A"), "A000");
    }

    /// Encoding is case-insensitive.
    #[test]
    fn test_soundex_case_insensitive() {
        assert_eq!(soundex("smith"), soundex("SMITH"));
    }

    /// "Washington" encodes to W252 (W/H separators, collapsed digits).
    #[test]
    fn test_soundex_washington() {
        assert_eq!(soundex("Washington"), "W252");
    }

    /// A misspelled place name still phonetically matches the correct one.
    #[test]
    fn test_soundex_place_names() {
        assert!(soundex_match("Springfield", "Springfeild"));
    }
}
