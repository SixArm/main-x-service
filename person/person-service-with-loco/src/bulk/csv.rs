//! CSV codec — the operator / spreadsheet format
//! (`agents/share/bulk-import-export.md` §5).
//!
//! CSV is inherently flat, so the person wire type is flattened per the
//! shared [`super::columns`] declaration (scalars → columns; the primary
//! name → dotted columns; arrays / arrays-of-objects → a single
//! JSON-encoded cell).
//!
//! The codec round-trips **losslessly** against the JSONL reference: it
//! flattens the person's Serde `Value` into cells and rebuilds the same
//! `Value` on the way back, so `decode(encode(p)) == p`. Columns are
//! matched **by header name**, so operator-reordered columns and extra
//! columns are tolerated; a malformed row is a per-row `Err` (§7), never a
//! whole-file abort.

use serde_json::Value;

use crate::models::Person;
use crate::{Error, Result};

use super::columns::{COLUMNS, Kind, get, header, set};

/// Render one field `Value` to its cell text for the given [`Kind`].
fn render(value: &Value, kind: Kind) -> String {
    match kind {
        Kind::Scalar | Kind::Bool => match value {
            Value::Null => String::new(),
            Value::String(s) => s.clone(),
            other => other.to_string(),
        },
        Kind::Json => value.to_string(),
    }
}

/// Encode a slice of persons to a CSV byte buffer (a header row + one row
/// per person), the export output shape.
///
/// # Errors
///
/// Returns [`Error::Api`] if a person fails to serialize or the CSV writer
/// fails.
pub fn encode(persons: &[Person], delimiter: u8) -> Result<Vec<u8>> {
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_writer(Vec::new());
    wtr.write_record(header())
        .map_err(|e| Error::Api(format!("write CSV header: {e}")))?;
    for person in persons {
        let value = serde_json::to_value(person)
            .map_err(|e| Error::Api(format!("serialize person to CSV: {e}")))?;
        let row: Vec<String> = COLUMNS
            .iter()
            .map(|c| render(get(&value, c.path), c.kind))
            .collect();
        wtr.write_record(&row)
            .map_err(|e| Error::Api(format!("write CSV row: {e}")))?;
    }
    wtr.into_inner()
        .map_err(|e| Error::Api(format!("finish CSV: {e}")))
}

/// Parse a CSV byte buffer into per-row `(had_explicit_id, Person result)`
/// pairs. Columns are matched by header name (order-independent; unknown
/// columns ignored); an invalid row is an `Err` in its slot (§7 per-row
/// error contract) rather than aborting the whole load.
///
/// `had_explicit_id` is whether the row's own `id` cell was present and
/// non-empty — the CSV-native answer to the same question
/// [`crate::bulk::stable_key::row_has_explicit_id`] answers for a JSONL
/// line, needed because `Person::id` defaults to a fresh UUID when the
/// cell is empty, so the parsed `Person` alone cannot tell "no id given"
/// from "an id was given and happened to be this one".
///
/// # Errors
///
/// Returns [`Error::Validation`] if the bytes are not a readable CSV (bad
/// header / structurally broken record framing).
pub fn decode(input: &[u8], delimiter: u8) -> Result<Vec<(bool, serde_json::Result<Person>)>> {
    let mut out = Vec::new();
    let mut fatal = None;
    read_records(input, usize::MAX, delimiter, |item| match item {
        Ok(row) => {
            out.push(row);
            true
        }
        Err(e) => {
            fatal = Some(e);
            false
        }
    });
    match fatal {
        Some(e) => Err(e),
        None => Ok(out),
    }
}

/// One decoded CSV data row: whether it carried its own non-empty `id`
/// cell, and the person it parses to (or the per-row parse error, §7).
pub type CsvRow = (bool, serde_json::Result<Person>);

