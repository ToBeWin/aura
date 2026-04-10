use crate::errors::Result;
use crate::llm::LocalLLM;
use crate::models::CorrectionRecord;
use crate::storage::LocalVectorDB;
use std::collections::HashMap;

pub struct CorrectionManager {
    llm: LocalLLM,
    vector_db: LocalVectorDB,
}

impl CorrectionManager {
    pub fn new(llm: LocalLLM, vector_db: LocalVectorDB) -> Self {
        Self { llm, vector_db }
    }

    /// Save a user correction to the database
    pub async fn save_correction(
        &self,
        user_id: &str,
        original_text: &str,
        corrected_text: &str,
        context: HashMap<String, String>,
    ) -> Result<CorrectionRecord> {
        // Extract correction pattern
        let pattern = self.extract_pattern(original_text, corrected_text).await?;
        
        // Generate embedding for similarity search
        let embedding = self.llm.embed(original_text).await?;
        
        let record = CorrectionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            original_text: original_text.to_string(),
            corrected_text: corrected_text.to_string(),
            correction_type: pattern.correction_type.clone(),
            pattern: pattern.pattern,
            replacement: pattern.replacement,
            context,
            embedding,
            timestamp: chrono::Utc::now(),
            applied_count: 0,
        };

        // Save to vector database
        let mut data = HashMap::new();
        data.insert("id".to_string(), serde_json::Value::String(record.id.clone()));
        data.insert("user_id".to_string(), serde_json::Value::String(record.user_id.clone()));
        data.insert("original_text".to_string(), serde_json::Value::String(record.original_text.clone()));
        data.insert("corrected_text".to_string(), serde_json::Value::String(record.corrected_text.clone()));
        data.insert("correction_type".to_string(), serde_json::Value::String(record.correction_type.clone()));
        data.insert("pattern".to_string(), serde_json::Value::String(record.pattern.clone()));
        data.insert("replacement".to_string(), serde_json::Value::String(record.replacement.clone()));
        data.insert("timestamp".to_string(), serde_json::Value::String(record.timestamp.to_rfc3339()));
        
        self.vector_db.insert("corrections", vec![data])?;

        log::info!("Saved correction record: {} -> {}", original_text, corrected_text);
        
        Ok(record)
    }

    /// Extract correction pattern from original and corrected text
    async fn extract_pattern(&self, original: &str, corrected: &str) -> Result<CorrectionPattern> {
        // Use LLM to analyze the difference
        let prompt = format!(
            "Analyze the difference between these two texts and extract the correction pattern:\n\n\
             Original: {}\n\
             Corrected: {}\n\n\
             Identify:\n\
             1. Correction type (tone/format/content/terminology)\n\
             2. Pattern (what was changed)\n\
             3. Replacement (what it was changed to)\n\n\
             Respond in JSON format:\n\
             {{\"correction_type\": \"...\", \"pattern\": \"...\", \"replacement\": \"...\"}}"
            , original, corrected
        );

        let response = self.llm.generate(&prompt, None, Some(512), 0.3).await?;
        
        // Parse JSON response
        let pattern: CorrectionPattern = serde_json::from_str(&response)
            .unwrap_or_else(|_| CorrectionPattern {
                correction_type: "content".to_string(),
                pattern: original.to_string(),
                replacement: corrected.to_string(),
            });

        Ok(pattern)
    }

    /// Retrieve similar corrections from history
    pub async fn retrieve_corrections(
        &self,
        user_id: &str,
        query_text: &str,
        limit: usize,
    ) -> Result<Vec<CorrectionRecord>> {
        // Generate query embedding
        let query_embedding = self.llm.embed(query_text).await?;
        
        // Search in vector database
        let mut filter = HashMap::new();
        filter.insert("user_id".to_string(), user_id.to_string());
        
        let results = self.vector_db.search(
            "corrections",
            query_embedding,
            Some(filter),
            limit,
        )?;

        // Convert search results to CorrectionRecords
        let records: Vec<CorrectionRecord> = results
            .into_iter()
            .filter_map(|result| {
                let id = result.metadata.get("id").cloned()?;
                let user_id = result.metadata.get("user_id").cloned()?;
                let original_text = result.metadata.get("original_text").cloned()?;
                let corrected_text = result.metadata.get("corrected_text").cloned()?;
                let correction_type = result.metadata.get("correction_type").cloned()?;
                let pattern = result.metadata.get("pattern").cloned()?;
                let replacement = result.metadata.get("replacement").cloned()?;
                let applied_count = result.metadata.get("applied_count")
                    .and_then(|v| v.parse::<i32>().ok())
                    .unwrap_or(0);
                let embedding = serde_json::from_str(&result.metadata.get("embedding").cloned().unwrap_or_default())
                    .unwrap_or_default();
                let context = serde_json::from_str(&result.metadata.get("context").cloned().unwrap_or_else(|| "{}".to_string()))
                    .unwrap_or_default();
                let timestamp = result.metadata.get("timestamp")
                    .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(chrono::Utc::now);

                Some(CorrectionRecord {
                    id,
                    user_id,
                    original_text,
                    corrected_text,
                    correction_type,
                    pattern,
                    replacement,
                    context,
                    embedding,
                    timestamp,
                    applied_count,
                })
            })
            .collect();

        Ok(records)
    }

    /// Apply correction rules to text
    #[allow(dead_code)]
    pub fn apply_corrections(
        &self,
        text: &str,
        corrections: &[CorrectionRecord],
    ) -> String {
        let mut result = text.to_string();
        
        for correction in corrections {
            if result.contains(&correction.pattern) {
                result = result.replace(&correction.pattern, &correction.replacement);
                log::debug!("Applied correction: {} -> {}", correction.pattern, correction.replacement);
            }
        }
        
        result
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CorrectionPattern {
    correction_type: String,
    pattern: String,
    replacement: String,
}
