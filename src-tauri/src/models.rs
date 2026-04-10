use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderMode {
    Local,
    Cloud,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LLMCloudProvider {
    OpenAI,
    Anthropic,
    Gemini,
    DeepSeek,
    Qwen,
    Glm,
    Kimi,
    Minimax,
    OpenRouter,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ASRCloudProvider {
    OpenAI,
    Groq,
    Deepgram,
    AssemblyAI,
    Azure,
    Google,
    Custom,
}

fn default_provider_mode() -> ProviderMode {
    ProviderMode::Local
}

fn default_cloud_api_base() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_llm_cloud_provider() -> LLMCloudProvider {
    LLMCloudProvider::OpenAI
}

fn default_asr_cloud_provider() -> ASRCloudProvider {
    ASRCloudProvider::OpenAI
}

fn default_asr_local_model() -> String {
    "whisper-base".to_string()
}

fn default_llm_local_model() -> String {
    "qwen3.5:2b".to_string()
}

fn default_llm_local_base_url() -> String {
    "http://localhost:11434".to_string()
}

fn default_asr_cloud_model() -> String {
    "gpt-4o-mini-transcribe".to_string()
}

fn default_llm_cloud_model() -> String {
    "gpt-4.1-mini".to_string()
}

fn default_llm_cloud_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

fn default_language_hint() -> String {
    "auto".to_string()
}

fn default_ui_locale() -> String {
    "en".to_string()
}

fn default_wake_word_enabled() -> bool {
    false
}

fn default_wake_word_phrase() -> String {
    "Aura".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ASRProviderSettings {
    #[serde(default = "default_provider_mode")]
    pub provider: ProviderMode,
    #[serde(default = "default_asr_local_model")]
    pub local_model: String,
    #[serde(default = "default_asr_cloud_provider")]
    pub cloud_provider: ASRCloudProvider,
    #[serde(default = "default_cloud_api_base")]
    pub cloud_base_url: String,
    #[serde(default)]
    pub cloud_api_key: String,
    #[serde(default = "default_asr_cloud_model")]
    pub cloud_model: String,
    #[serde(default = "default_language_hint")]
    pub language: String,
}

impl Default for ASRProviderSettings {
    fn default() -> Self {
        Self {
            provider: default_provider_mode(),
            local_model: default_asr_local_model(),
            cloud_provider: default_asr_cloud_provider(),
            cloud_base_url: default_cloud_api_base(),
            cloud_api_key: String::new(),
            cloud_model: default_asr_cloud_model(),
            language: default_language_hint(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LLMProviderSettings {
    #[serde(default = "default_provider_mode")]
    pub provider: ProviderMode,
    #[serde(default = "default_llm_local_model")]
    pub local_model: String,
    #[serde(default = "default_llm_local_base_url")]
    pub local_base_url: String,
    #[serde(default = "default_llm_cloud_provider")]
    pub cloud_provider: LLMCloudProvider,
    #[serde(default = "default_cloud_api_base")]
    pub cloud_base_url: String,
    #[serde(default)]
    pub cloud_api_key: String,
    #[serde(default = "default_llm_cloud_model")]
    pub cloud_model: String,
    #[serde(default = "default_llm_cloud_embedding_model")]
    pub cloud_embedding_model: String,
}

impl Default for LLMProviderSettings {
    fn default() -> Self {
        Self {
            provider: default_provider_mode(),
            local_model: default_llm_local_model(),
            local_base_url: default_llm_local_base_url(),
            cloud_provider: default_llm_cloud_provider(),
            cloud_base_url: default_cloud_api_base(),
            cloud_api_key: String::new(),
            cloud_model: default_llm_cloud_model(),
            cloud_embedding_model: default_llm_cloud_embedding_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettings {
    #[serde(default)]
    pub asr: ASRProviderSettings,
    #[serde(default)]
    pub llm: LLMProviderSettings,
    #[serde(default = "default_ui_locale")]
    pub locale: String,
    #[serde(default = "default_wake_word_enabled")]
    pub wake_word_enabled: bool,
    #[serde(default = "default_wake_word_phrase")]
    pub wake_word_phrase: String,
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            asr: ASRProviderSettings::default(),
            llm: LLMProviderSettings::default(),
            locale: default_ui_locale(),
            wake_word_enabled: default_wake_word_enabled(),
            wake_word_phrase: default_wake_word_phrase(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct RawInput {
    pub text: String,
    pub language: String,
    #[serde(default)]
    pub audio_duration: f64,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub segments: Vec<Segment>,
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[allow(dead_code)]
fn default_confidence() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[allow(dead_code)]
impl RawInput {
    pub fn validate(&self) -> Result<(), String> {
        if self.text.is_empty() {
            return Err("Text is empty".to_string());
        }
        if self.text.len() > 10000 {
            return Err("Text exceeds 10000 characters".to_string());
        }
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err("Confidence must be between 0.0 and 1.0".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: String,
    #[serde(default)]
    pub name_mappings: HashMap<String, String>,
    #[serde(default)]
    pub location_preferences: HashMap<String, String>,
    #[serde(default)]
    pub terminology: HashMap<String, String>,
    #[serde(default)]
    pub forbidden_words: Vec<String>,
    #[serde(default = "default_tone")]
    pub default_tone: String,
    pub default_format: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn default_tone() -> String {
    "professional".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionRecord {
    pub id: String,
    pub user_id: String,
    pub original_text: String,
    pub corrected_text: String,
    pub correction_type: String,
    pub pattern: String,
    pub replacement: String,
    pub context: HashMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub embedding: Vec<f32>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub applied_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinedOutput {
    pub text: String,
    pub format: String,
    pub tone: String,
    pub confidence: f64,
    pub processing_time: f64,
    pub applied_rules: Vec<AppliedRule>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedRule {
    pub rule_type: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenoiseResult {
    pub cleaned_text: String,
    pub removed_fillers: Vec<String>,
    pub removed_duplicates: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureResult {
    pub formatted_text: String,
    pub detected_format: String,
    pub applied_tone: String,
    pub structure_metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizeResult {
    pub refined_output: String,
    pub applied_rules: Vec<AppliedRule>,
    pub context_used: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub transcript: String,
    pub refined: String,
    pub delivered: bool,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalASRModelStatus {
    pub model_name: String,
    pub downloaded: bool,
    pub path: String,
    pub size_mb: f64,
    pub suggested_download_mb: u32,
}
