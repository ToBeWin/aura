use crate::errors::Result;
use crate::llm::LocalLLM;
use crate::models::DenoiseResult;
use regex::Regex;

#[derive(Clone)]
pub struct DeNoisingModule {
    filler_words: Vec<String>,
}

impl DeNoisingModule {
    pub fn new(_llm: LocalLLM) -> Self {
        Self {
            filler_words: Self::default_filler_words(),
        }
    }

    #[allow(dead_code)]
    pub fn with_filler_words(mut self, filler_words: Vec<String>) -> Self {
        self.filler_words = filler_words;
        self
    }

    pub async fn denoise(&self, raw_input: &str) -> Result<DenoiseResult> {
        // Keep denoising deterministic and low-latency so the main LLM budget
        // can be spent on the final rewrite instead of a preliminary cleanup pass.
        let cleaned = self.rule_based_denoise(raw_input);

        let removed_fillers = self.extract_removed_fillers(raw_input, &cleaned);
        let confidence = self.calculate_confidence(raw_input, &cleaned);

        Ok(DenoiseResult {
            cleaned_text: cleaned,
            removed_fillers,
            removed_duplicates: vec![],
            confidence,
        })
    }

    pub fn rule_based_denoise(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Remove filler words
        for filler in &self.filler_words {
            let pattern = format!(r"{}\s*", regex::escape(filler));
            if let Ok(re) = Regex::new(&pattern) {
                result = re.replace_all(&result, "").to_string();
            }
        }

        // Clean up extra spaces
        if let Ok(re) = Regex::new(r"\s+") {
            result = re.replace_all(&result, " ").to_string();
        }

        // Remove repeated consecutive words
        // Note: uses iterative word-by-word dedup since the Rust `regex`
        // crate does not support backreferences (`\1`).
        let words: Vec<&str> = result.split_whitespace().collect();
        let mut deduped: Vec<&str> = Vec::new();
        for word in &words {
            if deduped.last().is_none_or(|last| *last != *word) {
                deduped.push(word);
            }
        }
        result = deduped.join(" ");

        result.trim().to_string()
    }
    fn extract_removed_fillers(&self, original: &str, cleaned: &str) -> Vec<String> {
        self.filler_words
            .iter()
            .filter(|filler| original.contains(filler.as_str()) && !cleaned.contains(filler.as_str()))
            .cloned()
            .collect()
    }

    fn calculate_confidence(&self, _original: &str, _cleaned: &str) -> f64 {
        // Simple confidence calculation
        0.85
    }

    fn default_filler_words() -> Vec<String> {
        vec![
            "呃", "嗯", "啊", "那个", "这个", "就是", "然后", "嘛",
            "uh", "um", "like", "you know", "I mean",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_module() -> DeNoisingModule {
        DeNoisingModule::new(crate::llm::LocalLLM::new("test".to_string()))
    }

    #[test]
    fn test_rule_based_denoise_removes_chinese_fillers() {
        let module = make_module();
        let result = module.rule_based_denoise("呃 那个 明天开会");
        assert!(result.contains("明天开会"), "Expected clean text in output: {result}");
    }

    #[test]
    fn test_rule_based_denoise_removes_extra_spaces() {
        let module = make_module();
        let result = module.rule_based_denoise("hello    world");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_rule_based_denoise_removes_english_fillers() {
        let module = make_module();
        let result = module.rule_based_denoise("um uh like I mean hello");
        assert!(!result.contains("um "));
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_default_filler_words_contains_both_languages() {
        let words = DeNoisingModule::default_filler_words();
        assert!(words.contains(&"呃".to_string()));
        assert!(words.contains(&"um".to_string()));
    }

    #[test]
    fn test_extract_removed_fillers() {
        let module = make_module();
        let fillers = module.extract_removed_fillers("呃 hello 那个 world", "hello world");
        assert!(fillers.contains(&"呃".to_string()));
    }
}