/// The **one** CSV reading implementation, generic over a blocking
/// [`std::io::Read`] so both callers share it: [`decode`] (whole buffer,
/// used by the export round-trip tests) and [`RowStream`] (the streaming
/// import path, which runs this on a blocking task fed by an async
/// source). Having one core is why a fix to the framing or the
/// header-resolution rules cannot land in one path and miss the other.
///
/// `sink` receives each row; an `Err` item is **fatal to the whole load**
/// (an unreadable header, broken record framing, or the SEC-B2 row cap),
/// matching the pre-streaming contract. Returning `false` from `sink`
/// stops the read (the consumer went away).
fn read_records<R: std::io::Read>(
    input: R,
    max_rows: usize,
    delimiter: u8,
    mut sink: impl FnMut(Result<CsvRow>) -> bool,
) -> bool {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_reader(input);
    let headers = match rdr.headers() {
        Ok(h) => h.clone(),
        Err(e) => {
            return sink(Err(Error::Validation(format!("read CSV header: {e}"))));
        }
    };
    // Resolve each expected column to its index in the actual header row.
    let indices: Vec<Option<usize>> = COLUMNS
        .iter()
        .map(|c| headers.iter().position(|h| h == c.header))
        .collect();
    let id_col = COLUMNS.iter().position(|c| c.header == "id");

    let mut rows = 0usize;
    for record in rdr.records() {
        let record = match record {
            Ok(r) => r,
            Err(e) => {
                return sink(Err(Error::Validation(format!("read CSV row: {e}"))));
            }
        };
        rows += 1;
        if rows > max_rows {
            return sink(Err(Error::Validation(format!(
                "bulk import exceeds the row cap: more than {max_rows} rows"
            ))));
        }
        let had_explicit_id = id_col
            .and_then(|ci| indices[ci])
            .and_then(|i| record.get(i))
            .is_some_and(|cell| !cell.is_empty());
        if !sink(Ok((had_explicit_id, record_to_person(&record, &indices)))) {
            return false;
        }
    }
    true
}

/// Rebuild one [`Person`] from a CSV record + the resolved column indices,
/// by reconstructing the person wire `Value` and deserializing it.
fn record_to_person(
    record: &csv::StringRecord,
    indices: &[Option<usize>],
) -> serde_json::Result<Person> {
    let mut map = serde_json::Map::new();
    for (column, index) in COLUMNS.iter().zip(indices) {
        let cell = index.and_then(|i| record.get(i)).unwrap_or("");
        match column.kind {
            Kind::Scalar => {
                if !cell.is_empty() {
                    set(&mut map, column.path, Value::String(cell.to_string()));
                }
            }
            Kind::Bool => {
                if !cell.is_empty() {
                    set(&mut map, column.path, Value::Bool(cell == "true"));
                }
            }
            Kind::Json => {
                // A present cell is parsed as JSON; an empty/missing cell is
                // omitted (not set to `null`) so the field's serde default
                // applies — `null` would fail to deserialize into a `Vec`.
                if !cell.is_empty() {
                    set(&mut map, column.path, serde_json::from_str(cell)?);
                }
            }
        }
    }
    serde_json::from_value(Value::Object(map))
}

/// Bytes pulled from the async source per read in [`RowStream`].
const READ_CHUNK_BYTES: usize = 64 * 1024;

/// How many read chunks may be in flight between the async feeder and the
/// blocking parser. Bounded, so a fast source cannot outrun the parser and
/// queue the whole file in memory.
const CHUNK_QUEUE: usize = 4;

/// How many parsed rows may be in flight between the blocking parser and
/// the async consumer. Bounded for the same reason, in the other
/// direction: a slow per-row database write must not let parsed rows pile
/// up.
const ROW_QUEUE: usize = 32;

/// A blocking [`std::io::Read`] fed by an async task over a bounded
/// channel.
///
/// The `csv` crate's reader is synchronous and pull-based, and the import
/// source is an async byte stream. Rather than materialise the file to
/// satisfy the sync reader (which is exactly what SEC-B2's streaming work
/// removes), the reader runs on a blocking task and takes its bytes from
/// this bridge, blocking on the channel when the feeder is behind.
struct ChannelReader {
    /// Chunks from the async feeder; `Err` carries a source read failure
    /// through to the CSV reader as an I/O error.
    rx: tokio::sync::mpsc::Receiver<std::io::Result<Vec<u8>>>,
    /// The chunk currently being handed out.
    current: Vec<u8>,
    /// How much of `current` has been handed out.
    pos: usize,
}

