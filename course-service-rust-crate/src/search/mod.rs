//! Tantivy-backed search index.
//!
//! Mirrors the family-wide pattern: open / create at a directory path,
//! index on every CRUD write, force `reload()` after each commit so
//! reads observe the new segment immediately. The duplicate detector
//! blocks candidates via `search_by_name_and_provider`.

use std::path::Path;

use tantivy::{
    TantivyDocument,
    collector::TopDocs,
    doc,
    query::{BooleanQuery, FuzzyTermQuery, Occur, Query, QueryParser, TermQuery},
    schema::{IndexRecordOption, Term, Value},
};

use crate::Result;
use crate::models::Course;

pub mod index;

pub use index::{CourseIndex, CourseIndexSchema, IndexStats};

pub struct SearchEngine {
    index: CourseIndex,
    pub index_path: String,
}

impl SearchEngine {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        std::fs::create_dir_all(p)
            .map_err(|e| crate::Error::Search(format!("ensure index dir: {e}")))?;
        let index = CourseIndex::create_or_open(p)?;
        Ok(Self {
            index,
            index_path: p.to_string_lossy().into_owned(),
        })
    }

    /// Index (or re-index) one course. Caller is responsible for
    /// having already removed any prior segment for this `id` via
    /// `delete_course`.
    pub fn index_course(&self, course: &Course) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let s = self.index.schema();

        let alt = course.alternate_names.join(" ");
        let kw = course.keywords.join(" ");
        let teaches = course.teaches.join(" ");
        let idents: String = course
            .identifiers
            .iter()
            .map(|i| i.value.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let doc = doc!(
            s.id => course.id.to_string(),
            s.name => course.name.clone(),
            s.alternate_names => alt,
            s.course_code => course.course_code.clone().unwrap_or_default(),
            s.provider_id => course.provider_id.map(|p| p.to_string()).unwrap_or_default(),
            s.provider_name => String::new(),
            s.keywords => kw,
            s.teaches => teaches,
            s.identifiers => idents,
            s.active => if course.active { "true" } else { "false" },
        );

        writer
            .add_document(doc)
            .map_err(|e| crate::Error::Search(format!("add document: {e}")))?;
        writer
            .commit()
            .map_err(|e| crate::Error::Search(format!("commit: {e}")))?;
        self.index.reload()?;
        Ok(())
    }

    /// Full-text search over name + alternate_names + keywords +
    /// teaches + identifiers.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let parser = QueryParser::for_index(
            self.index.index(),
            vec![s.name, s.alternate_names, s.keywords, s.teaches, s.identifiers],
        );
        let query = parser
            .parse_query(query_str)
            .map_err(|e| crate::Error::Search(format!("parse query: {e}")))?;
        self.collect_ids(searcher, query.as_ref(), limit)
    }

    /// Fuzzy search — tolerates typos. Multi-token queries decompose
    /// into one `FuzzyTermQuery` per (token × field), combined with
    /// `Occur::Should`. A query that tokenises to nothing returns an
    /// empty result rather than an error (matches the person-service
    /// behaviour).
    pub fn fuzzy_search(&self, query_str: &str, limit: usize) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let tokens: Vec<String> = tokenise(query_str);
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let fields = [s.name, s.alternate_names, s.keywords, s.teaches];
        let mut sub: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for t in &tokens {
            for f in fields {
                let term = Term::from_field_text(f, t);
                sub.push((Occur::Should, Box::new(FuzzyTermQuery::new(term, 2, true))));
            }
        }
        let q = BooleanQuery::new(sub);
        self.collect_ids(searcher, &q, limit)
    }

    /// Blocking query used by the duplicate detector. Fuzzy name match
    /// AND-combined with an exact `provider_id` filter when supplied.
    pub fn search_by_name_and_provider(
        &self,
        name: &str,
        provider_id: Option<uuid::Uuid>,
        limit: usize,
    ) -> Result<Vec<String>> {
        let searcher = self.index.reader().searcher();
        let s = self.index.schema();
        let tokens = tokenise(name);
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let name_query: Box<dyn Query> = if tokens.len() == 1 {
            Box::new(FuzzyTermQuery::new(
                Term::from_field_text(s.name, &tokens[0]),
                2,
                true,
            ))
        } else {
            let sub: Vec<(Occur, Box<dyn Query>)> = tokens
                .iter()
                .map(|t| {
                    let q: Box<dyn Query> =
                        Box::new(FuzzyTermQuery::new(Term::from_field_text(s.name, t), 2, true));
                    (Occur::Should, q)
                })
                .collect();
            Box::new(BooleanQuery::new(sub))
        };

        let final_q: Box<dyn Query> = if let Some(pid) = provider_id {
            let pid_term = Term::from_field_text(s.provider_id, &pid.to_string());
            let pid_q: Box<dyn Query> =
                Box::new(TermQuery::new(pid_term, IndexRecordOption::Basic));
            Box::new(BooleanQuery::new(vec![
                (Occur::Must, name_query),
                (Occur::Must, pid_q),
            ]))
        } else {
            name_query
        };

        self.collect_ids(searcher, final_q.as_ref(), limit)
    }

    /// Delete a course from the index by id.
    pub fn delete_course(&self, course_id: &str) -> Result<()> {
        let mut writer = self.index.writer(50)?;
        let s = self.index.schema();
        let term = Term::from_field_text(s.id, course_id);
        writer.delete_term(term);
        writer
            .commit()
            .map_err(|e| crate::Error::Search(format!("commit delete: {e}")))?;
        self.index.reload()?;
        Ok(())
    }

    pub fn stats(&self) -> Result<IndexStats> {
        self.index.stats()
    }

    pub fn reload(&self) -> Result<()> {
        self.index.reload()
    }

    fn collect_ids(
        &self,
        searcher: tantivy::Searcher,
        query: &dyn Query,
        limit: usize,
    ) -> Result<Vec<String>> {
        let s = self.index.schema();
        let top = searcher
            .search(query, &TopDocs::with_limit(limit))
            .map_err(|e| crate::Error::Search(format!("search: {e}")))?;
        let mut ids = Vec::with_capacity(top.len());
        for (_score, addr) in top {
            let doc: TantivyDocument = searcher
                .doc(addr)
                .map_err(|e| crate::Error::Search(format!("retrieve doc: {e}")))?;
            if let Some(v) = doc.get_first(s.id) {
                if let Some(t) = v.as_str() {
                    ids.push(t.to_string());
                }
            }
        }
        Ok(ids)
    }
}

