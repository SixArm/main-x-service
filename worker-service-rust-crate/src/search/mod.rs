//! Full-text search over worker records, backed by Tantivy.
//!
//! [`SearchEngine`](crate::search::SearchEngine) wraps a
//! [`WorkerIndex`](crate::search::index::WorkerIndex) and exposes index/search
//! operations used by the REST search endpoint and by the matching layer's
//! candidate-blocking step. Indexed fields include the names, birth date,
//! gender, primary-address components, and identifiers; see
//! [`index`](crate::search::index) for the schema.
//!
//! Searches return worker-ID strings; the caller hydrates full records from
//! the repository. The index is kept in sync with database writes by the
//! handlers that call [`index_worker`](crate::search::SearchEngine::index_worker) and
//! [`delete_worker`](crate::search::SearchEngine::delete_worker).

use tantivy::{
    collector::TopDocs,
    doc,
    query::{Query, QueryParser, FuzzyTermQuery, BooleanQuery, Occur},
    schema::{Term, Value},
};
use std::path::Path;

use crate::models::Worker;
use crate::Result;

pub mod index;
pub mod query;

pub use index::{WorkerIndex, WorkerIndexSchema, IndexStats};

/// Search engine for worker records.
///
/// A thin wrapper over [`WorkerIndex`] (the Tantivy index plus reader/writer
/// handles) that translates [`Worker`] domain records into index documents
/// and translates query hits back into worker-ID strings. The REST search
/// endpoint and the matching layer's candidate-blocking step both call into
/// this type. Construct one per index directory via [`SearchEngine::new`].
pub struct SearchEngine {
    /// The underlying Tantivy index, schema, reader, and writer factory.
    index: WorkerIndex,
}