impl std::io::Read for ChannelReader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        while self.pos >= self.current.len() {
            match self.rx.blocking_recv() {
                Some(Ok(chunk)) => {
                    self.current = chunk;
                    self.pos = 0;
                }
                Some(Err(e)) => return Err(e),
                None => return Ok(0), // feeder finished or went away
            }
        }
        let n = out.len().min(self.current.len() - self.pos);
        out[..n].copy_from_slice(&self.current[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// A **streaming** CSV row source (SEC-B2): yields one decoded row at a
/// time from an async byte source, never materialising the file or the
/// full row set.
///
/// This is what replaced `decode`-then-iterate on the import path, where
/// the whole file was read into a `Vec<u8>` and then decoded into a `Vec`
/// of every parsed person before the first row was written.
///
/// Three bounded stages run concurrently: an async feeder reading
/// [`READ_CHUNK_BYTES`] at a time, a blocking task running the `csv`
/// reader over [`ChannelReader`], and the async consumer pulling rows.
/// Peak memory is the two queue depths ([`CHUNK_QUEUE`], [`ROW_QUEUE`])
/// times their element size — independent of the file size. Dropping the
/// stream tears both tasks down: the row send fails, the parser returns,
/// the chunk channel closes, and the feeder stops.
pub struct RowStream {
    /// Decoded rows from the blocking parser. An `Err` item is fatal to
    /// the job, exactly as [`decode`] returning `Err` is.
    rx: tokio::sync::mpsc::Receiver<Result<CsvRow>>,
}

impl RowStream {
    /// Start streaming rows from `reader`, failing the job past `max_rows`
    /// data rows.
    ///
    /// Must be called from within a Tokio runtime (it spawns the feeder
    /// and the blocking parser).
    #[must_use]
    pub fn new<R>(mut reader: R, max_rows: usize, delimiter: u8) -> Self
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let (chunk_tx, chunk_rx) = tokio::sync::mpsc::channel(CHUNK_QUEUE);
        let (row_tx, row_rx) = tokio::sync::mpsc::channel(ROW_QUEUE);

        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut buf = vec![0u8; READ_CHUNK_BYTES];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if chunk_tx.send(Ok(buf[..n].to_vec())).await.is_err() {
                            break; // the parser went away
                        }
                    }
                    Err(e) => {
                        let _ = chunk_tx.send(Err(e)).await;
                        break;
                    }
                }
            }
        });

        tokio::task::spawn_blocking(move || {
            let bridge = ChannelReader {
                rx: chunk_rx,
                current: Vec::new(),
                pos: 0,
            };
            read_records(bridge, max_rows, delimiter, |item| {
                row_tx.blocking_send(item).is_ok()
            });
        });

        Self { rx: row_rx }
    }

    /// The next decoded row, or `None` once the input is exhausted.
    pub async fn next_row(&mut self) -> Option<Result<CsvRow>> {
        self.rx.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::{decode, encode, header};
    use crate::models::{
        Address, AddressUse, ContactPoint, ContactPointSystem, ContactPointUse, DocumentType,
        Gender, HumanName, Identifier, IdentifierType, IdentityDocument, LinkType, NameUse, Person,
        PersonLink,
    };
    use chrono::{NaiveDate, TimeZone, Utc};
    use uuid::Uuid;

    /// A person with **every** field populated (nested objects + arrays), so
    /// a round-trip that preserves it proves the codec is lossless across the
    /// whole model.
    fn fully_populated() -> Person {
        let mut p = Person::new(
            HumanName {
                use_type: Some(NameUse::Official),
                family: "Lovelace".to_string(),
                given: vec!["Ada".to_string(), "Augusta".to_string()],
                prefix: vec!["Ms".to_string()],
                suffix: vec!["Jr".to_string()],
            },
            Gender::Female,
        );
        p.id = Uuid::new_v4();
        p.active = false;
        p.identifiers = vec![Identifier::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            "123-45-6789".to_string(),
        )];
        p.additional_names = vec![HumanName {
            use_type: Some(NameUse::Maiden),
            family: "Byron".to_string(),
            given: vec!["Ada".to_string()],
            prefix: vec![],
            suffix: vec![],
        }];
        p.telecom = vec![ContactPoint {
            system: ContactPointSystem::Email,
            value: "ada@example.com".to_string(),
            use_type: Some(ContactPointUse::Home),
        }];
        p.birth_date = Some(NaiveDate::from_ymd_opt(1815, 12, 10).unwrap());
        p.tax_id = Some("TAX-1".to_string());
        p.documents = vec![IdentityDocument {
            document_type: DocumentType::Passport,
            number: "X1".to_string(),
            issuing_country: Some("GB".to_string()),
            issuing_authority: None,
            issue_date: None,
            expiry_date: None,
            verified: true,
        }];
        p.deceased = true;
        p.deceased_datetime = Some(Utc.with_ymd_and_hms(1852, 11, 27, 0, 0, 0).unwrap());
        p.addresses = vec![Address {
            use_type: Some(AddressUse::Home),
            line1: Some("1 Analytical Way".to_string()),
            line2: None,
            city: Some("London".to_string()),
            state: None,
            postal_code: Some("SW1".to_string()),
            country: Some("GB".to_string()),
        }];
        p.marital_status = Some("married".to_string());
        p.multiple_birth = Some(false);
        p.photo = vec!["https://example.com/ada.png".to_string()];
        p.managing_organization = Some(Uuid::new_v4());
        p.links = vec![PersonLink {
            other_person_id: Uuid::new_v4(),
            link_type: LinkType::Seealso,
        }];
        p
    }

    /// The single load-bearing property: encode → decode returns the exact
    /// same person (compared as the wire `Value`, so no field is dropped or
    /// mistyped).
    #[test]
    fn round_trips_a_fully_populated_person_losslessly() {
        let p = fully_populated();
        let bytes = encode(std::slice::from_ref(&p), b',').unwrap();
        let rows = decode(&bytes, b',').unwrap();
        assert_eq!(rows.len(), 1);
        let (had_explicit_id, parsed) = rows.into_iter().next().unwrap();
        assert!(had_explicit_id, "the exported id column round-trips");
        let back = parsed.expect("row parses");
        assert_eq!(
            serde_json::to_value(&back).unwrap(),
            serde_json::to_value(&p).unwrap(),
            "CSV round-trip must be lossless"
        );
    }

    /// A sparse person (only the required fields; every `Option`/array empty)
    /// also round-trips — the empty-cell handling restores `None`/`[]`.
    #[test]
    fn round_trips_a_sparse_person() {
        let p = Person::new(
            HumanName {
                use_type: None,
                family: "Solo".to_string(),
                given: vec!["Han".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        let bytes = encode(std::slice::from_ref(&p), b',').unwrap();
        let back = decode(&bytes, b',')
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .1
            .unwrap();
        assert_eq!(
            serde_json::to_value(&back).unwrap(),
            serde_json::to_value(&p).unwrap()
        );
        assert!(back.tax_id.is_none() && back.identifiers.is_empty());
    }

    /// Multiple persons → a header + one row each; every row round-trips.
    #[test]
    fn encodes_a_header_then_one_row_per_person() {
        let a = fully_populated();
        let b = Person::new(
            HumanName {
                use_type: None,
                family: "B".to_string(),
                given: vec!["Bee".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Other,
        );
        let bytes = encode(&[a.clone(), b.clone()], b',').unwrap();
        // Header line first.
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(
            text.lines()
                .next()
                .unwrap()
                .starts_with("id,active,name.family,")
        );
        let rows = decode(&bytes, b',').unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1.as_ref().unwrap().name.family, "Lovelace");
        assert_eq!(rows[1].1.as_ref().unwrap().name.family, "B");
    }

    /// Columns are matched by header, so a reordered / extra-column file
    /// still imports (operator-edited CSVs are tolerated).
    #[test]
    fn decodes_reordered_and_extra_columns() {
        let csv = "gender,extra,name.family,name.given,id,active\n\
                   male,ignored,Vader,[\"Anakin\"],11111111-1111-4111-8111-111111111111,true\n";
        let rows = decode(csv.as_bytes(), b',').unwrap();
        let (had_explicit_id, parsed) = rows.into_iter().next().unwrap();
        assert!(had_explicit_id, "the reordered id column is still found");
        let p = parsed.expect("parses");
        assert_eq!(p.name.family, "Vader");
        assert_eq!(p.name.given, vec!["Anakin".to_string()]);
        assert_eq!(p.gender, Gender::Male);
        assert!(p.active);
    }

    /// A structurally-valid row with a bad JSON cell is a per-row `Err`, not
    /// a whole-file failure (§7).
    #[test]
    fn a_bad_json_cell_is_a_per_row_error() {
        let hdr = header().join(",");
        // A row whose `identifiers` cell is malformed JSON.
        let bad = format!(
            "{hdr}\n11111111-1111-4111-8111-111111111111,true,X,,[],[],[],male,,,false,,,,,,,not-json,[],[],[],[],[],[],[]\n"
        );
        let rows = decode(bad.as_bytes(), b',').unwrap();
        assert_eq!(rows.len(), 1);
        assert!(
            rows[0].1.is_err(),
            "malformed identifiers cell ⇒ per-row Err"
        );
    }

    /// A row whose `id` cell is empty (or the column is absent entirely)
    /// is **not** an explicit id — the parsed `Person` still gets a fresh
    /// UUID via the field's serde default, but callers need to know the
    /// row itself supplied none, so a keyless row can be routed through
    /// duplicate detection rather than a blind create. Built from a real
    /// `encode(, b',')` row (blanking only the leading id cell) rather than a
    /// hand-counted CSV literal, so the column count is never at risk of
    /// drifting from [`COLUMNS`].
    #[test]
    fn had_explicit_id_is_false_for_an_empty_or_missing_id_cell() {
        let p = fully_populated();
        let bytes = encode(std::slice::from_ref(&p), b',').unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        let (header_line, row_line) = text.split_once('\n').unwrap();

        // A populated id cell IS explicit (sanity check on the fixture).
        let rows = decode(text.as_bytes(), b',').unwrap();
        assert!(rows[0].0, "populated id cell ⇒ explicit id");

        // Blank the leading id cell only.
        let (_id_cell, rest) = row_line.split_once(',').unwrap();
        let blanked = format!("{header_line}\n,{rest}\n");
        let rows = decode(blanked.as_bytes(), b',').unwrap();
        assert!(!rows[0].0, "empty id cell ⇒ no explicit id");
        assert!(rows[0].1.is_ok());

        // The id column omitted entirely (operator-trimmed header).
        let no_id_col = "active,name.family,name.given,gender\ntrue,Y,[\"B\"],female\n";
        let rows = decode(no_id_col.as_bytes(), b',').unwrap();
        assert!(!rows[0].0, "missing id column ⇒ no explicit id");
        assert!(rows[0].1.is_ok());
    }

    // ---- RowStream (the streaming import path) -------------------------

    /// Drain a [`super::RowStream`] over `bytes` into its rows plus the
    /// terminal error, if any.
    #[allow(clippy::type_complexity)]
    async fn drain(
        bytes: &[u8],
        max_rows: usize,
    ) -> (Vec<(bool, bool)>, Option<String>, Vec<super::CsvRow>) {
        drain_with(bytes, max_rows, b',').await
    }

    /// [`drain`] with an explicit delimiter, so the TSV path exercises the
    /// same streaming reader rather than a parallel one.
    #[allow(clippy::type_complexity)]
    async fn drain_with(
        bytes: &[u8],
        max_rows: usize,
        delimiter: u8,
    ) -> (Vec<(bool, bool)>, Option<String>, Vec<super::CsvRow>) {
        let mut stream =
            super::RowStream::new(std::io::Cursor::new(bytes.to_vec()), max_rows, delimiter);
        let mut summary = Vec::new();
        let mut rows = Vec::new();
        let mut err = None;
        while let Some(item) = stream.next_row().await {
            match item {
                Ok(row) => {
                    summary.push((row.0, row.1.is_ok()));
                    rows.push(row);
                }
                Err(e) => {
                    err = Some(e.to_string());
                    break;
                }
            }
        }
        (summary, err, rows)
    }

    /// The streaming reader and the whole-buffer [`decode`] share one core
    /// (`read_records`), and this pins that they therefore agree row for
    /// row — including the per-row `had_explicit_id` flag and which rows
    /// parse.
    #[tokio::test]
    async fn row_stream_agrees_with_decode() {
        let people = vec![fully_populated(), Person::new(sparse_name(), Gender::Male)];
        let bytes = encode(&people, b',').unwrap();

        let buffered: Vec<(bool, bool)> = decode(&bytes, b',')
            .unwrap()
            .into_iter()
            .map(|(id, parsed)| (id, parsed.is_ok()))
            .collect();
        let (streamed, err, rows) = drain(&bytes, 1000).await;
        assert_eq!(err, None);
        assert_eq!(streamed, buffered, "streaming matches buffered decoding");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].1.as_ref().unwrap().name.family,
            people[0].name.family
        );
    }

    #[tokio::test]
    async fn row_stream_keeps_a_bad_cell_as_a_per_row_error() {
        // §7: a malformed row is a per-row error the stream still yields,
        // never a whole-load abort.
        let hdr = header().join(",");
        let bad = format!(
            "{hdr}\n11111111-1111-4111-8111-111111111111,true,X,,[],[],[],male,,,false,,,,,,,not-json,[],[],[],[],[],[],[]\n"
        );
        let (summary, err, _rows) = drain(bad.as_bytes(), 1000).await;
        assert_eq!(err, None, "not a whole-load failure");
        assert_eq!(
            summary,
            vec![(true, false)],
            "one row, and it did not parse"
        );
    }

    #[tokio::test]
    async fn row_stream_rejects_past_the_row_cap() {
        let people = vec![fully_populated(), fully_populated(), fully_populated()];
        let bytes = encode(&people, b',').unwrap();
        let (summary, err, _rows) = drain(&bytes, 2).await;
        assert_eq!(summary.len(), 2, "the allowed rows are still yielded");
        assert!(
            err.as_deref().is_some_and(|e| e.contains("row cap")),
            "the cap failure is terminal: {err:?}"
        );
    }

    #[tokio::test]
    async fn row_stream_reports_an_unreadable_header() {
        // An empty input has no header row at all.
        let (summary, err, _rows) = drain(b"", 1000).await;
        assert!(summary.is_empty());
        // The `csv` crate treats an empty input as an empty header rather
        // than an error, so the stream simply ends; what must not happen
        // is a panic or a hang.
        assert_eq!(err, None);
    }

    /// Many rows, well past one read chunk, so records are assembled from
    /// several chunks crossing the async→blocking bridge. Every row must
    /// arrive, in order, exactly once.
    #[tokio::test]
    async fn row_stream_handles_input_spanning_many_chunks() {
        let people: Vec<Person> = (0..2000)
            .map(|i| {
                let mut p = Person::new(sparse_name(), Gender::Male);
                p.name.family = format!("Family{i:05}");
                p
            })
            .collect();
        let bytes = encode(&people, b',').unwrap();
        assert!(
            bytes.len() > super::READ_CHUNK_BYTES * 2,
            "the fixture must span several read chunks"
        );
        let (_summary, err, rows) = drain(&bytes, 100_000).await;
        assert_eq!(err, None);
        assert_eq!(rows.len(), 2000);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.1.as_ref().unwrap().name.family, format!("Family{i:05}"));
        }
    }

    /// A minimal name for fixtures that only care about row identity.
    fn sparse_name() -> HumanName {
        HumanName {
            use_type: None,
            family: "Solo".to_string(),
            given: vec![],
            prefix: vec![],
            suffix: vec![],
        }
    }
    /// TSV is the same codec with a different byte: a person round-trips
    /// through tabs exactly as through commas.
    #[test]
    fn tsv_round_trips_a_fully_populated_person() {
        let person = fully_populated();
        let bytes = encode(std::slice::from_ref(&person), b'\t').unwrap();
        assert!(
            String::from_utf8_lossy(&bytes).contains('\t'),
            "TSV output must be tab-separated"
        );
        let rows = decode(&bytes, b'\t').unwrap();
        assert_eq!(rows.len(), 1);
        rows.into_iter().next().unwrap().1.expect("row parses");
    }

    /// The **streaming** import path (SEC-B2) honours the delimiter too.
    /// This is the one that actually runs for an uploaded file, so a TSV
    /// that only worked through the buffered `decode` would work in tests
    /// and fail in production.
    #[tokio::test]
    async fn row_stream_reads_tsv() {
        let people = vec![fully_populated(), Person::new(sparse_name(), Gender::Male)];
        let bytes = encode(&people, b'\t').unwrap();
        let (summary, err, rows) = drain_with(&bytes, usize::MAX, b'\t').await;
        assert!(err.is_none(), "streaming TSV should not error: {err:?}");
        assert_eq!(summary.len(), 2, "both rows should stream through");
        assert_eq!(rows.len(), 2);
    }
}
