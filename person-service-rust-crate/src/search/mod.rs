//! Search functionality using Tantivy

use tantivy::{
    collector::TopDocs,
    doc,
    query::{Query, QueryParser, FuzzyTermQuery, BooleanQuery, Occur},
    schema::{Term, Value},
};
use std::path::Path;

use crate::models::Person;
use crate::Result;

pub mod index;
pub mod query;

pub use index::{PersonIndex, PersonIndexSchema, IndexStats};

/// Search engine for person records
pub struct SearchEngine {
    index: PersonIndex,
}

impl SearchEngine {
    /// Create a new search engine instance
    pub fn new<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let index = PersonIndex::create_or_open(index_path)?;
        Ok(Self { index })
    }

    /// Index a person record
    pub fn index_person(&self, person: &Person) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let schema = self.index.schema();

        // Build full name
        let full_name = person.full_name();

        // Collect given names
        let given_names = person.name.given.join(" ");

        // Collect identifiers
        let identifiers: Vec<String> = person
            .identifiers
            .iter()
            .map(|id| format!("{}:{}", id.identifier_type.to_string(), id.value))
            .collect();
        let identifiers_str = identifiers.join(" ");

        // Get primary address components
        let (postal_code, city, state) = if let Some(addr) = person.addresses.first() {
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
            schema.id => person.id.to_string(),
            schema.family_name => person.name.family.clone(),
            schema.given_names => given_names,
            schema.full_name => full_name,
            schema.birth_date => person.birth_date.map(|d| d.to_string()).unwrap_or_default(),
            schema.gender => format!("{:?}", person.gender).to_lowercase(),
            schema.postal_code => postal_code,
            schema.city => city,
            schema.state => state,
            schema.identifiers => identifiers_str,
            schema.active => if person.active { "true" } else { "false" },
        );

        writer.add_document(doc)
            .map_err(|e| crate::Error::Search(format!("Failed to add document: {}", e)))?;

        writer.commit()
            .map_err(|e| crate::Error::Search(format!("Failed to commit: {}", e)))?;

        // Force reader to pick up the new segment so a search issued
        // immediately after this call (as the create / update / merge
        // handlers do via the audit-then-search dance in the e2e
        // suite) sees the new record. Without this the default
        // `OnCommitWithDelay` policy lets queries observe a stale view.
        self.index.reload()?;
        Ok(())
    }

    /// Bulk index multiple persons
    pub fn index_persons(&self, persons: &[Person]) -> Result<()> {
        let mut writer = self.index.writer(100)?;
        let schema = self.index.schema();

        for person in persons {
            let full_name = person.full_name();
            let given_names = person.name.given.join(" ");
            let identifiers: Vec<String> = person
                .identifiers
                .iter()
                .map(|id| format!("{}:{}", id.identifier_type.to_string(), id.value))
                .collect();
            let identifiers_str = identifiers.join(" ");

            let (postal_code, city, state) = if let Some(addr) = person.addresses.first() {
                (
                    addr.postal_code.clone().unwrap_or_default(),
                    addr.city.clone().unwrap_or_default(),
                    addr.state.clone().unwrap_or_default(),
                )
            } else {
                (String::new(), String::new(), String::new())
            };

            let doc = doc!(
                schema.id => person.id.to_string(),
                schema.family_name => person.name.family.clone(),
                schema.given_names => given_names,
                schema.full_name => full_name,
                schema.birth_date => person.birth_date.map(|d| d.to_string()).unwrap_or_default(),
                schema.gender => format!("{:?}", person.gender).to_lowercase(),
                schema.postal_code => postal_code,
                schema.city => city,
                schema.state => state,
                schema.identifiers => identifiers_str,
                schema.active => if person.active { "true" } else { "false" },
            );

            writer.add_document(doc)
                .map_err(|e| crate::Error::Search(format!("Failed to add document: {}", e)))?;
        }

        writer.commit()
            .map_err(|e| crate::Error::Search(format!("Failed to commit: {}", e)))?;

        self.index.reload()?;
        Ok(())
    }

    /// Search for persons by query string
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

        let mut person_ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| crate::Error::Search(format!("Failed to retrieve document: {}", e)))?;

            if let Some(id_value) = retrieved_doc.get_first(schema.id) {
                if let Some(id_text) = id_value.as_str() {
                    person_ids.push(id_text.to_string());
                }
            }
        }

        Ok(person_ids)
    }

    /// Search for persons with fuzzy matching.
    ///
    /// Multi-token queries (e.g. `"E2E_search_test"` which the default
    /// text tokenizer splits as `["e2e", "search", "test"]`) are
    /// supported: each alphanumeric run becomes its own `FuzzyTermQuery`
    /// with edit distance 2, combined with `Occur::Should`, and any
    /// match across `family_name`, `given_names`, or `full_name`
    /// counts. A query that tokenizes to nothing returns an empty
    /// result rather than an error.
    pub fn fuzzy_search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let schema = self.index.schema();

        // Split on non-alphanumeric runs (matches the default Tantivy
        // SimpleTokenizer behaviour) and lowercase (matches the default
        // LowerCaseFilter that TEXT fields apply at index time).
        let tokens: Vec<String> = query_str
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|t| t.to_lowercase())
            .collect();

        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        let fuzzy_fields = [schema.family_name, schema.given_names, schema.full_name];
        let mut subqueries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for token in &tokens {
            for field in fuzzy_fields {
                let term = Term::from_field_text(field, token);
                subqueries.push((Occur::Should, Box::new(FuzzyTermQuery::new(term, 2, true))));
            }
        }

        let bool_query = BooleanQuery::new(subqueries);
        let top_docs = searcher
            .search(&bool_query, &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("Fuzzy search failed: {}", e)))?;

        let mut person_ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| crate::Error::Search(format!("Failed to retrieve document: {}", e)))?;

            if let Some(id_value) = retrieved_doc.get_first(schema.id) {
                if let Some(id_text) = id_value.as_str() {
                    person_ids.push(id_text.to_string());
                }
            }
        }

        Ok(person_ids)
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

        // Build fuzzy query for family name. The `family_name` field
        // is TEXT, so the indexer lowercases + splits on
        // non-alphanumeric. FuzzyTermQuery does NOT pre-process its
        // term — we have to match the indexer's normalisation
        // ourselves. For underscored or compound family names
        // (`"E2E_seed_…"`) we fan out one fuzzy clause per
        // alphanumeric run so duplicate detection finds them.
        let tokens: Vec<String> = family_name
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|t| t.to_lowercase())
            .collect();
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let name_query: Box<dyn Query> = if tokens.len() == 1 {
            Box::new(FuzzyTermQuery::new(
                Term::from_field_text(schema.family_name, &tokens[0]),
                2,
                true,
            ))
        } else {
            let subqueries: Vec<(Occur, Box<dyn Query>)> = tokens
                .iter()
                .map(|t| {
                    let q: Box<dyn Query> = Box::new(FuzzyTermQuery::new(
                        Term::from_field_text(schema.family_name, t),
                        2,
                        true,
                    ));
                    (Occur::Should, q)
                })
                .collect();
            Box::new(BooleanQuery::new(subqueries))
        };

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

        let mut person_ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| crate::Error::Search(format!("Failed to retrieve document: {}", e)))?;

            if let Some(id_value) = retrieved_doc.get_first(schema.id) {
                if let Some(id_text) = id_value.as_str() {
                    person_ids.push(id_text.to_string());
                }
            }
        }

        Ok(person_ids)
    }

    /// Remove a person from the index
    pub fn delete_person(&self, person_id: &str) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let schema = self.index.schema();

        let term = Term::from_field_text(schema.id, person_id);
        writer.delete_term(term);

        writer.commit()
            .map_err(|e| crate::Error::Search(format!("Failed to commit deletion: {}", e)))?;

        self.index.reload()?;
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

    fn create_test_person(family: &str, given: &str, birth_date: Option<NaiveDate>) -> Person {
        Person {
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
    fn test_index_and_search_person() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let person = create_test_person("Smith", "John", None);
        engine.index_person(&person).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new document

        let results = engine.search("Smith", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], person.id.to_string());
    }

    #[test]
    fn test_fuzzy_search() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let person = create_test_person("Smith", "John", None);
        engine.index_person(&person).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new document

        // Fuzzy search with typo
        let results = engine.fuzzy_search("Smyth", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], person.id.to_string());
    }

    #[test]
    fn test_bulk_indexing() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let persons = vec![
            create_test_person("Smith", "John", None),
            create_test_person("Johnson", "Jane", None),
            create_test_person("Williams", "Bob", None),
        ];

        engine.index_persons(&persons).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new documents

        let stats = engine.stats().unwrap();
        assert_eq!(stats.num_docs, 3);
    }

    #[test]
    fn test_delete_person() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let person = create_test_person("Smith", "John", None);
        let person_id = person.id.to_string();

        engine.index_person(&person).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new document
        assert_eq!(engine.stats().unwrap().num_docs, 1);

        engine.delete_person(&person_id).unwrap();
        engine.reload().unwrap(); // Ensure reader sees deletion

        let results = engine.search("Smith", 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_by_name_and_year() {
        let temp_dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(temp_dir.path()).unwrap();

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let person = create_test_person("Smith", "John", dob);
        engine.index_person(&person).unwrap();
        engine.reload().unwrap(); // Ensure reader sees new document

        let results = engine.search_by_name_and_year("Smith", Some(1980), 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], person.id.to_string());
    }
}
