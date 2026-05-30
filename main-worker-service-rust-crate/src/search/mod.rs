//! Search functionality using Tantivy

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

/// Search engine for worker records
pub struct SearchEngine {
    index: WorkerIndex,
}

impl SearchEngine {
    /// Create a new search engine instance
    pub fn new<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let index = WorkerIndex::create_or_open(index_path)?;
        Ok(Self { index })
    }

    /// Index a worker record
    pub fn index_worker(&self, worker: &Worker) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let schema = self.index.schema();

        // Build full name
        let full_name = worker.full_name();

        // Collect given names
        let given_names = worker.name.given.join(" ");

        // Collect identifiers
        let identifiers: Vec<String> = worker
            .identifiers
            .iter()
            .map(|id| format!("{}:{}", id.identifier_type.to_string(), id.value))
            .collect();
        let identifiers_str = identifiers.join(" ");

        // Get primary address components
        let (postal_code, city, state) = if let Some(addr) = worker.addresses.first() {
            (
                addr.postal_code.clone().unwrap_or_default(),
                addr.city.clone().unwrap_or_default(),
                addr.state.clone().unwrap_or_default(),
            )
        } else {
            (String::new(), String::new(), String::new())
        };

        // Create document
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

    /// Bulk index multiple workers
    pub fn index_workers(&self, workers: &[Worker]) -> Result<()> {
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

    /// Search for workers by query string
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let schema = self.index.schema();

        // Create query parser for name and identifier fields
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

    /// Search for workers with fuzzy matching
    pub fn fuzzy_search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let schema = self.index.schema();

        // Build fuzzy query for family name
        let term = Term::from_field_text(schema.family_name, query_str);
        let fuzzy_query = FuzzyTermQuery::new(term, 2, true);

        let top_docs = searcher
            .search(&fuzzy_query, &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("Fuzzy search failed: {}", e)))?;

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

    /// Search by name and birth year (for blocking in matching)
    pub fn search_by_name_and_year(
        &self,
        family_name: &str,
        birth_year: Option<i32>,
        limit: usize,
    ) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let schema = self.index.schema();

        // Build fuzzy query for family name
        let name_term = Term::from_field_text(schema.family_name, family_name);
        let name_query: Box<dyn Query> = Box::new(FuzzyTermQuery::new(name_term, 2, true));

        // If birth year provided, add it to the query
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
                name_query
            }
        } else {
            name_query
        };

        let top_docs = searcher
            .search(final_query.as_ref(), &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("Search failed: {}", e)))?;

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

    /// Remove a worker from the index
    pub fn delete_worker(&self, worker_id: &str) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let schema = self.index.schema();

        let term = Term::from_field_text(schema.id, worker_id);
        writer.delete_term(term);

        writer.commit()
            .map_err(|e| crate::Error::Search(format!("Failed to commit deletion: {}", e)))?;

        Ok(())
    }

    /// Get index statistics
    pub fn stats(&self) -> Result<IndexStats> {
        self.index.stats()
    }

    /// Optimize the index
    pub fn optimize(&self) -> Result<()> {
        self.index.optimize()
    }

    /// Manually reload the index reader (useful for tests to ensure documents are visible)
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
