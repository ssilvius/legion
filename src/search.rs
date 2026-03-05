use std::path::Path;

use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, QueryParser, TermQuery};
use tantivy::schema::{
    Field, IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing, TextOptions, Value,
};
use tantivy::{Index, IndexWriter, ReloadPolicy, TantivyDocument, Term, doc};

use crate::error::{LegionError, Result};

/// Full-text search index backed by Tantivy with BM25 scoring.
///
/// Indexes reflection text for retrieval by keyword similarity.
/// Documents are filtered by repo (exact match) and ranked by
/// BM25 score on the text field (tokenized, stemmed).
pub struct SearchIndex {
    index: Index,
    id_field: Field,
    repo_field: Field,
    text_field: Field,
}

/// A single search result with its document ID and BM25 relevance score.
pub struct SearchResult {
    pub id: String,
    pub score: f32,
}

impl SearchIndex {
    /// Open or create a Tantivy index at the given directory path.
    ///
    /// Schema fields:
    /// - `id`: STRING | STORED -- exact match, retrievable after search
    /// - `repo`: STRING -- exact match filtering per repository
    /// - `text`: TEXT -- tokenized with English stemmer, BM25 scored
    pub fn open(path: &Path) -> Result<Self> {
        let mut schema_builder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let repo_field = schema_builder.add_text_field("repo", STRING);

        let text_options = TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("en_stem")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        );
        let text_field = schema_builder.add_text_field("text", text_options);

        let schema = schema_builder.build();

        let index = if path.join("meta.json").exists() {
            Index::open_in_dir(path).map_err(|e| LegionError::Search(e.to_string()))?
        } else {
            std::fs::create_dir_all(path).map_err(|e| LegionError::Search(e.to_string()))?;
            Index::create_in_dir(path, schema.clone())
                .map_err(|e| LegionError::Search(e.to_string()))?
        };

        Ok(Self {
            index,
            id_field,
            repo_field,
            text_field,
        })
    }

    /// Add a document to the search index and commit immediately.
    ///
    /// Each document consists of an id (stored for retrieval), a repo name
    /// (for filtering), and the reflection text (for BM25 scoring).
    ///
    /// Commits after each write. The reflection corpus is tiny, so the
    /// per-write commit overhead is negligible.
    pub fn add(&self, id: &str, repo: &str, text: &str) -> Result<()> {
        let mut writer: IndexWriter = self
            .index
            .writer(15_000_000)
            .map_err(|e| LegionError::Search(e.to_string()))?;

        writer
            .add_document(doc!(
                self.id_field => id,
                self.repo_field => repo,
                self.text_field => text,
            ))
            .map_err(|e| LegionError::Search(e.to_string()))?;

        writer
            .commit()
            .map_err(|e| LegionError::Search(e.to_string()))?;

        Ok(())
    }

    /// Search for reflections matching a query within a specific repo.
    ///
    /// Combines an exact-match filter on `repo` with a BM25-scored query
    /// on the `text` field. Returns up to `limit` results ordered by
    /// descending relevance score.
    ///
    /// Returns an empty vec if the query string is empty or whitespace-only.
    pub fn search(&self, repo: &str, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .map_err(|e: tantivy::TantivyError| LegionError::Search(e.to_string()))?;

        let searcher = reader.searcher();

        // Parse the user query against the text field (uses en_stem tokenizer)
        let query_parser = QueryParser::for_index(&self.index, vec![self.text_field]);
        let text_query = query_parser
            .parse_query(trimmed)
            .map_err(|e| LegionError::Search(e.to_string()))?;

        // Build exact-match filter on repo field
        let repo_term = Term::from_field_text(self.repo_field, repo);
        let repo_query = TermQuery::new(repo_term, IndexRecordOption::Basic);

        // Combine: must match repo AND must match text query
        let combined = BooleanQuery::new(vec![
            (Occur::Must, Box::new(repo_query)),
            (Occur::Must, text_query),
        ]);

        let top_docs = searcher
            .search(&combined, &TopDocs::with_limit(limit))
            .map_err(|e| LegionError::Search(e.to_string()))?;

        let mut results: Vec<SearchResult> = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| LegionError::Search(e.to_string()))?;

            let id = retrieved_doc
                .get_first(self.id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            results.push(SearchResult { id, score });
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a SearchIndex backed by a temporary directory.
    ///
    /// Returns both the index and the TempDir handle. The TempDir must
    /// outlive the index to keep the mmap-backed files accessible.
    fn test_index() -> (SearchIndex, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let idx = SearchIndex::open(dir.path()).expect("failed to open index");
        (idx, dir)
    }

    #[test]
    fn add_and_search() {
        let (idx, _dir) = test_index();
        idx.add(
            "id-1",
            "kelex",
            "mapping rules are fragile when adding new Zod types",
        )
        .unwrap();
        idx.add(
            "id-2",
            "kelex",
            "discriminated unions inside arrays are where complexity hides",
        )
        .unwrap();
        idx.add("id-3", "kelex", "the CLI flag parser is straightforward")
            .unwrap();
        let results = idx.search("kelex", "Zod type mapping", 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "id-1");
    }

    #[test]
    fn search_filters_by_repo() {
        let (idx, _dir) = test_index();
        idx.add("id-1", "kelex", "schema introspection is complex")
            .unwrap();
        idx.add("id-2", "rafters", "schema tokens need attention")
            .unwrap();
        let results = idx.search("kelex", "schema", 5).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "id-1");
    }

    #[test]
    fn search_respects_limit() {
        let (idx, _dir) = test_index();
        for i in 0..10 {
            idx.add(
                &format!("id-{i}"),
                "test",
                &format!("reflection about testing {i}"),
            )
            .unwrap();
        }
        let results = idx.search("test", "testing", 3).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn search_empty_query_returns_empty() {
        let (idx, _dir) = test_index();
        idx.add("id-1", "kelex", "some reflection").unwrap();
        let results = idx.search("kelex", "", 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn stemming_matches_variants() {
        let (idx, _dir) = test_index();
        idx.add("id-1", "test", "nested arrays in the codegen templates")
            .unwrap();
        let results = idx.search("test", "nesting array codegen", 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "id-1");
    }
}
