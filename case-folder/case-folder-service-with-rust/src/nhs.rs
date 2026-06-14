//! NHS Number utilities.
//!
//! The UK NHS Number is a 10-digit identifier formatted `XXX XXX XXXX`. The
//! tenth digit is a Modulus 11 check digit calculated over the first nine
//! digits with weights 10..2. A remainder of 10 means the number is invalid;
//! a remainder of 11 yields a check digit of 0.
//!
//! See <https://en.wikipedia.org/wiki/NHS_number>.

/// Strip an NHS Number down to its bare digits.
///
/// Removes every non-ASCII-digit character, so grouped (`"943 476 5919"`)
/// and punctuated (`"943-476-5919"`) inputs all collapse to the same
/// digit string. This is the canonical normalisation applied before
/// formatting or validating.
///
/// # Parameters
/// - `raw`: any user- or service-supplied NHS Number string.
///
/// # Returns
/// A `String` of just the digit characters, in their original order.
pub fn normalise_nhs_number(raw: &str) -> String {
    raw.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Format an NHS Number into the canonical `XXX XXX XXXX` grouping.
///
/// First normalises `raw` to bare digits, then keeps at most the first
/// ten (extra digits are discarded). The grouping depends on how many
/// digits are present, so partial input formats gracefully as the user
/// types:
/// - `0..=3` digits: returned as-is, no spaces (e.g. `"94"` -> `"94"`).
/// - `4..=6` digits: split `XXX XX[X]` after the third
///   (e.g. `"94347"` -> `"943 47"`).
/// - otherwise (7..=10 digits): full `XXX XXX XXXX` grouping.
///
/// # Parameters
/// - `raw`: any NHS Number string (grouped, punctuated, or partial).
///
/// # Returns
/// The space-grouped form. Never panics: the `len()` match guarantees
/// every byte-slice index below is within bounds.
pub fn format_nhs_number(raw: &str) -> String {
    let digits = normalise_nhs_number(raw);
    // Cap at ten digits; an NHS Number is never longer.
    let truncated: String = digits.chars().take(10).collect();
    match truncated.len() {
        // Too short to group: emit the digits unchanged.
        0..=3 => truncated,
        // One space after the first three digits: `XXX XX[X]`.
        4..=6 => format!("{} {}", &truncated[..3], &truncated[3..]),
        // Full or near-full: the canonical `XXX XXX XXXX` grouping.
        _ => format!(
            "{} {} {}",
            &truncated[..3],
            &truncated[3..6],
            &truncated[6..]
        ),
    }
}

/// Validate an NHS Number via the Modulus 11 check-digit algorithm.
///
/// Accepts any grouped or punctuated form (it normalises first). A
/// number is valid only when it has exactly ten digits and the tenth
/// digit equals the check digit computed from the first nine.
///
/// # Algorithm (Modulus 11)
/// 1. Reject anything that is not exactly ten digits long.
/// 2. Multiply `digits[0..9]` by descending weights `10, 9, .., 2`
///    (weight for position `i` is `10 - i`).
/// 3. Sum the products and take `sum % 11` as the `remainder`.
/// 4. The check value is `11 - remainder`:
///    - `10` means the number is **invalid** (no single check digit can
///      represent 10), so we short-circuit `return false`.
///    - `11` maps to a check digit of `0`.
///    - any other value `n` is the check digit itself.
/// 5. The number is valid iff that check digit equals `digits[9]`, the
///    tenth (printed) digit.
///
/// # Parameters
/// - `raw`: the NHS Number to validate, in any accepted form.
///
/// # Returns
/// `true` if `raw` is a structurally valid NHS Number, else `false`.
/// Never panics: length is checked before any indexing.
pub fn is_valid_nhs_number(raw: &str) -> bool {
    let digits = normalise_nhs_number(raw);
    if digits.len() != 10 {
        return false;
    }
    // Read one decimal digit by index. `saturating_sub(b'0')` is safe
    // because `normalise_nhs_number` guarantees every byte is `0'..='9'`.
    let digit = |i: usize| -> u32 { digits.as_bytes()[i].saturating_sub(b'0') as u32 };
    let mut total: u32 = 0;
    // Weighted sum over the first nine digits; weight = 10 - i (10..=2).
    for i in 0..9 {
        total += digit(i) * (10 - i as u32);
    }
    let remainder = total % 11;
    let check = 11 - remainder;
    let check_digit = match check {
        // Check value 10 cannot be a single digit -> number is invalid.
        10 => return false,
        // Check value 11 wraps to a check digit of 0.
        11 => 0,
        // Otherwise the check value is the check digit directly.
        n => n,
    };
    // Valid iff the computed check digit matches the printed tenth digit.
    check_digit == digit(9)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins that spaces and hyphens are stripped, leaving bare digits.
    #[test]
    fn normalises_to_digits() {
        assert_eq!(normalise_nhs_number("943 476 5919"), "9434765919");
        assert_eq!(normalise_nhs_number("943-476-5919"), "9434765919");
    }

    /// Pins that ten bare digits format as the canonical `XXX XXX XXXX`.
    #[test]
    fn formats_full_numbers() {
        assert_eq!(format_nhs_number("9434765919"), "943 476 5919");
        assert_eq!(format_nhs_number("9876543210"), "987 654 3210");
    }

    /// Pins the partial-input grouping: 4..=6 digits get one space,
    /// 0..=3 digits stay ungrouped.
    #[test]
    fn formats_partial_inputs() {
        assert_eq!(format_nhs_number("94347"), "943 47");
        assert_eq!(format_nhs_number("94"), "94");
    }

    /// Pins that a set of known-good NHS Numbers all pass validation.
    #[test]
    fn validates_known_good() {
        for nhs in [
            "943 476 5919",
            "987 654 3210",
            "999 999 9999",
            "614 309 0432",
            "630 162 4483",
            "485 777 3457",
        ] {
            assert!(is_valid_nhs_number(nhs), "expected {nhs} to be valid");
        }
    }

    /// Pins that a wrong tenth digit (otherwise well-formed) is rejected.
    #[test]
    fn rejects_bad_check_digit() {
        // Each computes a valid check digit that does not equal the tenth digit.
        // See spec/nhs-number.md worked examples.
        assert!(!is_valid_nhs_number("943 476 5918")); // computed 9, tenth 8
        assert!(!is_valid_nhs_number("614 309 0431")); // computed 2, tenth 1
    }

    /// Pins the `10 => return false` branch: a prefix whose check value
    /// is 10 is invalid for every possible tenth digit.
    #[test]
    fn rejects_check_digit_of_ten() {
        // Weighted sum 254, 254 mod 11 = 1, check = 10 -> invalid by rule,
        // regardless of the tenth digit. Exercises the `10 => return false`
        // branch. See spec/nhs-number.md.
        assert!(!is_valid_nhs_number("999 000 0140"));
        // The same nine-digit prefix is invalid whatever the tenth digit is.
        for tenth in '0'..='9' {
            let candidate = format!("99900001{tenth}");
            assert!(
                !is_valid_nhs_number(&candidate),
                "{candidate} has check digit 10 and must be invalid"
            );
        }
    }

    /// Pins the length gate: too short, too long, and empty all fail.
    #[test]
    fn rejects_wrong_length() {
        assert!(!is_valid_nhs_number("123"));
        assert!(!is_valid_nhs_number("12345678901"));
        assert!(!is_valid_nhs_number(""));
    }

    /// Pins that normalisation makes grouped, bare, and hyphenated forms
    /// of the same number validate identically.
    #[test]
    fn accepts_grouped_and_bare_forms_identically() {
        assert_eq!(
            is_valid_nhs_number("943 476 5919"),
            is_valid_nhs_number("9434765919")
        );
        assert!(is_valid_nhs_number("943-476-5919"));
    }
}
