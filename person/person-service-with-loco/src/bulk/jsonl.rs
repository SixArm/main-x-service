//! JSONL codec — the lossless reference format
//! (`agents/share/bulk-import-export.md` §5).
//!
//! One [`Person`] per line, each line the person's API wire type (the
//! same Serde shape as `GET /api/persons/{id}`), so a JSONL round-trip is
//! lossless including nested identifiers / names / addresses / contacts /
//! documents. Reading is line-oriented and streaming: [`LineReader`] pulls
//! fixed-size chunks from an async source and yields one row at a time
//! (bounded memory regardless of file size — SEC-B2), and the caller
//! parses each with [`parse_line`], recording per-row parse errors (§7)
//! rather than aborting the whole file.
//!
//! [`split_lines`] remains as the whole-buffer splitter for **encode-side**
//! round-trip checks and tests; the import path does not use it.

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

/// The size of one read from the underlying source in [`LineReader`].
/// Small and fixed: this, plus the row being assembled, is the reader's
/// entire memory footprint regardless of how large the input is.
const READ_CHUNK_BYTES: usize = 64 * 1024;

/// A **streaming** JSONL line reader (SEC-B2): pulls fixed-size chunks
/// from an [`AsyncRead`] source and yields one complete line at a time,
/// carrying a partial line across a chunk boundary.
///
/// This is what replaced `split_lines_capped` — the whole-buffer splitter
/// that materialised every row of the file as a `Vec<String>` before the
/// first row was processed. Peak memory here is
/// [`READ_CHUNK_BYTES`] + the current row, so it is bounded by
/// [`MAX_IMPORT_ROW_BYTES`](crate::bulk::MAX_IMPORT_ROW_BYTES) and
/// **independent of the file size**.
///
/// Semantics are preserved from the splitter it replaced: blank (and
/// whitespace-only) lines are skipped and do not count toward the row cap;
/// invalid UTF-8 fails the job; more than `max_rows` non-blank rows fails
/// the job. Both failures are now observed **at the offending row** rather
/// than before the first one (see
/// [`MAX_IMPORT_ROWS`](crate::bulk::MAX_IMPORT_ROWS)).
pub struct LineReader<R> {
    /// The byte source.
    inner: R,
    /// The row currently being assembled (bytes seen since the last `\n`).
    carry: Vec<u8>,
    /// How far into `carry` the newline scan has already looked, so a long
    /// row is not rescanned from the start on every chunk (O(n) overall,
    /// not O(n²)).
    scanned: usize,
    /// The fixed-size read buffer.
    chunk: Vec<u8>,
    /// Non-blank rows yielded so far (the row-cap counter).
    rows: usize,
    /// The row cap; exceeding it fails the job.
    max_rows: usize,
    /// Whether the source has signalled end-of-input.
    eof: bool,
    /// Set once a terminal error or the end of input has been reported, so
    /// a caller that keeps polling gets `None` rather than a repeat error.
    done: bool,
}

impl<R> LineReader<R> {
    /// Build a reader over `inner`, failing the job past `max_rows`
    /// non-blank rows.
    pub fn new(inner: R, max_rows: usize) -> Self {
        Self {
            inner,
            carry: Vec::new(),
            scanned: 0,
            chunk: vec![0u8; READ_CHUNK_BYTES],
            rows: 0,
            max_rows,
            eof: false,
            done: false,
        }
    }

    /// Non-blank rows yielded so far.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Turn one framed row's bytes into the yielded line, applying the
    /// UTF-8 check and the row cap. `None` for a blank row (skipped).
    fn finish_row(&mut self, bytes: Vec<u8>) -> Option<Result<String>> {
        let text = match String::from_utf8(bytes) {
            Ok(t) => t,
            Err(e) => {
                self.done = true;
                return Some(Err(Error::Validation(format!(
                    "bulk input is not valid UTF-8: {e}"
                ))));
            }
        };
        if text.trim().is_empty() {
            return None;
        }
        self.rows += 1;
        if self.rows > self.max_rows {
            self.done = true;
            return Some(Err(Error::Validation(format!(
                "bulk import exceeds the row cap: more than {} rows",
                self.max_rows
            ))));
        }
        Some(Ok(text))
    }
}

