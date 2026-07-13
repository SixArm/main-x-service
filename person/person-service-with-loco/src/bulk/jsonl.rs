//! JSONL codec — the lossless reference format
//! (`agents/share/bulk-import-export.md` §5).
//!
//! One [`Person`] per line, each line the person's API wire type (the
//! same Serde shape as `GET /api/persons/{id}`), so a JSONL round-trip is
//! lossless including nested identifiers / names / addresses / contacts /
//! documents. Reading is line-oriented and streaming: the caller iterates
//! lines and parses each with [`parse_line`], recording per-row parse
//! errors (§7) rather than aborting the whole file.

use crate::models::Person;
use crate::{Error, Result};

/// Serialize one person to a single JSONL line (no trailing newline).
///
/// # Errors
///
/// Returns [`Error::Api`] if the person fails to serialize.
pub fn to_line(person: &Person) -> Result<String> {
    serde_json::to_string(person).map_err(|e| Error::Api(format!("serialize person to JSONL: {e}")))
}

/// Parse one JSONL line into a [`Person`].
///
/// The line must be a single JSON object matching the person wire type.
/// The raw `serde_json` error is preserved so callers can surface it in
/// the per-row error report.
///
/// # Errors
///
/// Returns the underlying [`serde_json::Error`] on malformed input.
pub fn parse_line(line: &str) -> serde_json::Result<Person> {
    serde_json::from_str(line)
}

/// Encode a slice of persons to a JSONL byte buffer (newline-terminated
/// lines), the export output shape.
///
/// # Errors
///
/// Returns [`Error::Api`] if any person fails to serialize.
pub fn encode(persons: &[Person]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for person in persons {
        out.extend_from_slice(to_line(person)?.as_bytes());
        out.push(b'\n');
    }
    Ok(out)
}

/// Split a JSONL byte buffer into its non-empty text lines, ignoring
/// blank lines. Returns an error if the bytes are not valid UTF-8.
///
/// # Errors
///
/// Returns [`Error::Validation`] if `input` is not valid UTF-8.
pub fn split_lines(input: &[u8]) -> Result<Vec<String>> {
    let text = std::str::from_utf8(input)
        .map_err(|e| Error::Validation(format!("bulk input is not valid UTF-8: {e}")))?;
    Ok(text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(ToString::to_string)
        .collect())
}

/// Like [`split_lines`], but reject the whole load when the non-blank row
/// count exceeds `max` (SEC-B2 row cap). Bounds the per-job work so a file
/// of millions of tiny lines cannot enqueue millions of per-row database
/// round-trips.
///
/// # Errors
///
/// Returns [`Error::Validation`] if `input` is not valid UTF-8 (via
/// [`split_lines`]) or if the row count exceeds `max`.
pub fn split_lines_capped(input: &[u8], max: usize) -> Result<Vec<String>> {
    let lines = split_lines(input)?;
    if lines.len() > max {
        return Err(Error::Validation(format!(
            "bulk import exceeds the row cap: {} rows > {max}",
            lines.len()
        )));
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::{encode, parse_line, split_lines, to_line};
    use crate::models::{Gender, HumanName, Identifier, IdentifierType, Person};

    fn sample(family: &str) -> Person {
        let mut p = Person::new(
            HumanName {
                use_type: None,
                family: family.to_string(),
                given: vec!["Ada".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Female,
        );
        p.identifiers.push(Identifier::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            "123-45-6789".to_string(),
        ));
        p
    }

    #[test]
    fn line_round_trips_losslessly() {
        let p = sample("Lovelace");
        let line = to_line(&p).unwrap();
        assert!(!line.contains('\n'));
        let back = parse_line(&line).unwrap();
        assert_eq!(back.id, p.id);
        assert_eq!(back.name.family, "Lovelace");
        assert_eq!(back.identifiers.len(), 1);
        assert_eq!(back.identifiers[0].value, "123-45-6789");
    }

    #[test]
    fn encode_then_split_yields_one_line_per_record() {
        let people = vec![sample("A"), sample("B")];
        let bytes = encode(&people).unwrap();
        let lines = split_lines(&bytes).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(parse_line(&lines[0]).unwrap().name.family, "A");
        assert_eq!(parse_line(&lines[1]).unwrap().name.family, "B");
    }

    #[test]
    fn split_lines_ignores_blank_lines() {
        let bytes = b"\n  \n{}\n\n";
        assert_eq!(split_lines(bytes).unwrap().len(), 1);
    }

    #[test]
    fn parse_line_rejects_garbage() {
        assert!(parse_line("not json").is_err());
    }

    #[test]
    fn split_lines_capped_rejects_over_the_cap() {
        // Three non-blank lines; a cap of 2 rejects, a cap of 3 accepts.
        let bytes = b"{}\n{}\n{}\n";
        assert!(super::split_lines_capped(bytes, 2).is_err());
        assert_eq!(super::split_lines_capped(bytes, 3).unwrap().len(), 3);
    }

    #[test]
    fn split_lines_capped_ignores_blanks_when_counting() {
        // Blank lines don't count toward the cap.
        let bytes = b"\n{}\n  \n{}\n\n";
        assert_eq!(super::split_lines_capped(bytes, 2).unwrap().len(), 2);
    }

    // SEC-B2 fuzz: the JSONL codec's parse boundary must never panic on
    // adversarial input — arbitrary strings, random bytes (incl. invalid /
    // truncated UTF-8), and a pathologically long single line. A malformed
    // row is an `Err`, never a crash.
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parse_line_never_panics(s in ".*") {
            let _ = parse_line(&s);
        }

        #[test]
        fn split_lines_never_panics_on_random_bytes(
            bytes in proptest::collection::vec(any::<u8>(), 0..4096)
        ) {
            let _ = split_lines(&bytes);
        }

        #[test]
        fn split_lines_capped_never_panics_on_random_bytes(
            bytes in proptest::collection::vec(any::<u8>(), 0..4096),
            cap in 0usize..64,
        ) {
            let _ = super::split_lines_capped(&bytes, cap);
        }
    }

    #[test]
    fn parse_line_handles_a_giant_line_without_panicking() {
        // A single 2 MiB line of one repeated byte: malformed JSON, but the
        // parser must return an error rather than blow the stack or panic.
        let giant = "a".repeat(2 * 1024 * 1024);
        assert!(parse_line(&giant).is_err());
        // And a giant *valid-UTF-8* buffer with no newline is one line.
        assert_eq!(split_lines(giant.as_bytes()).unwrap().len(), 1);
    }

    #[test]
    fn split_lines_handles_truncated_utf8_without_panicking() {
        // A leading valid line, then a truncated multi-byte sequence: the
        // whole buffer is invalid UTF-8, so split_lines returns Err.
        let mut bytes = b"{}\n".to_vec();
        bytes.push(0xF0); // start of a 4-byte sequence, then EOF
        assert!(split_lines(&bytes).is_err());
    }
}