impl SearchEngine {
    /// Opens the Tantivy index at `index_path`, creating it (and the schema)
    /// if the directory does not yet hold one. Returns an error if the index
    /// cannot be created or opened.
    pub fn new<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let index = WorkerIndex::create_or_open(index_path)?;
        Ok(Self { index })
    }

    /// Adds a single `worker` to the index and commits immediately so the
    /// document is durable.
    ///
    /// The worker is flattened into the index schema: family name, joined
    /// given names, full name, birth date (as a string), lower-cased gender,
    /// the primary address's postal code / city / state, a space-joined
    /// `type:value` list of identifiers, worker type, and active flag. For
    /// bulk loads prefer [`index_workers`](Self::index_workers), which commits
    /// once for the whole batch.
    pub fn index_worker(&self, worker: &Worker) -> Result<()> {
        // 50 MB writer heap; the writer is dropped at end of scope after commit.
        let mut writer = self.index.writer(50)?;
        let schema = self.index.schema();

        // Build full name ("Given Family") for the free-text name field.
        let full_name = worker.full_name();

        // Join the given-name list into a single searchable field.
        let given_names = worker.name.given.join(" ");

        // Flatten identifiers to "TYPE:value" tokens so a search for either
        // the type or the value matches.
        let identifiers: Vec<String> = worker
            .identifiers
            .iter()
            .map(|id| format!("{}:{}", id.identifier_type.to_string(), id.value))
            .collect();
        let identifiers_str = identifiers.join(" ");

        // Only the first (primary) address is indexed; missing components
        // become empty strings rather than absent fields.
        let (postal_code, city, state) = if let Some(addr) = worker.addresses.first() {
            (
                addr.postal_code.clone().unwrap_or_default(),
                addr.city.clone().unwrap_or_default(),
                addr.state.clone().unwrap_or_default(),
            )
        } else {
            (String::new(), String::new(), String::new())
        };

        // Assemble the Tantivy document from the schema field handles.
        let doc = doc!(
            schema.id => worker.id.to_string(),
            schema.family_name => worker.name.family.clone(),
            schema.given_names => given_names,
            schema.full_name => full_name,
            schema.birth_date => worker.birth_date.map(|d| d.to_string()).unwrap_or_default(),
            schema.gender => format!("{:?}", worker.gender).to_lowercase(),
            schema.postal_code => postal_code,
            schema.city => city,
            schema.state => state,
            schema.identifiers => identifiers_str,
            schema.worker_type => worker.worker_type.as_ref().map(|wt| wt.to_string()).unwrap_or_default(),
            schema.active => if worker.active { "true" } else { "false" },
        );

        writer.add_document(doc)
            .map_err(|e| crate::Error::Search(format!("Failed to add document: {}", e)))?;

        writer.commit()
            .map_err(|e| crate::Error::Search(format!("Failed to commit: {}", e)))?;

        Ok(())
    }

    /// Indexes a slice of workers in one writer session, committing only once
    /// at the end. Preferred over repeated [`index_worker`](Self::index_worker)
    /// calls for initial loads or re-indexing, as a single commit is far
    /// cheaper than one per document.
    pub fn index_workers(&self, workers: &[Worker]) -> Result<()> {
        // Larger 100 MB heap for batch throughput.
        let mut writer = self.index.writer(100)?;
        let schema = self.index.schema();

        for worker in workers {
            let full_name = worker.full_name();
            let given_names = worker.name.given.join(" ");
            let identifiers: Vec<String> = worker
                .identifiers
                .iter()
                .map(|id| format!("{}:{}", id.identifier_type.to_string(), id.value))
                .collect();
            let identifiers_str = identifiers.join(" ");

            let (postal_code, city, state) = if let Some(addr) = worker.addresses.first() {
                (
                    addr.postal_code.clone().unwrap_or_default(),
                    addr.city.clone().unwrap_or_default(),
                    addr.state.clone().unwrap_or_default(),
                )
            } else {
                (String::new(), String::new(), String::new())
            };

            let doc = doc!(
                schema.id => worker.id.to_string(),
                schema.family_name => worker.name.family.clone(),
                schema.given_names => given_names,
                schema.full_name => full_name,
                schema.birth_date => worker.birth_date.map(|d| d.to_string()).unwrap_or_default(),
                schema.gender => format!("{:?}", worker.gender).to_lowercase(),
                schema.postal_code => postal_code,
                schema.city => city,
                schema.state => state,
                schema.identifiers => identifiers_str,
                schema.worker_type => worker.worker_type.as_ref().map(|wt| wt.to_string()).unwrap_or_default(),
                schema.active => if worker.active { "true" } else { "false" },
            );

            writer.add_document(doc)
                .map_err(|e| crate::Error::Search(format!("Failed to add document: {}", e)))?;
        }

        writer.commit()
            .map_err(|e| crate::Error::Search(format!("Failed to commit: {}", e)))?;

        Ok(())
    }

    /// Runs a full-text query over the name and identifier fields and returns
    /// up to `limit` matching worker-ID strings, ranked by relevance.
    ///
    /// `query_str` uses Tantivy's [`QueryParser`] syntax (supports boolean
    /// operators, field prefixes, etc.). The caller hydrates full [`Worker`]
    /// records from the repository using the returned IDs. For typo-tolerant
    /// lookups use [`fuzzy_search`](Self::fuzzy_search).
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let schema = self.index.schema();

        // Search across the free-text name fields and the identifier tokens.
        let query_parser = QueryParser::for_index(
            self.index.index(),
            vec![
                schema.full_name,
                schema.family_name,
                schema.given_names,
                schema.identifiers,
            ],
        );

        let query = query_parser
            .parse_query(query_str)
            .map_err(|e| crate::Error::Search(format!("Failed to parse query: {}", e)))?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("Search failed: {}", e)))?;

        // Resolve each hit's stored `id` field into a worker-ID string.
        let mut worker_ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| crate::Error::Search(format!("Failed to retrieve document: {}", e)))?;

            if let Some(id_value) = retrieved_doc.get_first(schema.id) {
                if let Some(id_text) = id_value.as_str() {
                    worker_ids.push(id_text.to_string());
                }
            }
        }

        Ok(worker_ids)
    }

    /// Performs a typo-tolerant search on the family name, returning up to
    /// `limit` worker-ID strings.
    ///
    /// Uses a Tantivy [`FuzzyTermQuery`] with a Levenshtein edit distance of
    /// `2` and prefix matching enabled, so "Smyth" matches "Smith". Useful
    /// when the input may contain spelling errors.
    pub fn fuzzy_search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let schema = self.index.schema();

        // Edit distance 2, prefix-enabled, against the family-name field.
        let term = Term::from_field_text(schema.family_name, query_str);
        let fuzzy_query = FuzzyTermQuery::new(term, 2, true);

        let top_docs = searcher
            .search(&fuzzy_query, &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("Fuzzy search failed: {}", e)))?;

        // Resolve each hit's stored `id` field into a worker-ID string.
        let mut worker_ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| crate::Error::Search(format!("Failed to retrieve document: {}", e)))?;

            if let Some(id_value) = retrieved_doc.get_first(schema.id) {
                if let Some(id_text) = id_value.as_str() {
                    worker_ids.push(id_text.to_string());
                }
            }
        }

        Ok(worker_ids)
    }

    /// Blocking query for the matching layer: finds candidate workers by a
    /// fuzzy family-name match, optionally boosted by birth year.
    ///
    /// The family name is matched with edit distance 2 (a `Must` clause); when
    /// `birth_year` is supplied it is added as a `Should` clause so matching
    /// years rank higher without excluding name-only matches. Returns up to
    /// `limit` worker-ID strings — a reduced candidate set the matcher then
    /// scores precisely.
    pub fn search_by_name_and_year(
        &self,
        family_name: &str,
        birth_year: Option<i32>,
        limit: usize,
    ) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let schema = self.index.schema();

        // Required clause: fuzzy family-name match (edit distance 2).
        let name_term = Term::from_field_text(schema.family_name, family_name);
        let name_query: Box<dyn Query> = Box::new(FuzzyTermQuery::new(name_term, 2, true));

        // When a birth year is given, OR it in as a `Should` clause so it
        // boosts ranking but does not filter out name-only matches.
        let final_query: Box<dyn Query> = if let Some(year) = birth_year {
            let year_str = year.to_string();
            let year_query_parser = QueryParser::for_index(
                self.index.index(),
                vec![schema.birth_date],
            );

            if let Ok(year_query) = year_query_parser.parse_query(&year_str) {
                Box::new(BooleanQuery::new(vec![
                    (Occur::Must, name_query),
                    (Occur::Should, year_query),
                ]))
            } else {
                // If the year fails to parse as a query, fall back to name only.
                name_query
            }
        } else {
            name_query
        };

        let top_docs = searcher
            .search(final_query.as_ref(), &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("Search failed: {}", e)))?;

        // Resolve each hit's stored `id` field into a worker-ID string.
        let mut worker_ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| crate::Error::Search(format!("Failed to retrieve document: {}", e)))?;

            if let Some(id_value) = retrieved_doc.get_first(schema.id) {
                if let Some(id_text) = id_value.as_str() {
                    worker_ids.push(id_text.to_string());
                }
            }
        }

        Ok(worker_ids)
    }

    /// Removes the worker whose `id` field equals `worker_id` from the index,
    /// committing the deletion. Called when a record is soft-deleted or merged
    /// away so it no longer appears in search results.
    pub fn delete_worker(&self, worker_id: &str) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let schema = self.index.schema();

        // Delete by exact term on the unique `id` field.
        let term = Term::from_field_text(schema.id, worker_id);
        writer.delete_term(term);

        writer.commit()
            .map_err(|e| crate::Error::Search(format!("Failed to commit deletion: {}", e)))?;

        Ok(())
    }

    /// Returns index statistics (e.g. the live document count) from the
    /// underlying [`WorkerIndex`].
    pub fn stats(&self) -> Result<IndexStats> {
        self.index.stats()
    }

    /// Merges the index segments to reclaim space and improve query speed.
    /// Delegates to the underlying [`WorkerIndex`].
    pub fn optimize(&self) -> Result<()> {
        self.index.optimize()
    }

    /// Forces the index reader to pick up the latest committed documents.
    ///
    /// Tantivy readers are eventually consistent, so a commit is not
    /// immediately visible to searches. Tests call this after indexing to
    /// guarantee the just-written documents are searchable.
    pub fn reload(&self) -> Result<()> {
        self.index.reload()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{HumanName, Gender};
    use chrono::{Utc, NaiveDate};
    use tempfile::TempDir;
    use uuid::Uuid;

    /// Builds a minimal active worker with the given name and optional birth
    /// date; all other fields are empty/default for focused search tests.
    fn create_test_worker(family: &str, given: &str, birth_date: Option<NaiveDate>) -> Worker {
        Worker {
            id: Uuid::new_v4(),
            identifiers: vec![],
            active: true,
            name: HumanName {
                use_type: None,
                family: family.to_string(),
                given: vec![given.to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            additional_names: vec![],
            telecom: vec![],
            gender: Gender::Male,
            worker_type: None,
            birth_date,
            tax_id: None,
            documents: vec![],
            emergency_contacts: vec![],
            deceased: false,
            deceased_datetime: None,
            addresses: vec![],
            marital_status: None,
            multiple_birth: None,
            photo: vec![],
            managing_organization: None,
            links: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Indexing then searching a family name returns that worker's ID.
    #[test]
    fn test_index_and_search_worker() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let worker = create_test_worker("Smith", "John", None);
        engine.index_worker(&worker).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new document

        let results = engine.search("Smith", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], worker.id.to_string());
    }

    /// Fuzzy search tolerates a typo ("Smyth" finds "Smith").
    #[test]
    fn test_fuzzy_search() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let worker = create_test_worker("Smith", "John", None);
        engine.index_worker(&worker).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new document

        // Fuzzy search with typo
        let results = engine.fuzzy_search("Smyth", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], worker.id.to_string());
    }

    /// Bulk indexing three workers yields a document count of three.
    #[test]
    fn test_bulk_indexing() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let workers = vec![
            create_test_worker("Smith", "John", None),
            create_test_worker("Johnson", "Jane", None),
            create_test_worker("Williams", "Bob", None),
        ];

        engine.index_workers(&workers).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new documents

        let stats = engine.stats().unwrap();
        assert_eq!(stats.num_docs, 3);
    }

    /// Deleting an indexed worker removes it from subsequent search results.
    #[test]
    fn test_delete_worker() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let worker = create_test_worker("Smith", "John", None);
        let worker_id = worker.id.to_string();

        engine.index_worker(&worker).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new document
        assert_eq!(engine.stats().unwrap().num_docs, 1);

        engine.delete_worker(&worker_id).unwrap();
        engine.reload().unwrap(); // Ensure reader sees deletion

        let results = engine.search("Smith", 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    /// Name-plus-year blocking returns the matching worker's ID.
    #[test]
    fn test_search_by_name_and_year() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let worker = create_test_worker("Smith", "John", dob);
        engine.index_worker(&worker).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new document

        let results = engine.search_by_name_and_year("Smith", Some(1980), 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], worker.id.to_string());
    }
}
