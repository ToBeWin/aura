use crate::errors::{AuraError, Result};
use crate::models::{ASRCloudProvider, LLMCloudProvider, ProviderSettings};
use std::path::PathBuf;

pub fn aura_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".aura")
}

pub fn settings_path() -> PathBuf {
    aura_data_dir().join("settings.json")
}

pub fn context_db_path() -> PathBuf {
    aura_data_dir().join("aura_context.db")
}

pub fn vector_db_path() -> PathBuf {
    aura_data_dir().join("aura_vectors.db")
}

pub fn load_provider_settings() -> Result<ProviderSettings> {
    let path = settings_path();
    if !path.exists() {
        return Ok(ProviderSettings::default());
    }

    let content = std::fs::read_to_string(&path).map_err(|error| AuraError::Processing {
        message: format!("Cannot read settings file: {}", error),
        error_code: "SETTINGS_001".to_string(),
    })?;

    if content.trim().is_empty() {
        return Ok(ProviderSettings::default());
    }

    let settings: ProviderSettings = serde_json::from_str(&content).map_err(|error| AuraError::Processing {
        message: format!("Cannot parse settings file: {}", error),
        error_code: "SETTINGS_002".to_string(),
    })?;
    Ok(settings)
}

pub fn normalize_provider_settings(settings: &mut ProviderSettings) -> bool {
    let mut changed = false;

    if settings.asr.language.trim().is_empty() || settings.asr.language.eq_ignore_ascii_case("zh") {
        settings.asr.language = "auto".to_string();
        changed = true;
    }

    let desired_asr_base = default_asr_cloud_base_url(&settings.asr.cloud_provider);
    if settings.asr.cloud_base_url.trim().is_empty()
        || is_legacy_openai_base(&settings.asr.cloud_base_url, desired_asr_base)
    {
        let next = desired_asr_base.to_string();
        if settings.asr.cloud_base_url != next {
            settings.asr.cloud_base_url = next;
            changed = true;
        }
    }

    let desired_asr_model = default_asr_cloud_model(&settings.asr.cloud_provider);
    if settings.asr.cloud_model.trim().is_empty() {
        settings.asr.cloud_model = desired_asr_model.to_string();
        changed = true;
    }

    let desired_llm_base = default_llm_cloud_base_url(&settings.llm.cloud_provider);
    if settings.llm.cloud_base_url.trim().is_empty()
        || is_legacy_openai_base(&settings.llm.cloud_base_url, desired_llm_base)
    {
        let next = desired_llm_base.to_string();
        if settings.llm.cloud_base_url != next {
            settings.llm.cloud_base_url = next;
            changed = true;
        }
    }

    let desired_llm_model = default_llm_cloud_model(&settings.llm.cloud_provider);
    if settings.llm.cloud_model.trim().is_empty() {
        settings.llm.cloud_model = desired_llm_model.to_string();
        changed = true;
    }

    let desired_embedding_model = default_llm_embedding_model(&settings.llm.cloud_provider);
    if settings.llm.cloud_embedding_model.trim().is_empty() {
        settings.llm.cloud_embedding_model = desired_embedding_model.to_string();
        changed = true;
    }

    changed
}

fn is_legacy_openai_base(current: &str, desired: &str) -> bool {
    let trimmed = current.trim_end_matches('/');
    trimmed.eq_ignore_ascii_case("https://api.openai.com/v1") && !trimmed.eq_ignore_ascii_case(desired)
}

pub fn default_llm_cloud_base_url(provider: &LLMCloudProvider) -> &'static str {
    match provider {
        LLMCloudProvider::OpenAI => "https://api.openai.com/v1",
        LLMCloudProvider::Anthropic => "https://api.anthropic.com/v1",
        LLMCloudProvider::Gemini => "https://generativelanguage.googleapis.com/v1beta",
        LLMCloudProvider::DeepSeek => "https://api.deepseek.com/v1",
        LLMCloudProvider::Qwen => "https://dashscope.aliyuncs.com/compatible-mode/v1",
        LLMCloudProvider::Glm => "https://open.bigmodel.cn/api/paas/v4",
        LLMCloudProvider::Kimi => "https://api.moonshot.cn/v1",
        LLMCloudProvider::Minimax => "https://api.minimax.chat/v1",
        LLMCloudProvider::OpenRouter => "https://openrouter.ai/api/v1",
        LLMCloudProvider::Custom => "https://api.openai.com/v1",
    }
}

