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
}
