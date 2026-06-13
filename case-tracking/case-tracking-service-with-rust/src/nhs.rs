//! NHS Number utilities.
//!
//! The UK NHS Number is a 10-digit identifier formatted `XXX XXX XXXX`. The
//! tenth digit is a Modulus 11 check digit calculated over the first nine
//! digits with weights 10..2. A remainder of 10 means the number is invalid;
//! a remainder of 11 yields a check digit of 0.
//!
//! See <https://en.wikipedia.org/wiki/NHS_number>.

pub fn normalise_nhs_number(raw: &str) -> String {
    raw.chars().filter(|c| c.is_ascii_digit()).collect()
}

pub fn format_nhs_number(raw: &str) -> String {
    let digits = normalise_nhs_number(raw);
    let truncated: String = digits.chars().take(10).collect();
    match truncated.len() {
        0..=3 => truncated,
        4..=6 => format!("{} {}", &truncated[..3], &truncated[3..]),
        _ => format!(
            "{} {} {}",
            &truncated[..3],
            &truncated[3..6],
            &truncated[6..]
        ),
    }
}

pub fn is_valid_nhs_number(raw: &str) -> bool {
    let digits = normalise_nhs_number(raw);
    if digits.len() != 10 {
        return false;
    }
    let digit = |i: usize| -> u32 { digits.as_bytes()[i].saturating_sub(b'0') as u32 };
    let mut total: u32 = 0;
    for i in 0..9 {
        total += digit(i) * (10 - i as u32);
    }
    let remainder = total % 11;
    let check = 11 - remainder;
    let check_digit = match check {
        10 => return false,
        11 => 0,
        n => n,
    };
    check_digit == digit(9)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_to_digits() {
        assert_eq!(normalise_nhs_number("943 476 5919"), "9434765919");
        assert_eq!(normalise_nhs_number("943-476-5919"), "9434765919");
    }

    #[test]
    fn formats_full_numbers() {
        assert_eq!(format_nhs_number("9434765919"), "943 476 5919");
        assert_eq!(format_nhs_number("9876543210"), "987 654 3210");
    }

    #[test]
    fn formats_partial_inputs() {
        assert_eq!(format_nhs_number("94347"), "943 47");
        assert_eq!(format_nhs_number("94"), "94");
    }

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

    #[test]
    fn rejects_bad_check_digit() {
        assert!(!is_valid_nhs_number("943 476 5918"));
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(!is_valid_nhs_number("123"));
        assert!(!is_valid_nhs_number("12345678901"));
    }
}
