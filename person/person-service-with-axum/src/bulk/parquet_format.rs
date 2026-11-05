//! Parquet codec — the analytics / large-export format
//! (`agents/share/bulk-import-export.md` §5). **Export-only**
//! ([`BulkFormat::Parquet`](super::BulkFormat::Parquet) is
//! [`is_export_only`](super::BulkFormat::is_export_only); §12's lean:
//! "export-only in v1, import is roadmap") and **feature-gated** behind
//! this crate's `parquet` Cargo feature — this whole module, and the
//! `arrow`/`parquet` dependencies it needs, only exist when the feature
//! is on, so the default dependency tree carries none of that weight.
//!
//! Flattens the person wire type per the shared [`super::columns`]
//! declaration — the same column set the CSV codec uses (§5's "nested via
//! Parquet nested types or JSON-encoded columns" names both as
//! acceptable; this crate takes the JSON-encoded option, matching CSV's
//! own choice rather than inventing a second, divergent nested schema) —
//! but into typed, nullable Arrow columns rather than text cells:
//!
//! - [`Kind::Scalar`] → a nullable `Utf8` column (null for an absent
//!   field — a real null, not CSV's ambiguous empty string);
//! - [`Kind::Bool`] → a nullable `Boolean` column (null for an absent
//!   field);
//! - [`Kind::Json`] → a non-nullable `Utf8` column carrying the same
//!   compact JSON text CSV puts in its JSON-encoded cells (always
//!   present, never omitted, so a required array such as `name.given`
//!   always has a value — schema-nullable regardless, since Arrow does
//!   not distinguish "never null" at the type level, but the writer never
//!   emits one).

use std::sync::Arc;

use arrow::array::{ArrayRef, BooleanBuilder, StringBuilder};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;

use crate::models::Person;
use crate::{Error, Result};

use super::columns::{COLUMNS, Kind, get};

/// Build the Arrow schema from [`COLUMNS`]: one field per column, in the
/// same order as the CSV header, typed `Utf8` (scalar / json) or
/// `Boolean` (bool), all nullable.
fn schema() -> SchemaRef {
    let fields: Vec<Field> = COLUMNS
        .iter()
        .map(|c| {
            let data_type = match c.kind {
                Kind::Scalar | Kind::Json => DataType::Utf8,
                Kind::Bool => DataType::Boolean,
            };
            Field::new(c.header, data_type, true)
        })
        .collect();
    Arc::new(Schema::new(fields))
}

/// Build one column's Arrow array from every record's `Value` at that
/// column's path (§5 flattening, mirroring [`super::columns::get`]).
fn build_column(values: &[serde_json::Value], column: &super::columns::Column) -> ArrayRef {
    match column.kind {
        Kind::Bool => {
            let mut builder = BooleanBuilder::with_capacity(values.len());
            for v in values {
                match get(v, column.path) {
                    serde_json::Value::Bool(b) => builder.append_value(*b),
                    _ => builder.append_null(),
                }
            }
            Arc::new(builder.finish())
        }
        Kind::Scalar => {
            let mut builder = StringBuilder::with_capacity(values.len(), 0);
            for v in values {
                match get(v, column.path) {
                    serde_json::Value::Null => builder.append_null(),
                    serde_json::Value::String(s) => builder.append_value(s),
                    other => builder.append_value(other.to_string()),
                }
            }
            Arc::new(builder.finish())
        }
        Kind::Json => {
            // Always present (never omitted) — see the module docs.
            let mut builder = StringBuilder::with_capacity(values.len(), 0);
            for v in values {
                builder.append_value(get(v, column.path).to_string());
            }
            Arc::new(builder.finish())
        }
    }
}