impl<R: tokio::io::AsyncRead + Unpin> LineReader<R> {
    /// Yield the next non-blank line, or `None` at end of input.
    ///
    /// An `Err` item is **terminal and fatal to the job** (invalid UTF-8,
    /// an oversized row, the row cap, or a source read error); the reader
    /// yields `None` afterwards.
    pub async fn next_line(&mut self) -> Option<Result<String>> {
        use tokio::io::AsyncReadExt;

        loop {
            if self.done {
                return None;
            }

            // A complete row already in the carry buffer?
            if let Some(offset) = self.carry[self.scanned..].iter().position(|&b| b == b'\n') {
                let end = self.scanned + offset;
                let mut row: Vec<u8> = self.carry.drain(..=end).collect();
                row.pop(); // the '\n'
                if row.last() == Some(&b'\r') {
                    row.pop();
                }
                self.scanned = 0;
                if let Some(item) = self.finish_row(row) {
                    return Some(item);
                }
                continue; // blank line — skip, keep reading
            }
            self.scanned = self.carry.len();

            if self.eof {
                self.done = true;
                if self.carry.is_empty() {
                    return None;
                }
                // A final row with no trailing newline.
                let mut row = std::mem::take(&mut self.carry);
                if row.last() == Some(&b'\r') {
                    row.pop();
                }
                // `done` is already set, so a blank tail simply ends the
                // stream; `finish_row` may still override with an error.
                return self.finish_row(row);
            }

            // SEC-B2: bound the in-progress row so one enormous
            // unterminated line cannot grow the carry buffer to the whole
            // file — the guard the file-size cap used to provide.
            if self.carry.len() > crate::bulk::MAX_IMPORT_ROW_BYTES {
                self.done = true;
                return Some(Err(Error::Validation(format!(
                    "bulk import row exceeds the {}-byte row cap",
                    crate::bulk::MAX_IMPORT_ROW_BYTES
                ))));
            }

            match self.inner.read(&mut self.chunk).await {
                Ok(0) => self.eof = true,
                Ok(n) => self.carry.extend_from_slice(&self.chunk[..n]),
                Err(e) => {
                    self.done = true;
                    return Some(Err(Error::Internal(format!("read bulk import input: {e}"))));
                }
            }
        }
    }
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

    // ---- LineReader (the streaming replacement for split_lines_capped) --

    /// Drain a [`LineReader`] over `bytes` into `(lines, terminal error)`.
    /// `chunk_hint` is unused by the reader itself (it reads in fixed
    /// chunks); the source is a plain slice, which tokio reads in one go
    /// for small inputs and in pieces for large ones.
    async fn drain(bytes: &[u8], max_rows: usize) -> (Vec<String>, Option<String>) {
        let mut reader = super::LineReader::new(bytes, max_rows);
        let mut lines = Vec::new();
        let mut err = None;
        while let Some(item) = reader.next_line().await {
            match item {
                Ok(l) => lines.push(l),
                Err(e) => {
                    err = Some(e.to_string());
                    break;
                }
            }
        }
        (lines, err)
    }

    #[tokio::test]
    async fn line_reader_matches_split_lines_on_the_same_input() {
        // The streaming reader and the whole-buffer splitter must frame
        // rows identically — blank lines skipped, CRLF tolerated, a final
        // row without a trailing newline still yielded.
        for input in [
            &b"{}\n{}\n{}\n"[..],
            &b"\n  \n{}\n\n{}"[..],
            &b"{}\r\n{}\r\n"[..],
            &b"{}"[..],
            &b""[..],
        ] {
            let (streamed, err) = drain(input, 1000).await;
            assert_eq!(err, None, "no terminal error for {input:?}");
            assert_eq!(
                streamed,
                split_lines(input).unwrap(),
                "streaming framing differs for {input:?}"
            );
        }
    }

