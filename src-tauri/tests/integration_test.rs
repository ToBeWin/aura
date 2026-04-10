// Integration tests for Aura core functionality

#[cfg(test)]
mod tests {
    use aura_lib::storage::UserContextStore;
    use std::collections::HashMap;

    #[test]
    fn test_user_context_lifecycle() {
        let db_path = "/tmp/aura_integration_context.db";
        let _ = std::fs::remove_file(db_path);

        let store = UserContextStore::new(db_path).unwrap();

        // Create context with all fields populated
        use aura_lib::models::UserContext;
        let mut name_mappings = HashMap::new();
        name_mappings.insert("小李".to_string(), "李经理".to_string());
        name_mappings.insert("苏".to_string(), "苏总".to_string());
        let mut locations = HashMap::new();
        locations.insert("家".to_string(), "上海市浦东新区".to_string());
        locations.insert("公司".to_string(), "北京市海淀区".to_string());
        let mut terms = HashMap::new();
        terms.insert("PPT".to_string(), "演示文稿".to_string());

        let ctx = UserContext {
            user_id: "integration_test".to_string(),
            name_mappings,
            location_preferences: locations,
            terminology: terms,
            forbidden_words: vec!["敏感词".to_string(), "禁用词".to_string()],
            default_tone: "formal".to_string(),
            default_format: Some("email".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Save and reload
        store.save_user_context(&ctx).unwrap();
        let loaded = store.get_context("integration_test").unwrap();

        assert_eq!(loaded.name_mappings.get("小李").unwrap(), "李经理");
        assert_eq!(loaded.name_mappings.get("苏").unwrap(), "苏总");
        assert_eq!(loaded.location_preferences.get("家").unwrap(), "上海市浦东新区");
        assert_eq!(loaded.terminology.get("PPT").unwrap(), "演示文稿");
        assert_eq!(loaded.forbidden_words, vec!["敏感词", "禁用词"]);
        assert_eq!(loaded.default_tone, "formal");
        assert_eq!(loaded.default_format, Some("email".to_string()));
    }

    #[test]
    fn test_user_context_update_preserves_others() {
        let db_path = "/tmp/aura_integration_update.db";
        let _ = std::fs::remove_file(db_path);
        let store = UserContextStore::new(db_path).unwrap();

        // First save name mappings
        store.update_context("user1", "name_mappings", r#"{"张三":"张先生"}"#).unwrap();
        // Then update terminology
        store.update_context("user1", "terminology", r#"{"bug":"缺陷"}"#).unwrap();

        let ctx = store.get_context("user1").unwrap();
        assert_eq!(ctx.name_mappings.get("张三").unwrap(), "张先生");
        assert_eq!(ctx.terminology.get("bug").unwrap(), "缺陷");
    }

    #[test]
    fn test_denose_module_rule_based_only() {
        // Test the rule-based denoising module in isolation
        // (without requiring a real LLM connection)
        let llm = aura_lib::llm::LocalLLM::new("test".to_string());
        let denoise = aura_lib::processing::DeNoisingModule::new(llm);

        // Chinese filler words
        let result = denoise.rule_based_denoise("呃 那个 明天开会 就是 然后");
        assert!(result.contains("明天开会"));

        // English filler words
        let result2 = denoise.rule_based_denoise("um uh I mean hello like");
        assert!(result2.contains("hello"));

        // Duplicate words
        let result3 = denoise.rule_based_denoise("hello hello world world");
        assert_eq!(result3, "hello world");

        // Extra whitespace
        let result4 = denoise.rule_based_denoise("hello    world");
        assert_eq!(result4, "hello world");

        // Chinese sentence
        let result5 = denoise.rule_based_denoise("呃，小李啊，那个...明天上海那个会");
        assert!(result5.contains("小李"));
        assert!(result5.contains("明天上海"));
    }

    #[test]
    fn test_vector_db_full_lifecycle() {
        use aura_lib::storage::LocalVectorDB;

        let db_path = "/tmp/aura_integration_vectors.db";
        let _ = std::fs::remove_file(db_path);
        let db = LocalVectorDB::new(db_path.to_string()).unwrap();

        // Insert a correction record
        let mut data1 = HashMap::new();
        data1.insert("id".to_string(), serde_json::Value::String("c1".to_string()));
        data1.insert("user_id".to_string(), serde_json::Value::String("u1".to_string()));
        data1.insert("original_text".to_string(), serde_json::Value::String("呃 明天开会".to_string()));
        data1.insert("corrected_text".to_string(), serde_json::Value::String("明天开会".to_string()));
        data1.insert("correction_type".to_string(), serde_json::Value::String("denoise".to_string()));
        data1.insert("pattern".to_string(), serde_json::Value::String("呃".to_string()));
        data1.insert("replacement".to_string(), serde_json::Value::String("".to_string()));
        data1.insert("timestamp".to_string(), serde_json::Value::String("2026-04-01T00:00:00Z".to_string()));
        data1.insert("embedding".to_string(), serde_json::json!([1.0, 0.0, 0.0]));

        // Insert another record with different embedding
        let mut data2 = HashMap::new();
        data2.insert("id".to_string(), serde_json::Value::String("c2".to_string()));
        data2.insert("user_id".to_string(), serde_json::Value::String("u1".to_string()));
        data2.insert("original_text".to_string(), serde_json::Value::String("那个 周报写一下".to_string()));
        data2.insert("corrected_text".to_string(), serde_json::Value::String("请撰写本周工作周报".to_string()));
        data2.insert("correction_type".to_string(), serde_json::Value::String("format".to_string()));
        data2.insert("pattern".to_string(), serde_json::Value::String("写一下".to_string()));
        data2.insert("replacement".to_string(), serde_json::Value::String("请撰写".to_string()));
        data2.insert("timestamp".to_string(), serde_json::Value::String("2026-04-01T00:00:00Z".to_string()));
        data2.insert("embedding".to_string(), serde_json::json!([0.0, 1.0, 0.0]));

        db.insert("corrections", vec![data1, data2]).unwrap();

        // Search for similar corrections
        let results = db.search("corrections", vec![1.0, 0.0, 0.0], None, 10).unwrap();
        assert_eq!(results.len(), 2);
        // First result should be the most similar one
        assert_eq!(results[0].metadata.get("id").unwrap(), "c1");
    }

    #[test]
    fn test_resource_monitor_model_selection() {
        use aura_lib::monitoring::ResourceMonitor;

        let monitor = ResourceMonitor::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let status = rt.block_on(monitor.check_resources());

        // Just verify it runs and returns valid data
        assert!(status.available_memory_mb > 0);
        assert!(status.cpu_usage >= 0.0);
        assert!(status.cpu_usage <= 100.0);
    }

    #[tokio::test]
    async fn test_asr_transcribe_nonexistent_file() {
        use aura_lib::asr::ASREngine;
        use aura_lib::errors::AuraError;

        let asr = ASREngine::new("test".to_string(), "zh-CN".to_string());
        let result = asr.transcribe("/nonexistent/file.wav").await;

        assert!(result.is_err());
        if let Err(AuraError::InputValidation { error_code, .. }) = result {
            assert_eq!(error_code, "ASR_001");
        } else {
            panic!("Expected InputValidation error");
        }
    }
}
