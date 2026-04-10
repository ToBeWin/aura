use crate::errors::{AuraError, Result};
use crate::models::{LLMCloudProvider, LLMProviderSettings, ProviderMode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_LOCAL_BASE_URL: &str = "http://127.0.0.1:11434";

#[derive(Debug, Clone)]
enum LLMBackend {
    Local {
        model_name: String,
        base_url: String,
    },
    Cloud {
        provider: LLMCloudProvider,
        model_name: String,
        embedding_model: String,
        base_url: String,
        api_key: String,
    },
}

#[derive(Debug, Clone)]
pub struct LocalLLM {
    backend: LLMBackend,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Debug, Serialize)]
struct GenerateOptions {
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Debug, Serialize)]
struct EmbedRequest {
    model: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Serialize)]
struct CloudEmbedRequest {
    model: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct CloudEmbedResponse {
    data: Vec<CloudEmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct CloudEmbeddingData {
    embedding: Vec<f32>,
}

impl LocalLLM {
    pub fn new(model_name: String) -> Self {
        Self::from_settings(&LLMProviderSettings {
            local_model: model_name,
            ..LLMProviderSettings::default()
        })
    }

    pub fn from_settings(settings: &LLMProviderSettings) -> Self {
        let client = match settings.provider {
            ProviderMode::Local => reqwest::Client::builder()
                .connect_timeout(Duration::from_millis(500))
                .timeout(Duration::from_secs(18))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            ProviderMode::Cloud => reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(45))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        };

        let backend = match settings.provider {
            ProviderMode::Local => LLMBackend::Local {
                model_name: settings.local_model.clone(),
                base_url: if settings.local_base_url.trim().is_empty() {
                    DEFAULT_LOCAL_BASE_URL.trim_end_matches('/').to_string()
                } else {
                    settings.local_base_url.trim_end_matches('/').to_string()
                },
            },
            ProviderMode::Cloud => LLMBackend::Cloud {
                provider: settings.cloud_provider.clone(),
                model_name: settings.cloud_model.clone(),
                embedding_model: settings.cloud_embedding_model.clone(),
                base_url: settings.cloud_base_url.trim_end_matches('/').to_string(),
                api_key: settings.cloud_api_key.clone(),
            },
        };

        Self { backend, client }
    }

    #[allow(dead_code)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        match &mut self.backend {
            LLMBackend::Local { base_url: current, .. } => *current = base_url,
            LLMBackend::Cloud { base_url: current, .. } => *current = base_url,
        }
        self
    }

    pub async fn generate(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        max_tokens: Option<i32>,
        temperature: f32,
    ) -> Result<String> {
        match &self.backend {
            LLMBackend::Local { model_name, base_url } => {
                let request = GenerateRequest {
                    model: model_name.clone(),
                    prompt: prompt.to_string(),
                    system: system_prompt.map(|s| s.to_string()),
                    stream: false,
                    options: GenerateOptions {
                        temperature,
                        num_predict: max_tokens,
                    },
                };

                let response = self
                    .client
                    .post(format!("{}/api/generate", base_url))
                    .json(&request)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(AuraError::Processing {
                        message: format!("LLM API error: {}", response.status()),
                        error_code: "LLM_001".to_string(),
                    });
                }

                let result: GenerateResponse = response.json().await?;
                Ok(result.response)
            }
            LLMBackend::Cloud {
                provider,
                model_name,
                base_url,
                api_key,
                ..
            } => {
                if api_key.trim().is_empty() {
                    return Err(AuraError::Processing {
                        message: "Cloud LLM API key is missing".to_string(),
                        error_code: "LLM_101".to_string(),
                    });
                }

                let content = match provider {
                    LLMCloudProvider::Anthropic => {
                        self.generate_anthropic(model_name, base_url, api_key, prompt, system_prompt, max_tokens, temperature)
                            .await?
                    }
                    LLMCloudProvider::Gemini => {
                        self.generate_gemini(model_name, base_url, api_key, prompt, system_prompt, max_tokens, temperature)
                            .await?
                    }
                    LLMCloudProvider::OpenAI
                    | LLMCloudProvider::DeepSeek
                    | LLMCloudProvider::Qwen
                    | LLMCloudProvider::Glm
                    | LLMCloudProvider::Kimi
                    | LLMCloudProvider::Minimax
                    | LLMCloudProvider::OpenRouter
                    | LLMCloudProvider::Custom => {
                        self.generate_openai_compatible(model_name, base_url, api_key, prompt, system_prompt, max_tokens, temperature)
                            .await?
                    }
                };

                if content.trim().is_empty() {
                    return Err(AuraError::Processing {
                        message: "Cloud LLM returned an empty response".to_string(),
                        error_code: "LLM_103".to_string(),
                    });
                }

                Ok(content)
            }
        }
    }

    async fn generate_openai_compatible(
        &self,
        model_name: &str,
        base_url: &str,
        api_key: &str,
        prompt: &str,
        system_prompt: Option<&str>,
        max_tokens: Option<i32>,
        temperature: f32,
    ) -> Result<String> {
        let mut messages = Vec::new();
        if let Some(system) = system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: serde_json::Value::String(system.to_string()),
            });
        }
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: serde_json::Value::String(prompt.to_string()),
        });

        let request = ChatCompletionRequest {
            model: model_name.to_string(),
            messages,
            temperature,
            max_tokens,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", base_url))
            .bearer_auth(api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuraError::Processing {
                message: format!("Cloud LLM API error: {}", response.status()),
                error_code: "LLM_102".to_string(),
            });
        }

        let result: ChatCompletionResponse = response.json().await?;
        Ok(result
            .choices
            .into_iter()
            .next()
            .and_then(|choice| extract_chat_content(choice.message.content))
            .unwrap_or_default())
    }

    async fn generate_anthropic(
        &self,
        model_name: &str,
        base_url: &str,
        api_key: &str,
        prompt: &str,
        system_prompt: Option<&str>,
        max_tokens: Option<i32>,
        temperature: f32,
    ) -> Result<String> {
        #[derive(Serialize)]
        struct AnthropicMessage {
            role: String,
            content: Vec<AnthropicContent>,
        }

        #[derive(Serialize)]
        struct AnthropicContent {
            #[serde(rename = "type")]
            kind: String,
            text: String,
        }

        #[derive(Serialize)]
        struct AnthropicRequest {
            model: String,
            max_tokens: i32,
            temperature: f32,
            #[serde(skip_serializing_if = "Option::is_none")]
            system: Option<String>,
            messages: Vec<AnthropicMessage>,
        }

        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Vec<AnthropicContentResponse>,
        }

        #[derive(Deserialize)]
        struct AnthropicContentResponse {
            text: String,
        }

        let request = AnthropicRequest {
            model: model_name.to_string(),
            max_tokens: max_tokens.unwrap_or(512),
            temperature,
            system: system_prompt.map(|value| value.to_string()),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: vec![AnthropicContent {
                    kind: "text".to_string(),
                    text: prompt.to_string(),
                }],
            }],
        };

        let response = self
            .client
            .post(format!("{}/messages", base_url.trim_end_matches('/')))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuraError::Processing {
                message: format!("Anthropic API error: {}", response.status()),
                error_code: "LLM_202".to_string(),
            });
        }

        let result: AnthropicResponse = response.json().await?;
        Ok(result
            .content
            .into_iter()
            .next()
            .map(|value| value.text)
            .unwrap_or_default())
    }

    async fn generate_gemini(
        &self,
        model_name: &str,
        base_url: &str,
        api_key: &str,
        prompt: &str,
        system_prompt: Option<&str>,
        max_tokens: Option<i32>,
        temperature: f32,
    ) -> Result<String> {
        #[derive(Serialize)]
        struct GeminiPart {
            text: String,
        }

        #[derive(Serialize)]
        struct GeminiContent {
            role: String,
            parts: Vec<GeminiPart>,
        }

        #[derive(Serialize)]
        struct GeminiSystemInstruction {
            parts: Vec<GeminiPart>,
        }

        #[derive(Serialize)]
        struct GeminiRequest {
            contents: Vec<GeminiContent>,
            #[serde(skip_serializing_if = "Option::is_none")]
            system_instruction: Option<GeminiSystemInstruction>,
            generation_config: GeminiGenerationConfig,
        }

        #[derive(Serialize)]
        struct GeminiGenerationConfig {
            temperature: f32,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_output_tokens: Option<i32>,
        }

        #[derive(Deserialize)]
        struct GeminiResponse {
            candidates: Vec<GeminiCandidate>,
        }

        #[derive(Deserialize)]
        struct GeminiCandidate {
            content: GeminiResponseContent,
        }

        #[derive(Deserialize)]
        struct GeminiResponseContent {
            parts: Vec<GeminiPartResponse>,
        }

        #[derive(Deserialize)]
        struct GeminiPartResponse {
            text: String,
        }

        let request = GeminiRequest {
            contents: vec![GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    text: prompt.to_string(),
                }],
            }],
            system_instruction: system_prompt.map(|value| GeminiSystemInstruction {
                parts: vec![GeminiPart {
                    text: value.to_string(),
                }],
            }),
            generation_config: GeminiGenerationConfig {
                temperature,
                max_output_tokens: max_tokens,
            },
        };

        let base = base_url.trim_end_matches('/');
        let url = format!("{}/models/{}:generateContent?key={}", base, model_name, api_key);

        let response = self.client.post(url).json(&request).send().await?;

        if !response.status().is_success() {
            return Err(AuraError::Processing {
                message: format!("Gemini API error: {}", response.status()),
                error_code: "LLM_302".to_string(),
            });
        }

        let result: GeminiResponse = response.json().await?;
        Ok(result
            .candidates
            .into_iter()
            .next()
            .and_then(|candidate| candidate.content.parts.into_iter().next())
            .map(|part| part.text)
            .unwrap_or_default())
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        match &self.backend {
            LLMBackend::Local { model_name, base_url } => {
                let request = EmbedRequest {
                    model: model_name.clone(),
                    input: text.to_string(),
                };

                let response = self
                    .client
                    .post(format!("{}/api/embed", base_url))
                    .json(&request)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(AuraError::Processing {
                        message: format!("Embed API error: {}", response.status()),
                        error_code: "LLM_002".to_string(),
                    });
                }

                let result: EmbedResponse = response.json().await?;
                Ok(result.embeddings.into_iter().next().unwrap_or_default())
            }
            LLMBackend::Cloud {
                embedding_model,
                base_url,
                api_key,
                ..
            } => {
                if api_key.trim().is_empty() {
                    return Err(AuraError::Processing {
                        message: "Cloud embedding API key is missing".to_string(),
                        error_code: "LLM_104".to_string(),
                    });
                }

                let request = CloudEmbedRequest {
                    model: embedding_model.clone(),
                    input: text.to_string(),
                };

                let response = self
                    .client
                    .post(format!("{}/embeddings", base_url))
                    .bearer_auth(api_key)
                    .json(&request)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(AuraError::Processing {
                        message: format!("Cloud embed API error: {}", response.status()),
                        error_code: "LLM_105".to_string(),
                    });
                }

                let result: CloudEmbedResponse = response.json().await?;
                Ok(result
                    .data
                    .into_iter()
                    .next()
                    .map(|value| value.embedding)
                    .unwrap_or_default())
            }
        }
    }

    pub async fn preload(&self) -> Result<()> {
        match &self.backend {
            LLMBackend::Local { model_name, .. } => {
                log::info!("Preloading local model: {}", model_name);
                let warmup_prompt = "Hello";
                self.generate(warmup_prompt, None, Some(10), 0.7).await?;
                log::info!("Local model preloaded successfully");
                Ok(())
            }
            LLMBackend::Cloud { .. } => Ok(()),
        }
    }

    #[allow(dead_code)]
    pub async fn check_model_available(&self) -> Result<bool> {
        match &self.backend {
            LLMBackend::Local { model_name, base_url } => {
                let response = self.client.get(format!("{}/api/tags", base_url)).send().await?;
                if !response.status().is_success() {
                    return Ok(false);
                }
                let body = response.text().await?;
                Ok(body.contains(model_name))
            }
            LLMBackend::Cloud { api_key, .. } => Ok(!api_key.trim().is_empty()),
        }
    }
}

fn extract_chat_content(content: serde_json::Value) -> Option<String> {
    match content {
        serde_json::Value::String(text) => Some(text),
        serde_json::Value::Array(parts) => {
            let combined = parts
                .into_iter()
                .filter_map(|part| {
                    part.get("text")
                        .and_then(|value| value.as_str())
                        .map(|value| value.to_string())
                })
                .collect::<Vec<_>>()
                .join("");
            if combined.trim().is_empty() {
                None
            } else {
                Some(combined)
            }
        }
        _ => None,
    }
}