pub fn default_llm_cloud_model(provider: &LLMCloudProvider) -> &'static str {
    match provider {
        LLMCloudProvider::OpenAI => "gpt-4.1-mini",
        LLMCloudProvider::Anthropic => "claude-3-5-sonnet-latest",
        LLMCloudProvider::Gemini => "gemini-1.5-pro",
        LLMCloudProvider::DeepSeek => "deepseek-chat",
        LLMCloudProvider::Qwen => "qwen-plus",
        LLMCloudProvider::Glm => "glm-4",
        LLMCloudProvider::Kimi => "moonshot-v1-32k",
        LLMCloudProvider::Minimax => "abab6.5s",
        LLMCloudProvider::OpenRouter => "openai/gpt-4.1-mini",
        LLMCloudProvider::Custom => "gpt-4.1-mini",
    }
}

pub fn default_llm_embedding_model(provider: &LLMCloudProvider) -> &'static str {
    match provider {
        LLMCloudProvider::OpenAI => "text-embedding-3-small",
        LLMCloudProvider::Anthropic => "text-embedding-3-small",
        LLMCloudProvider::Gemini => "text-embedding-3-small",
        LLMCloudProvider::DeepSeek => "text-embedding-3-small",
        LLMCloudProvider::Qwen => "text-embedding-3-small",
        LLMCloudProvider::Glm => "text-embedding-3-small",
        LLMCloudProvider::Kimi => "text-embedding-3-small",
        LLMCloudProvider::Minimax => "text-embedding-3-small",
        LLMCloudProvider::OpenRouter => "text-embedding-3-small",
        LLMCloudProvider::Custom => "text-embedding-3-small",
    }
}

pub fn default_asr_cloud_base_url(provider: &ASRCloudProvider) -> &'static str {
    match provider {
        ASRCloudProvider::OpenAI => "https://api.openai.com/v1",
        ASRCloudProvider::Groq => "https://api.groq.com/openai/v1",
        ASRCloudProvider::Deepgram => "https://api.deepgram.com/v1",
        ASRCloudProvider::AssemblyAI => "https://api.assemblyai.com/v2",
        ASRCloudProvider::Azure => "https://eastus.stt.speech.microsoft.com",
        ASRCloudProvider::Google => "https://speech.googleapis.com/v1",
        ASRCloudProvider::Custom => "https://api.openai.com/v1",
    }
}

pub fn default_asr_cloud_model(provider: &ASRCloudProvider) -> &'static str {
    match provider {
        ASRCloudProvider::OpenAI => "gpt-4o-mini-transcribe",
        ASRCloudProvider::Groq => "whisper-large-v3-turbo",
        ASRCloudProvider::Deepgram => "nova-2",
        ASRCloudProvider::AssemblyAI => "best",
        ASRCloudProvider::Azure => "latest",
        ASRCloudProvider::Google => "latest_long",
        ASRCloudProvider::Custom => "gpt-4o-mini-transcribe",
    }
}

pub fn save_provider_settings(settings: &ProviderSettings) -> Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| AuraError::Processing {
            message: format!("Cannot create settings directory: {}", error),
            error_code: "SETTINGS_003".to_string(),
        })?;
    }

    let content = serde_json::to_string_pretty(settings).map_err(|error| AuraError::Processing {
        message: format!("Cannot serialize settings: {}", error),
        error_code: "SETTINGS_004".to_string(),
    })?;

    std::fs::write(&path, content).map_err(|error| AuraError::Processing {
        message: format!("Cannot write settings file: {}", error),
        error_code: "SETTINGS_005".to_string(),
    })?;

    Ok(())
}