    #[tokio::test]
    async fn line_reader_frames_rows_across_chunk_boundaries() {
        // Rows far larger than one read is likely to deliver, so a row is
        // assembled from several reads; and enough of them to span many
        // chunks. Every row must come back whole and in order.
        let row = format!("{{\"n\":\"{}\"}}", "x".repeat(100_000));
        let mut input = Vec::new();
        for _ in 0..40 {
            input.extend_from_slice(row.as_bytes());
            input.push(b'\n');
        }
        let (lines, err) = drain(&input, 1000).await;
        assert_eq!(err, None);
        assert_eq!(lines.len(), 40);
        assert!(lines.iter().all(|l| *l == row), "every row round-trips");
    }

    #[tokio::test]
    async fn line_reader_rejects_past_the_row_cap() {
        // Three non-blank rows: a cap of 2 fails the job after yielding
        // the rows it was allowed to, a cap of 3 completes.
        let (lines, err) = drain(b"{}\n{}\n{}\n", 2).await;
        assert_eq!(lines.len(), 2, "the allowed rows are still yielded");
        assert!(
            err.as_deref().is_some_and(|e| e.contains("row cap")),
            "the cap failure is terminal: {err:?}"
        );
        let (lines, err) = drain(b"{}\n{}\n{}\n", 3).await;
        assert_eq!(lines.len(), 3);
        assert_eq!(err, None);
    }

    #[tokio::test]
    async fn line_reader_does_not_count_blank_rows_toward_the_cap() {
        let (lines, err) = drain(b"\n{}\n  \n{}\n\n", 2).await;
        assert_eq!(lines.len(), 2);
        assert_eq!(err, None);
    }

    #[tokio::test]
    async fn line_reader_rejects_an_oversized_row() {
        // One unterminated row past MAX_IMPORT_ROW_BYTES: the carry buffer
        // must refuse to grow rather than swallow the whole file.
        let giant = vec![b'a'; crate::bulk::MAX_IMPORT_ROW_BYTES + 1024];
        let (lines, err) = drain(&giant, 1000).await;
        assert!(lines.is_empty());
        assert!(
            err.as_deref().is_some_and(|e| e.contains("row cap")),
            "expected the per-row byte cap: {err:?}"
        );
    }

    #[tokio::test]
    async fn line_reader_rejects_invalid_utf8() {
        let mut bytes = b"{}\n".to_vec();
        bytes.push(0xF0); // a truncated 4-byte sequence, then EOF
        let (lines, err) = drain(&bytes, 1000).await;
        assert_eq!(lines.len(), 1, "the good row before it still came out");
        assert!(
            err.as_deref().is_some_and(|e| e.contains("UTF-8")),
            "expected a UTF-8 failure: {err:?}"
        );
    }

    #[tokio::test]
    async fn line_reader_is_terminal_after_an_error() {
        // Polling past a terminal error yields None, never a repeat.
        let mut reader = super::LineReader::new(&b"{}\n{}\n{}\n"[..], 1);
        assert!(reader.next_line().await.unwrap().is_ok());
        assert!(reader.next_line().await.unwrap().is_err());
        assert!(reader.next_line().await.is_none());
    }

    // SEC-B2 fuzz: the JSONL codec's parse boundary must never panic on
    // adversarial input — arbitrary strings, random bytes (incl. invalid /
    // truncated UTF-8), and a pathologically long single line. A malformed
    // row is an `Err`, never a crash. The streaming reader inherits the
    // property from the splitter it replaced (last property below).
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
        fn line_reader_never_panics_on_random_bytes(
            bytes in proptest::collection::vec(any::<u8>(), 0..4096),
            cap in 0usize..64,
        ) {
            // proptest bodies are sync; drive the async reader on a
            // current-thread runtime built per case.
            let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
            rt.block_on(async {
                let (_lines, _err) = drain(&bytes, cap).await;
            });
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