fn tokenise(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|t| t.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn course(name: &str) -> Course {
        Course::new(name)
    }

    #[test]
    fn index_and_exact_search() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        let c = course("Introduction to Computer Science");
        eng.index_course(&c).unwrap();
        let hits = eng.search("Computer Science", 10).unwrap();
        assert_eq!(hits, vec![c.id.to_string()]);
    }

    #[test]
    fn fuzzy_search_tolerates_typo() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        let c = course("Algorithms");
        eng.index_course(&c).unwrap();
        let hits = eng.fuzzy_search("Algoritms", 10).unwrap();
        assert_eq!(hits, vec![c.id.to_string()]);
    }

    #[test]
    fn blocking_query_filters_by_provider() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        let p1 = uuid::Uuid::new_v4();
        let p2 = uuid::Uuid::new_v4();

        let mut a = course("Linear Algebra");
        a.provider_id = Some(p1);
        let mut b = course("Linear Algebra");
        b.provider_id = Some(p2);

        eng.index_course(&a).unwrap();
        eng.index_course(&b).unwrap();

        let hits = eng
            .search_by_name_and_provider("Linear Algebra", Some(p1), 10)
            .unwrap();
        assert_eq!(hits, vec![a.id.to_string()]);
    }

    #[test]
    fn delete_removes_from_index() {
        let dir = TempDir::new().unwrap();
        let eng = SearchEngine::new(dir.path()).unwrap();
        let c = course("Discrete Math");
        eng.index_course(&c).unwrap();
        assert_eq!(eng.stats().unwrap().num_docs, 1);
        eng.delete_course(&c.id.to_string()).unwrap();
        assert_eq!(eng.stats().unwrap().num_docs, 0);
    }

    #[test]
    fn tokenise_handles_underscores_and_punctuation() {
        assert_eq!(tokenise("CS101_intro"), vec!["cs101", "intro"]);
        assert_eq!(tokenise("   "), Vec::<String>::new());
    }
}
