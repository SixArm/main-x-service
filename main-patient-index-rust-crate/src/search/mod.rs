//! Search functionality using Tantivy

use tantivy::{
    collector::TopDocs,
    doc,
    query::{Query, QueryParser, FuzzyTermQuery, BooleanQuery, Occur},
    schema::{Term, Value},
};
use std::path::Path;

use crate::models::Patient;
use crate::Result;

pub mod index;
pub mod query;

pub use index::{PatientIndex, PatientIndexSchema, IndexStats};

/// Search engine for patient records
pub struct SearchEngine {
    index: PatientIndex,
}

impl SearchEngine {
    /// Create a new search engine instance
    pub fn new<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let index = PatientIndex::create_or_open(index_path)?;
        Ok(Self { index })
    }

    /// Index a patient record
    pub fn index_patient(&self, patient: &Patient) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let schema = self.index.schema();

        // Build full name
        let full_name = patient.full_name();

        // Collect given names
        let given_names = patient.name.given.join(" ");

        // Collect identifiers
        let identifiers: Vec<String> = patient
            .identifiers
            .iter()
            .map(|id| format!("{}:{}", id.identifier_type.to_string(), id.value))
            .collect();
        let identifiers_str = identifiers.join(" ");

        // Get primary address components
        let (postal_code, city, state) = if let Some(addr) = patient.addresses.first() {
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
            schema.id => patient.id.to_string(),
            schema.family_name => patient.name.family.clone(),
            schema.given_names => given_names,
            schema.full_name => full_name,
            schema.birth_date => patient.birth_date.map(|d| d.to_string()).unwrap_or_default(),
            schema.gender => format!("{:?}", patient.gender).to_lowercase(),
            schema.postal_code => postal_code,
            schema.city => city,
            schema.state => state,
            schema.identifiers => identifiers_str,
            schema.active => if patient.active { "true" } else { "false" },
        );

        writer.add_document(doc)
            .map_err(|e| crate::Error::Search(format!("Failed to add document: {}", e)))?;

        writer.commit()
            .map_err(|e| crate::Error::Search(format!("Failed to commit: {}", e)))?;

        Ok(())
    }

    /// Bulk index multiple patients
    pub fn index_patients(&self, patients: &[Patient]) -> Result<()> {
        let mut writer = self.index.writer(100)?;
        let schema = self.index.schema();

        for patient in patients {
            let full_name = patient.full_name();
            let given_names = patient.name.given.join(" ");
            let identifiers: Vec<String> = patient
                .identifiers
                .iter()
                .map(|id| format!("{}:{}", id.identifier_type.to_string(), id.value))
                .collect();
            let identifiers_str = identifiers.join(" ");

            let (postal_code, city, state) = if let Some(addr) = patient.addresses.first() {
                (
                    addr.postal_code.clone().unwrap_or_default(),
                    addr.city.clone().unwrap_or_default(),
                    addr.state.clone().unwrap_or_default(),
                )
            } else {
                (String::new(), String::new(), String::new())
            };

            let doc = doc!(
                schema.id => patient.id.to_string(),
                schema.family_name => patient.name.family.clone(),
                schema.given_names => given_names,
                schema.full_name => full_name,
                schema.birth_date => patient.birth_date.map(|d| d.to_string()).unwrap_or_default(),
                schema.gender => format!("{:?}", patient.gender).to_lowercase(),
                schema.postal_code => postal_code,
                schema.city => city,
                schema.state => state,
                schema.identifiers => identifiers_str,
                schema.active => if patient.active { "true" } else { "false" },
            );

            writer.add_document(doc)
                .map_err(|e| crate::Error::Search(format!("Failed to add document: {}", e)))?;
        }

        writer.commit()
            .map_err(|e| crate::Error::Search(format!("Failed to commit: {}", e)))?;

        Ok(())
    }

    /// Search for patients by query string
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

        let mut patient_ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| crate::Error::Search(format!("Failed to retrieve document: {}", e)))?;

            if let Some(id_value) = retrieved_doc.get_first(schema.id) {
                if let Some(id_text) = id_value.as_str() {
                    patient_ids.push(id_text.to_string());
                }
            }
        }

        Ok(patient_ids)
    }

    /// Search for patients with fuzzy matching
    pub fn fuzzy_search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let schema = self.index.schema();

        // Build fuzzy query for family name
        let term = Term::from_field_text(schema.family_name, query_str);
        let fuzzy_query = FuzzyTermQuery::new(term, 2, true);

        let top_docs = searcher
            .search(&fuzzy_query, &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("Fuzzy search failed: {}", e)))?;

        let mut patient_ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| crate::Error::Search(format!("Failed to retrieve document: {}", e)))?;

            if let Some(id_value) = retrieved_doc.get_first(schema.id) {
                if let Some(id_text) = id_value.as_str() {
                    patient_ids.push(id_text.to_string());
                }
            }
        }

        Ok(patient_ids)
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

        let mut patient_ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| crate::Error::Search(format!("Failed to retrieve document: {}", e)))?;

            if let Some(id_value) = retrieved_doc.get_first(schema.id) {
                if let Some(id_text) = id_value.as_str() {
                    patient_ids.push(id_text.to_string());
                }
            }
        }

        Ok(patient_ids)
    }

    /// Remove a patient from the index
    pub fn delete_patient(&self, patient_id: &str) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let schema = self.index.schema();

        let term = Term::from_field_text(schema.id, patient_id);
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

    fn create_test_patient(family: &str, given: &str, birth_date: Option<NaiveDate>) -> Patient {
        Patient {
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
    fn test_index_and_search_patient() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let patient = create_test_patient("Smith", "John", None);
        engine.index_patient(&patient).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new document

        let results = engine.search("Smith", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], patient.id.to_string());
    }

    #[test]
    fn test_fuzzy_search() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let patient = create_test_patient("Smith", "John", None);
        engine.index_patient(&patient).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new document

        // Fuzzy search with typo
        let results = engine.fuzzy_search("Smyth", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], patient.id.to_string());
    }

    #[test]
    fn test_bulk_indexing() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let patients = vec![
            create_test_patient("Smith", "John", None),
            create_test_patient("Johnson", "Jane", None),
            create_test_patient("Williams", "Bob", None),
        ];

        engine.index_patients(&patients).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new documents

        let stats = engine.stats().unwrap();
        assert_eq!(stats.num_docs, 3);
    }

    #[test]
    fn test_delete_patient() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let patient = create_test_patient("Smith", "John", None);
        let patient_id = patient.id.to_string();

        engine.index_patient(&patient).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new document
        assert_eq!(engine.stats().unwrap().num_docs, 1);

        engine.delete_patient(&patient_id).unwrap();
        engine.reload().unwrap(); // Ensure reader sees deletion

        let results = engine.search("Smith", 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_by_name_and_year() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let patient = create_test_patient("Smith", "John", dob);
        engine.index_patient(&patient).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new document

        let results = engine.search_by_name_and_year("Smith", Some(1980), 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], patient.id.to_string());
    }
}