/// Encode a slice of persons to Parquet bytes (one `RecordBatch`, one row
/// group), the export output shape.
///
/// # Errors
///
/// Returns [`Error::Api`] if a person fails to serialize to its wire
/// `Value`, the Arrow record batch cannot be built (a column-length
/// mismatch, which [`build_column`] cannot itself cause), or the Parquet
/// writer fails.
pub fn encode(persons: &[Person]) -> Result<Vec<u8>> {
    let values: Vec<serde_json::Value> = persons
        .iter()
        .map(serde_json::to_value)
        .collect::<serde_json::Result<_>>()
        .map_err(|e| Error::Api(format!("serialize person to Parquet: {e}")))?;

    let schema = schema();
    let arrays: Vec<ArrayRef> = COLUMNS.iter().map(|c| build_column(&values, c)).collect();
    let batch = RecordBatch::try_new(Arc::clone(&schema), arrays)
        .map_err(|e| Error::Api(format!("build Parquet record batch: {e}")))?;

    let mut buf = Vec::new();
    let mut writer = ArrowWriter::try_new(&mut buf, schema, None)
        .map_err(|e| Error::Api(format!("open Parquet writer: {e}")))?;
    writer
        .write(&batch)
        .map_err(|e| Error::Api(format!("write Parquet batch: {e}")))?;
    writer
        .close()
        .map_err(|e| Error::Api(format!("finish Parquet file: {e}")))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::encode;
    use crate::models::{Gender, HumanName, Identifier, IdentifierType, Person};
    use arrow::array::Array;
    use bytes::Bytes;
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

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

    /// The bytes `encode` produces are a **readable** Parquet file: reading
    /// them back through `parquet`'s own Arrow reader reproduces the row
    /// count, the scalar `name.family` column, the always-present JSON
    /// `identifiers` column, and a genuine SQL-style null (not an empty
    /// string) for an absent scalar (`tax_id`).
    #[test]
    fn encoded_bytes_are_readable_and_round_trip_scalars_and_json_cells() {
        let people = vec![sample("Lovelace"), sample("Byron")];
        let bytes = encode(&people).unwrap();
        assert!(!bytes.is_empty());

        let reader = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(bytes))
            .unwrap()
            .build()
            .unwrap();
        let batches: Vec<_> = reader.map(|b| b.unwrap()).collect();
        let total_rows: usize = batches
            .iter()
            .map(arrow::array::RecordBatch::num_rows)
            .sum();
        assert_eq!(total_rows, 2);

        let batch = &batches[0];
        let family_col = batch
            .column_by_name("name.family")
            .unwrap()
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .unwrap();
        assert_eq!(family_col.value(0), "Lovelace");

        let identifiers_col = batch
            .column_by_name("identifiers")
            .unwrap()
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .unwrap();
        assert!(
            identifiers_col.value(0).contains("123-45-6789"),
            "the JSON-encoded identifiers cell carries the SSN: {}",
            identifiers_col.value(0)
        );

        let tax_id_col = batch
            .column_by_name("tax_id")
            .unwrap()
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .unwrap();
        assert!(
            tax_id_col.is_null(0),
            "an absent scalar is a real Arrow null, not an empty string"
        );
    }

    /// An empty person slice still produces a valid, readable (zero-row)
    /// Parquet file rather than an error or malformed bytes.
    #[test]
    fn encodes_an_empty_slice_to_a_readable_zero_row_file() {
        let bytes = encode(&[]).unwrap();
        let reader = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(bytes))
            .unwrap()
            .build()
            .unwrap();
        let total_rows: usize = reader.map(|b| b.unwrap()).map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 0);
    }

    /// A `Bool` column (`active`) round-trips `true`/`false` as real
    /// Arrow booleans, not stringified text.
    #[test]
    fn bool_columns_are_typed_booleans() {
        let mut p = sample("Solo");
        p.active = false;
        let bytes = encode(std::slice::from_ref(&p)).unwrap();
        let reader = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(bytes))
            .unwrap()
            .build()
            .unwrap();
        let batch = reader.map(|b| b.unwrap()).next().unwrap();
        let active_col = batch
            .column_by_name("active")
            .unwrap()
            .as_any()
            .downcast_ref::<arrow::array::BooleanArray>()
            .unwrap();
        assert!(!active_col.value(0));
    }
}
