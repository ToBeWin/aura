use crate::errors::Result;
use rusqlite::{Connection, params};
use std::collections::HashMap;

// SQLite-based vector storage with cosine similarity search
// Used for correction history retrieval in the personalization pipeline
pub struct LocalVectorDB {
    db_path: String,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub similarity: f32,
    pub metadata: HashMap<String, String>,
}

/// Intermediate struct for row mapping before computing similarity
pub struct CorrectionRaw {
    pub metadata: HashMap<String, String>,
    pub embedding_vec: Vec<f32>,
}

impl CorrectionRaw {
    pub fn into_search_result(self, similarity: f32) -> SearchResult {
        SearchResult {
            similarity,
            metadata: self.metadata,
        }
    }
}

impl LocalVectorDB {
    pub fn new(db_path: String) -> Result<Self> {
        let db = Self { db_path };
        db.init_tables()?;
        Ok(db)
    }

    fn conn(&self) -> Result<Connection> {
        Ok(Connection::open(&self.db_path)?)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS correction_vectors (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                original_text TEXT NOT NULL,
                corrected_text TEXT NOT NULL,
                correction_type TEXT NOT NULL,
                pattern TEXT NOT NULL,
                replacement TEXT NOT NULL,
                context TEXT NOT NULL DEFAULT '{}',
                timestamp TEXT NOT NULL,
                applied_count INTEGER NOT NULL DEFAULT 0,
                embedding TEXT NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_correction_user_id ON correction_vectors(user_id)",
            [],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn create_collection(&self, _name: &str, _schema: HashMap<String, String>) -> Result<()> {
        // Table already created in init_tables
        Ok(())
    }

    pub fn insert(&self, _collection: &str, data: Vec<HashMap<String, serde_json::Value>>) -> Result<()> {
        let conn = self.conn()?;
        for row in data {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let user_id = row.get("user_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let original_text = row.get("original_text").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let corrected_text = row.get("corrected_text").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let correction_type = row.get("correction_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let pattern = row.get("pattern").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let replacement = row.get("replacement").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let timestamp = row.get("timestamp").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let embedding_json = row.get("embedding").cloned().unwrap_or(serde_json::json!([]));
            let embedding_str = serde_json::to_string(&embedding_json)?;

            conn.execute(
                "INSERT OR REPLACE INTO correction_vectors
                 (id, user_id, original_text, corrected_text, correction_type,
                  pattern, replacement, context, timestamp, applied_count, embedding)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '{}', ?8, 0, ?9)",
                params![id, user_id, original_text, corrected_text, correction_type,
                        pattern, replacement, timestamp, embedding_str],
            )?;
        }
        Ok(())
    }

    pub fn search(
        &self,
        _collection: &str,
        query_vector: Vec<f32>,
        filter: Option<HashMap<String, String>>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let conn = self.conn()?;
        let limit_val = limit as i64;

        let raw_results: Vec<CorrectionRaw> = match filter.as_ref().and_then(|f| f.get("user_id")).cloned() {
            Some(user_id) => {
                let sql = "SELECT id, user_id, original_text, corrected_text, correction_type,
                             pattern, replacement, context, timestamp, applied_count, embedding
                         FROM correction_vectors
                         WHERE user_id = ?1
                         LIMIT ?2";
                let mut stmt = conn.prepare(sql)?;
                self.query_rows(&mut stmt, params![user_id, limit_val])?
            }
            None => {
                let sql = "SELECT id, user_id, original_text, corrected_text, correction_type,
                             pattern, replacement, context, timestamp, applied_count, embedding
                         FROM correction_vectors
                         LIMIT ?1";
                let mut stmt = conn.prepare(sql)?;
                self.query_rows(&mut stmt, params![limit_val])?
            }
        };

        let mut results: Vec<SearchResult> = Vec::new();
        for row in raw_results {
            let similarity = cosine_similarity(&query_vector, &row.embedding_vec);
            results.push(row.into_search_result(similarity));
        }

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        Ok(results)
    }

    fn query_rows<'a>(
        &self,
        stmt: &'a mut rusqlite::Statement,
        params: impl rusqlite::Params,
    ) -> Result<Vec<CorrectionRaw>> {
        let rows = stmt.query_map(params, |row| self.map_row(row))?;
        let mut results = Vec::new();
        for row in rows.flatten() {
            results.push(row);
        }
        Ok(results)
    }

    fn map_row<'a>(
        &self,
        row: &rusqlite::Row<'a>,
    ) -> rusqlite::Result<CorrectionRaw> {
        let id: String = row.get(0)?;
        let user_id: String = row.get(1)?;
        let original_text: String = row.get(2)?;
        let corrected_text: String = row.get(3)?;
        let correction_type: String = row.get(4)?;
        let pattern: String = row.get(5)?;
        let replacement: String = row.get(6)?;
        let context: String = row.get(7)?;
        let timestamp: String = row.get(8)?;
        let applied_count: i32 = row.get(9)?;
        let embedding_str: String = row.get(10)?;

        let embedding_vec: Vec<f32> = serde_json::from_str(&embedding_str).unwrap_or_default();

        let mut metadata = HashMap::new();
        metadata.insert("id".to_string(), id);
        metadata.insert("user_id".to_string(), user_id);
        metadata.insert("original_text".to_string(), original_text);
        metadata.insert("corrected_text".to_string(), corrected_text);
        metadata.insert("correction_type".to_string(), correction_type);
        metadata.insert("pattern".to_string(), pattern);
        metadata.insert("replacement".to_string(), replacement);
        metadata.insert("context".to_string(), context);
        metadata.insert("timestamp".to_string(), timestamp);
        metadata.insert("applied_count".to_string(), applied_count.to_string());
        metadata.insert("embedding".to_string(), embedding_str);

        Ok(CorrectionRaw {
            metadata,
            embedding_vec,
        })
    }

    #[allow(dead_code)]
    pub fn delete(&self, _collection: &str, filter: HashMap<String, String>) -> Result<()> {
        let conn = self.conn()?;
        if let Some(id) = filter.get("id") {
            conn.execute("DELETE FROM correction_vectors WHERE id = ?", params![id])?;
        } else if let Some(user_id) = filter.get("user_id") {
            conn.execute("DELETE FROM correction_vectors WHERE user_id = ?", params![user_id])?;
        }
        Ok(())
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator == 0.0 {
        0.0
    } else {
        dot / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![-1.0, 0.0];
        let b = vec![1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_mismatch_length() {
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 0.0]), 0.0);
    }

    #[test]
    fn test_vector_db_insert_and_search() {
        let db_path = "/tmp/aura_test_vectors.db";
        let _ = std::fs::remove_file(db_path);

        let db = LocalVectorDB::new(db_path.to_string()).unwrap();
        db.create_collection("corrections", HashMap::new()).unwrap();

        let mut data = HashMap::new();
        data.insert("id".to_string(), serde_json::Value::String("test1".to_string()));
        data.insert("user_id".to_string(), serde_json::Value::String("user1".to_string()));
        data.insert("original_text".to_string(), serde_json::Value::String("hello world".to_string()));
        data.insert("corrected_text".to_string(), serde_json::Value::String("Hello, World!".to_string()));
        data.insert("correction_type".to_string(), serde_json::Value::String("format".to_string()));
        data.insert("pattern".to_string(), serde_json::Value::String("hello".to_string()));
        data.insert("replacement".to_string(), serde_json::Value::String("Hello".to_string()));
        data.insert("timestamp".to_string(), serde_json::Value::String("2026-04-01T00:00:00Z".to_string()));
        data.insert("embedding".to_string(), serde_json::json!([1.0, 0.5, 0.3]));

        db.insert("corrections", vec![data]).unwrap();

        let results = db.search("corrections", vec![1.0, 0.5, 0.3], None, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!("hello".eq(results[0].metadata.get("pattern").unwrap()));
    }

    #[test]
    fn test_vector_db_search_with_user_filter() {
        let db_path = "/tmp/aura_test_filter.db";
        let _ = std::fs::remove_file(db_path);
        let db = LocalVectorDB::new(db_path.to_string()).unwrap();
        db.create_collection("corrections", HashMap::new()).unwrap();

        // Insert two rows for different users
        for (uid, pid) in [("user_a", "a1"), ("user_b", "b1")] {
            let mut data = HashMap::new();
            data.insert("id".to_string(), serde_json::Value::String(pid.to_string()));
            data.insert("user_id".to_string(), serde_json::Value::String(uid.to_string()));
            data.insert("original_text".to_string(), serde_json::Value::String("text".to_string()));
            data.insert("corrected_text".to_string(), serde_json::Value::String("fixed".to_string()));
            data.insert("correction_type".to_string(), serde_json::Value::String("tone".to_string()));
            data.insert("pattern".to_string(), serde_json::Value::String("x".to_string()));
            data.insert("replacement".to_string(), serde_json::Value::String("y".to_string()));
            data.insert("timestamp".to_string(), serde_json::Value::String("2026-04-01T00:00:00Z".to_string()));
            data.insert("embedding".to_string(), serde_json::json!([0.5, 0.5]));
            db.insert("corrections", vec![data]).unwrap();
        }

        let filter = HashMap::from([("user_id".to_string(), "user_a".to_string())]);
        let results = db.search("corrections", vec![0.5, 0.5], Some(filter), 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.get("user_id").unwrap(), "user_a");
    }

    #[test]
    fn test_vector_db_delete() {
        let db_path = "/tmp/aura_test_delete.db";
        let _ = std::fs::remove_file(db_path);
        let db = LocalVectorDB::new(db_path.to_string()).unwrap();
        db.create_collection("corrections", HashMap::new()).unwrap();

        let mut data = HashMap::new();
        data.insert("id".to_string(), serde_json::Value::String("to_delete".to_string()));
        data.insert("user_id".to_string(), serde_json::Value::String("user1".to_string()));
        data.insert("original_text".to_string(), serde_json::Value::String("text".to_string()));
        data.insert("corrected_text".to_string(), serde_json::Value::String("fixed".to_string()));
        data.insert("correction_type".to_string(), serde_json::Value::String("tone".to_string()));
        data.insert("pattern".to_string(), serde_json::Value::String("x".to_string()));
        data.insert("replacement".to_string(), serde_json::Value::String("y".to_string()));
        data.insert("timestamp".to_string(), serde_json::Value::String("2026-04-01T00:00:00Z".to_string()));
        data.insert("embedding".to_string(), serde_json::json!([1.0]));
        db.insert("corrections", vec![data.clone()]).unwrap();

        let filter = HashMap::from([("id".to_string(), "to_delete".to_string())]);
        db.delete("corrections", filter).unwrap();

        let results = db.search("corrections", vec![1.0], None, 10).unwrap();
        assert_eq!(results.len(), 0);
    }
}
