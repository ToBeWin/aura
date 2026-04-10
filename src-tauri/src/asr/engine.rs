use crate::errors::{AuraError, Result};
use crate::models::{ASRCloudProvider, ASRProviderSettings, LocalASRModelStatus, ProviderMode};
use crate::text::normalize_to_simplified_chinese;
use base64::Engine;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::{sleep, Duration as TokioDuration};

#[derive(Clone)]
pub struct ASREngine {
    inner: std::sync::Arc<ASRInner>,
}

struct ASRInner {
    backend: ASRBackend,
    language: String,
    client: reqwest::Client,
}

enum ASRBackend {
    Local {
        model_path: PathBuf,
    },
    Cloud {
        provider: ASRCloudProvider,
        base_url: String,
        api_key: String,
        model: String,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: String,
    pub confidence: f64,
    pub segments: Vec<TranscriptionSegment>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptionSegment {
    pub text: String,
    pub start: f64,
    pub end: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
struct LocalTranscriptCandidate {
    text: String,
    language: String,
    confidence: f64,
}

impl ASREngine {
    pub fn new(_model_name: String, language: String) -> Self {
        Self::from_settings(&ASRProviderSettings {
            local_model: _model_name,
            language,
            ..ASRProviderSettings::default()
        })
    }

    pub fn from_settings(settings: &ASRProviderSettings) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let backend = match settings.provider {
            ProviderMode::Local => ASRBackend::Local {
                model_path: local_model_path(&settings.local_model),
            },
            ProviderMode::Cloud => ASRBackend::Cloud {
                provider: settings.cloud_provider.clone(),
                base_url: settings.cloud_base_url.trim_end_matches('/').to_string(),
                api_key: settings.cloud_api_key.clone(),
                model: settings.cloud_model.clone(),
            },
        };

        Self {
            inner: std::sync::Arc::new(ASRInner {
                backend,
                language: settings.language.clone(),
                client,
            }),
        }
    }

    /// Ensure whisper model exists. Download if missing.
    fn ensure_model(&self) -> Result<()> {
        if let ASRBackend::Local { model_path } = &self.inner.backend {
            if !model_path.exists() {
                self.download_model()?;
            }
        }
        Ok(())
    }

    /// Download whisper model, trying Chinese mirror first then fallback.
    fn download_model(&self) -> Result<()> {
        let model_path = match &self.inner.backend {
            ASRBackend::Local { model_path } => model_path,
            ASRBackend::Cloud { .. } => {
                return Ok(());
            }
        };

        let parent = model_path.parent().ok_or_else(|| AuraError::Processing {
            message: "Cannot determine model parent directory".to_string(),
            error_code: "ASR_010".to_string(),
        })?;

        std::fs::create_dir_all(parent).map_err(|e| AuraError::Processing {
            message: format!("Cannot create model directory: {}", e),
            error_code: "ASR_011".to_string(),
        })?;

        let file_name = model_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("ggml-base.bin")
            .to_string();

        log::info!("Downloading whisper model {} to {:?}", file_name, model_path);

        let mirrors = [
            format!("https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/{}", file_name),
            format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}", file_name),
        ];

        let mut last_error = None;

        for url in mirrors {
            log::info!("Trying mirror: {}", url);
            let status = std::process::Command::new("curl")
                .args([
                    "-L", "--fail", "--max-time", "120",
                    "-o", model_path.to_str().unwrap(),
                    url.as_str(),
                ])
                .status()
                .map_err(|e| AuraError::Processing {
                    message: format!("curl not found: {}. Please install curl to download the whisper model.", e),
                    error_code: "ASR_012".to_string(),
                })?;

            if status.success() {
                break;
            }

            let _ = std::fs::remove_file(model_path);
            last_error = Some(format!("curl exit {}", status));
            log::warn!("Mirror {} failed, trying next... ({})", url, last_error.as_ref().unwrap());
        }

        if let Some(err) = last_error {
            if !model_path.exists() {
                return Err(AuraError::Processing {
                    message: format!(
                        "All mirrors failed ({}). Manually download:\n  curl -L -o {:?} https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/{}",
                        err, model_path, file_name
                    ),
                    error_code: "ASR_013".to_string(),
                });
            }
        }

        let meta = std::fs::metadata(model_path).map_err(|e| AuraError::Processing {
            message: format!("Cannot verify model download: {}", e),
            error_code: "ASR_014".to_string(),
        })?;

        if meta.len() < 100_000_000 {
            return Err(AuraError::Processing {
                message: format!(
                    "Downloaded model seems too small ({} bytes, expected ~142MB). Try downloading manually:\n  curl -L -o {:?} https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/{}",
                    meta.len(), model_path, file_name
                ),
                error_code: "ASR_015".to_string(),
            });
        }

        log::info!("Whisper model downloaded successfully ({} bytes)", meta.len());
        Ok(())
    }

    pub fn download_local_model(model_name: &str) -> Result<LocalASRModelStatus> {
        let engine = Self::from_settings(&ASRProviderSettings {
            provider: ProviderMode::Local,
            local_model: model_name.to_string(),
            ..ASRProviderSettings::default()
        });
        engine.download_model()?;
        Self::local_model_status(model_name)
    }

    pub fn local_model_status(model_name: &str) -> Result<LocalASRModelStatus> {
        let path = local_model_path(model_name);
        let metadata = std::fs::metadata(&path).ok();
        let size_mb = metadata
            .as_ref()
            .map(|value| value.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);

        Ok(LocalASRModelStatus {
            model_name: normalize_local_model_name(model_name).to_string(),
            downloaded: metadata.is_some(),
            path: path.display().to_string(),
            size_mb,
            suggested_download_mb: local_model_expected_size_mb(model_name),
        })
    }

    /// Transcribe audio file to text
    pub async fn transcribe(&self, audio_path: impl AsRef<Path>) -> Result<TranscriptionResult> {
        let audio_path = audio_path.as_ref();

        if !audio_path.exists() {
            return Err(AuraError::InputValidation {
                message: format!("Audio file not found: {:?}", audio_path),
                error_code: "ASR_001".to_string(),
            });
        }

        if matches!(self.inner.backend, ASRBackend::Cloud { .. }) {
            let audio_data = std::fs::read(audio_path).map_err(|e| AuraError::Processing {
                message: format!("Cannot read audio file: {}", e),
                error_code: "ASR_004".to_string(),
            })?;
            let format = audio_path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("wav");
            return self.transcribe_cloud_bytes(&audio_data, format).await;
        }

        let wav_path = self.ensure_wav_format(audio_path)?;

        // Read WAV and extract f32 mono samples
        let mono_samples = self.wav_to_mono_f32(&wav_path)?;

        // Cleanup temp wav if we converted
        if wav_path != *audio_path {
            let _ = std::fs::remove_file(&wav_path);
        }

        let candidate = self.transcribe_mono_f32(&mono_samples)?;
        let normalized_text = normalize_to_simplified_chinese(&candidate.text);

        Ok(TranscriptionResult {
            text: normalized_text.clone(),
            language: candidate.language,
            confidence: candidate.confidence,
            segments: vec![TranscriptionSegment {
                text: normalized_text,
                start: 0.0,
                end: 0.0,
                confidence: candidate.confidence,
            }],
        })
    }

    /// Transcribe audio data from memory
    pub async fn transcribe_bytes(&self, audio_data: &[u8], format: &str) -> Result<TranscriptionResult> {
        if matches!(self.inner.backend, ASRBackend::Cloud { .. }) {
            return self.transcribe_cloud_bytes(audio_data, format).await;
        }

        let temp_path = std::env::temp_dir()
            .join(format!("aura_audio_{}.{}", uuid::Uuid::new_v4(), format));

        std::fs::write(&temp_path, audio_data).map_err(|e| AuraError::Processing {
            message: format!("Cannot write temp audio file: {}", e),
            error_code: "ASR_008".to_string(),
        })?;

        let result = self.transcribe(&temp_path).await?;

        let _ = std::fs::remove_file(&temp_path);

        Ok(result)
    }

    async fn transcribe_cloud_bytes(&self, audio_data: &[u8], format: &str) -> Result<TranscriptionResult> {
        let (provider, base_url, api_key, model) = match &self.inner.backend {
            ASRBackend::Cloud {
                provider,
                base_url,
                api_key,
                model,
            } => (provider.clone(), base_url.clone(), api_key.clone(), model.clone()),
            ASRBackend::Local { .. } => {
                return Err(AuraError::Processing {
                    message: "Cloud transcription is not enabled".to_string(),
                    error_code: "ASR_101".to_string(),
                });
            }
        };

        if api_key.trim().is_empty() {
            return Err(AuraError::Processing {
                message: "Cloud ASR API key is missing".to_string(),
                error_code: "ASR_102".to_string(),
            });
        }

        match provider {
            ASRCloudProvider::OpenAI | ASRCloudProvider::Groq | ASRCloudProvider::Custom => {
                self.transcribe_openai_compatible(&base_url, &api_key, &model, audio_data, format)
                    .await
            }
            ASRCloudProvider::Deepgram => {
                self.transcribe_deepgram(&base_url, &api_key, &model, audio_data, format)
                    .await
            }
            ASRCloudProvider::AssemblyAI => {
                self.transcribe_assemblyai(&base_url, &api_key, &model, audio_data, format)
                    .await
            }
            ASRCloudProvider::Azure => {
                self.transcribe_azure(&base_url, &api_key, audio_data, format)
                    .await
            }
            ASRCloudProvider::Google => {
                self.transcribe_google(&base_url, &api_key, audio_data, format)
                    .await
            }
        }
    }

    /// Ensure audio is WAV 16kHz mono
    fn ensure_wav_format(&self, audio_path: &Path) -> Result<std::path::PathBuf> {
        // Check if already WAV 16kHz mono
        if audio_path.extension().and_then(|e| e.to_str()) == Some("wav") {
            if let Ok(spec) = hound::WavReader::open(audio_path) {
                let header = spec.spec();
                if header.sample_rate == 16000 && header.channels == 1 {
                    return Ok(audio_path.to_path_buf());
                }
            }
        }

        // Convert to 16kHz mono WAV using ffmpeg
        let output_path = if audio_path.extension().and_then(|e| e.to_str()) == Some("wav") {
            std::env::temp_dir().join(format!("aura_ffmpeg_{}.wav", uuid::Uuid::new_v4()))
        } else {
            audio_path.with_extension("wav")
        };

        let status = std::process::Command::new("ffmpeg")
            .args([
                "-i", audio_path.to_str().unwrap(),
                "-ar", "16000",
                "-ac", "1",
                "-f", "wav",
                "-y",
                output_path.to_str().unwrap(),
            ])
            .status()
            .map_err(|e| AuraError::Processing {
                message: format!("ffmpeg not found: {}. Please install ffmpeg.", e),
                error_code: "ASR_002".to_string(),
            })?;

        if !status.success() {
            return Err(AuraError::Processing {
                message: "Audio conversion failed. Ensure ffmpeg is installed.".to_string(),
                error_code: "ASR_003".to_string(),
            });
        }

        Ok(output_path)
    }

    /// Read WAV file and produce mono f32 samples for whisper (already 16kHz from ensure_wav)
    fn wav_to_mono_f32(&self, wav_path: &Path) -> Result<Vec<f32>> {
        let reader = hound::WavReader::open(wav_path).map_err(|e| AuraError::Processing {
            message: format!("Cannot read WAV file: {}", e),
            error_code: "ASR_020".to_string(),
        })?;

        let spec = reader.spec();

        match spec.sample_format {
            hound::SampleFormat::Int => {
                let bits = spec.bits_per_sample;
                let samples: Result<Vec<i32>> = reader
                    .into_samples::<i32>()
                    .map(|s| s.map_err(|e| AuraError::Processing {
                        message: format!("WAV sample read error: {}", e),
                        error_code: "ASR_022".to_string(),
                    }))
                    .collect();

                let max = (1 << (bits - 1)) as f32;
                let mono: Vec<f32> = samples?.into_iter().map(|s| s as f32 / max).collect();

                if spec.channels == 1 {
                    Ok(mono)
                } else {
                    // Convert stereo to mono
                    let mut out = Vec::with_capacity(mono.len() / 2);
                    for chunk in mono.chunks(2) {
                        if chunk.len() == 2 {
                            out.push((chunk[0] + chunk[1]) / 2.0);
                        }
                    }
                    Ok(out)
                }
            }
            hound::SampleFormat::Float => {
                let samples: Result<Vec<f32>> = reader
                    .into_samples::<f32>()
                    .map(|s| s.map_err(|e| AuraError::Processing {
                        message: format!("WAV sample read error: {}", e),
                        error_code: "ASR_023".to_string(),
                    }))
                    .collect();

                let f32_samples = samples?;

                if spec.channels == 1 {
                    Ok(f32_samples)
                } else {
                    let mut out = Vec::with_capacity(f32_samples.len() / 2);
                    for chunk in f32_samples.chunks(2) {
                        if chunk.len() == 2 {
                            out.push((chunk[0] + chunk[1]) / 2.0);
                        }
                    }
                    Ok(out)
                }
            }
        }
    }

    /// Run whisper on mono f32 samples
    fn transcribe_mono_f32(&self, samples: &[f32]) -> Result<LocalTranscriptCandidate> {
        if samples.is_empty() {
            return Ok(LocalTranscriptCandidate {
                text: String::new(),
                language: self.inner.language.clone(),
                confidence: 0.0,
            });
        }

        self.ensure_model()?;

        let configured_language = self.inner.language.trim().to_ascii_lowercase();
        if configured_language.is_empty() || configured_language == "auto" {
            let zh_candidate = self.transcribe_with_language_hint(samples, Some("zh"))?;
            let en_candidate = self.transcribe_with_language_hint(samples, Some("en"))?;
            let best = choose_best_candidate(zh_candidate, en_candidate);
            log::info!(
                "[Aura] Local ASR auto resolved to language={} confidence={:.2} text={}",
                best.language,
                best.confidence,
                best.text
            );
            return Ok(best);
        }

        let hint = normalized_local_language_hint(&configured_language);
        self.transcribe_with_language_hint(samples, hint)
    }

    fn transcribe_with_language_hint(
        &self,
        samples: &[f32],
        language_hint: Option<&str>,
    ) -> Result<LocalTranscriptCandidate> {
        let text = self.run_whisper(samples, language_hint)?;
        let normalized_language = language_hint.unwrap_or(self.inner.language.as_str());
        let confidence = score_candidate_text(&text, normalized_language);
        Ok(LocalTranscriptCandidate {
            text,
            language: normalized_language.to_string(),
            confidence,
        })
    }

    fn run_whisper(&self, samples: &[f32], language_hint: Option<&str>) -> Result<String> {
        if samples.is_empty() {
            return Ok(String::new());
        }

        use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

        let model_path = match &self.inner.backend {
            ASRBackend::Local { model_path } => model_path,
            ASRBackend::Cloud { .. } => {
                return Err(AuraError::Processing {
                    message: "Local whisper model is not enabled".to_string(),
                    error_code: "ASR_105".to_string(),
                });
            }
        };

        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or_else(|| AuraError::Processing {
                message: "Model path contains invalid UTF-8".to_string(),
                error_code: "ASR_030".to_string(),
            })?,
            WhisperContextParameters::default(),
        )
        .map_err(|e| AuraError::Processing {
            message: format!("Failed to load whisper model: {}", e),
            error_code: "ASR_031".to_string(),
        })?;

        let mut state = ctx.create_state().map_err(|e| AuraError::Processing {
            message: format!("Failed to create whisper state: {}", e),
            error_code: "ASR_032".to_string(),
        })?;

        let mut params = FullParams::new(SamplingStrategy::Greedy {
            best_of: 1,
        });

        if let Some(code) = language_hint {
            params.set_language(Some(code));
        } else {
            params.set_detect_language(true);
        }

        let decode_threads = std::thread::available_parallelism()
            .map(|value| value.get().min(4) as i32)
            .unwrap_or(2);
        params.set_n_threads(decode_threads);
        params.set_translate(false);
        params.set_no_context(true);
        params.set_no_timestamps(true);
        params.set_single_segment(true);
        params.set_token_timestamps(false);
        params.set_split_on_word(false);
        params.set_max_len(96);
        params.set_debug_mode(false);

        // Suppress all whisper printing
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // Run transcription
        state.full(params, samples).map_err(|e| AuraError::Processing {
            message: format!("Whisper transcription failed: {}", e),
            error_code: "ASR_040".to_string(),
        })?;

        // Iterate over segments and collect text
        let mut full_text = String::new();
        for segment in state.as_iter() {
            if !full_text.is_empty() {
                full_text.push(' ');
            }
            full_text.push_str(
                segment.to_str_lossy().unwrap_or_else(|_| "error".into()).as_ref(),
            );
        }

        Ok(full_text.trim().to_string())
    }
}

fn choose_best_candidate(
    zh_candidate: LocalTranscriptCandidate,
    en_candidate: LocalTranscriptCandidate,
) -> LocalTranscriptCandidate {
    let zh_score = score_candidate(&zh_candidate);
    let en_score = score_candidate(&en_candidate);

    if (zh_score - en_score).abs() < 0.08 {
        let zh_cjk = count_cjk_chars(&zh_candidate.text);
        let en_ascii_words = count_ascii_words(&en_candidate.text);
        if en_ascii_words >= 3 && en_ascii_words >= zh_cjk {
            return en_candidate;
        }
        return zh_candidate;
    }

    if en_score > zh_score {
        en_candidate
    } else {
        zh_candidate
    }
}

fn score_candidate(candidate: &LocalTranscriptCandidate) -> f64 {
    let base = candidate.confidence;
    let text = candidate.text.trim();
    if text.is_empty() {
        return 0.0;
    }

    let cjk_count = count_cjk_chars(text);
    let ascii_letters = count_ascii_letters(text);
    let ascii_words = count_ascii_words(text);
    let replacement_count = text.matches('�').count();
    let digit_count = text.chars().filter(|ch| ch.is_ascii_digit()).count();
    let punctuation_count = text
        .chars()
        .filter(|ch| matches!(ch, '.' | ',' | '!' | '?' | ';' | ':' | '-' | '\'' | '"'))
        .count();

    let mut score = base;
    let total_chars = text.chars().count().max(1) as f64;
    let cjk_ratio = cjk_count as f64 / total_chars;
    let ascii_ratio = ascii_letters as f64 / total_chars;

    if candidate.language == "en" {
        score += ascii_ratio * 0.55;
        score += (ascii_words.min(12) as f64) * 0.035;
        score += (punctuation_count.min(6) as f64) * 0.01;
        score -= cjk_ratio * 0.45;
        score -= replacement_count as f64 * 0.25;
    } else if candidate.language == "zh" {
        score += cjk_ratio * 0.65;
        score += (digit_count.min(6) as f64) * 0.015;
        score -= ascii_ratio * 0.3;
        score -= replacement_count as f64 * 0.25;
    }

    score.clamp(0.0, 1.5)
}

fn score_candidate_text(text: &str, language: &str) -> f64 {
    score_candidate(&LocalTranscriptCandidate {
        text: text.to_string(),
        language: language.to_string(),
        confidence: 0.62,
    })
    .clamp(0.0, 0.98)
}

fn normalized_local_language_hint(language: &str) -> Option<&'static str> {
    if language.starts_with("zh") {
        Some("zh")
    } else if language.starts_with("ja") {
        Some("ja")
    } else if language.starts_with("ko") {
        Some("ko")
    } else if language.starts_with("en") {
        Some("en")
    } else {
        None
    }
}

fn count_cjk_chars(text: &str) -> usize {
    text.chars()
        .filter(|ch| {
            ('\u{4E00}'..='\u{9FFF}').contains(ch)
                || ('\u{3400}'..='\u{4DBF}').contains(ch)
                || ('\u{F900}'..='\u{FAFF}').contains(ch)
        })
        .count()
}

fn count_ascii_letters(text: &str) -> usize {
    text.chars().filter(|ch| ch.is_ascii_alphabetic()).count()
}

fn count_ascii_words(text: &str) -> usize {
    text.split_whitespace()
        .filter(|segment| segment.chars().any(|ch| ch.is_ascii_alphabetic()))
        .count()
}

fn sanitize_audio_extension(format: &str) -> String {
    match format.to_ascii_lowercase().as_str() {
        "m4a" => "m4a",
        "mp3" => "mp3",
        "wav" => "wav",
        "ogg" => "ogg",
        "webm" => "webm",
        _ => "wav",
    }
    .to_string()
}

fn audio_mime_type(format: &str) -> &'static str {
    match sanitize_audio_extension(format).as_str() {
        "m4a" => "audio/mp4",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "webm" => "audio/webm",
        _ => "audio/wav",
    }
}

fn normalized_language_for_cloud(language: &str) -> Option<String> {
    let trimmed = language.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        None
    } else {
        Some(trimmed.to_string())
    }
}

impl ASREngine {
    async fn transcribe_openai_compatible(
        &self,
        base_url: &str,
        api_key: &str,
        model: &str,
        audio_data: &[u8],
        format: &str,
    ) -> Result<TranscriptionResult> {
        let file_name = format!("aura-input.{}", sanitize_audio_extension(format));
        let part = reqwest::multipart::Part::bytes(audio_data.to_vec())
            .file_name(file_name)
            .mime_str(audio_mime_type(format))
            .map_err(|error| AuraError::Processing {
                message: format!("Invalid audio mime type: {}", error),
                error_code: "ASR_103".to_string(),
            })?;

        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", model.to_string());

        if let Some(language) = normalized_language_for_cloud(&self.inner.language) {
            form = form.text("language", language);
        }

        let response = self
            .inner
            .client
            .post(format!("{}/audio/transcriptions", base_url))
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuraError::Processing {
                message: format!("Cloud ASR API error: {}", response.status()),
                error_code: "ASR_104".to_string(),
            });
        }

        #[derive(serde::Deserialize)]
        struct CloudTranscriptResponse {
            text: String,
            #[serde(default)]
            language: Option<String>,
        }

        let result: CloudTranscriptResponse = response.json().await?;
        let normalized_text = normalize_to_simplified_chinese(&result.text);

        Ok(TranscriptionResult {
            text: normalized_text.clone(),
            language: result.language.unwrap_or_else(|| self.inner.language.clone()),
            confidence: 0.9,
            segments: vec![TranscriptionSegment {
                text: normalized_text,
                start: 0.0,
                end: 0.0,
                confidence: 0.9,
            }],
        })
    }

    async fn transcribe_deepgram(
        &self,
        base_url: &str,
        api_key: &str,
        model: &str,
        audio_data: &[u8],
        format: &str,
    ) -> Result<TranscriptionResult> {
        #[derive(serde::Deserialize)]
        struct DeepgramResponse {
            results: DeepgramResults,
        }

        #[derive(serde::Deserialize)]
        struct DeepgramResults {
            channels: Vec<DeepgramChannel>,
        }

        #[derive(serde::Deserialize)]
        struct DeepgramChannel {
            alternatives: Vec<DeepgramAlternative>,
        }

        #[derive(serde::Deserialize)]
        struct DeepgramAlternative {
            transcript: String,
            #[serde(default)]
            confidence: f64,
        }

        let language = normalized_language_for_cloud(&self.inner.language);
        let mut url = format!("{}/listen?model={}", base_url.trim_end_matches('/'), model);
        if let Some(lang) = language.as_ref() {
            url.push_str(&format!("&language={}", lang));
        }

        let response = self
            .inner
            .client
            .post(url)
            .header("Authorization", format!("Token {}", api_key))
            .header("Content-Type", audio_mime_type(format))
            .body(audio_data.to_vec())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuraError::Processing {
                message: format!("Deepgram API error: {}", response.status()),
                error_code: "ASR_204".to_string(),
            });
        }

        let result: DeepgramResponse = response.json().await?;
        let transcript = result
            .results
            .channels
            .into_iter()
            .next()
            .and_then(|channel| channel.alternatives.into_iter().next())
            .unwrap_or(DeepgramAlternative {
                transcript: String::new(),
                confidence: 0.0,
            });

        let normalized_text = normalize_to_simplified_chinese(&transcript.transcript);

        Ok(TranscriptionResult {
            text: normalized_text.clone(),
            language: language.unwrap_or_else(|| self.inner.language.clone()),
            confidence: transcript.confidence,
            segments: vec![TranscriptionSegment {
                text: normalized_text,
                start: 0.0,
                end: 0.0,
                confidence: transcript.confidence,
            }],
        })
    }

    async fn transcribe_assemblyai(
        &self,
        base_url: &str,
        api_key: &str,
        model: &str,
        audio_data: &[u8],
        format: &str,
    ) -> Result<TranscriptionResult> {
        #[derive(serde::Deserialize)]
        struct AssemblyUploadResponse {
            upload_url: String,
        }

        #[derive(serde::Serialize)]
        struct AssemblyTranscriptRequest {
            audio_url: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            speech_model: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            language_code: Option<String>,
        }

        #[derive(serde::Deserialize)]
        struct AssemblyTranscriptResponse {
            id: String,
        }

        #[derive(serde::Deserialize)]
        struct AssemblyTranscriptStatus {
            status: String,
            text: Option<String>,
            error: Option<String>,
        }

        let base = base_url.trim_end_matches('/');
        let upload_resp = self
            .inner
            .client
            .post(format!("{}/upload", base))
            .header("authorization", api_key)
            .header("Content-Type", audio_mime_type(format))
            .body(audio_data.to_vec())
            .send()
            .await?;

        if !upload_resp.status().is_success() {
            return Err(AuraError::Processing {
                message: format!("AssemblyAI upload error: {}", upload_resp.status()),
                error_code: "ASR_304".to_string(),
            });
        }

        let upload: AssemblyUploadResponse = upload_resp.json().await?;
        let language_code = normalized_language_for_cloud(&self.inner.language);
        let transcript_request = AssemblyTranscriptRequest {
            audio_url: upload.upload_url,
            speech_model: if model.trim().is_empty() { None } else { Some(model.to_string()) },
            language_code,
        };

        let transcript_resp = self
            .inner
            .client
            .post(format!("{}/transcript", base))
            .header("authorization", api_key)
            .json(&transcript_request)
            .send()
            .await?;

        if !transcript_resp.status().is_success() {
            return Err(AuraError::Processing {
                message: format!("AssemblyAI transcript error: {}", transcript_resp.status()),
                error_code: "ASR_305".to_string(),
            });
        }

        let transcript: AssemblyTranscriptResponse = transcript_resp.json().await?;
        let poll_url = format!("{}/transcript/{}", base, transcript.id);
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts > 30 {
                return Err(AuraError::Processing {
                    message: "AssemblyAI transcription timeout".to_string(),
                    error_code: "ASR_306".to_string(),
                });
            }

            let status_resp = self
                .inner
                .client
                .get(&poll_url)
                .header("authorization", api_key)
                .send()
                .await?;

            if !status_resp.status().is_success() {
                return Err(AuraError::Processing {
                    message: format!("AssemblyAI status error: {}", status_resp.status()),
                    error_code: "ASR_307".to_string(),
                });
            }

            let status: AssemblyTranscriptStatus = status_resp.json().await?;
            match status.status.as_str() {
                "completed" => {
                    let text = status.text.unwrap_or_default();
                    let normalized_text = normalize_to_simplified_chinese(&text);
                    return Ok(TranscriptionResult {
                        text: normalized_text.clone(),
                        language: self.inner.language.clone(),
                        confidence: 0.88,
                        segments: vec![TranscriptionSegment {
                            text: normalized_text,
                            start: 0.0,
                            end: 0.0,
                            confidence: 0.88,
                        }],
                    });
                }
                "error" => {
                    return Err(AuraError::Processing {
                        message: status.error.unwrap_or_else(|| "AssemblyAI failed".to_string()),
                        error_code: "ASR_308".to_string(),
                    });
                }
                _ => {
                    sleep(TokioDuration::from_millis(900)).await;
                }
            }
        }
    }

    async fn transcribe_azure(
        &self,
        base_url: &str,
        api_key: &str,
        audio_data: &[u8],
        format: &str,
    ) -> Result<TranscriptionResult> {
        #[derive(serde::Deserialize)]
        struct AzureResponse {
            #[serde(rename = "DisplayText")]
            display_text: Option<String>,
            #[serde(rename = "NBest")]
            nbest: Option<Vec<AzureNBest>>,
        }

        #[derive(serde::Deserialize)]
        struct AzureNBest {
            #[serde(rename = "Display")]
            display: Option<String>,
            #[serde(rename = "Confidence")]
            confidence: Option<f64>,
        }

        let language = azure_language_code(&self.inner.language);
        let mut url = format!(
            "{}/speech/recognition/conversation/cognitiveservices/v1?format=detailed",
            base_url.trim_end_matches('/')
        );
        if let Some(lang) = language.as_ref() {
            url.push_str(&format!("&language={}", lang));
        }

        let response = self
            .inner
            .client
            .post(url)
            .header("Ocp-Apim-Subscription-Key", api_key)
            .header("Content-Type", audio_mime_type(format))
            .body(audio_data.to_vec())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AuraError::Processing {
                message: format!("Azure Speech API error: {}", response.status()),
                error_code: "ASR_404".to_string(),
            });
        }

        let result: AzureResponse = response.json().await?;
        let (text, confidence) = if let Some(nbest) = result.nbest.as_ref().and_then(|list| list.first()) {
            (
                nbest.display.clone().unwrap_or_default(),
                nbest.confidence.unwrap_or(0.9),
            )
        } else {
            (result.display_text.unwrap_or_default(), 0.9)
        };

        let normalized_text = normalize_to_simplified_chinese(&text);

        Ok(TranscriptionResult {
            text: normalized_text.clone(),
            language: language.unwrap_or_else(|| self.inner.language.clone()),
            confidence,
            segments: vec![TranscriptionSegment {
                text: normalized_text,
                start: 0.0,
                end: 0.0,
                confidence,
            }],
        })
    }

    async fn transcribe_google(
        &self,
        base_url: &str,
        api_key: &str,
        audio_data: &[u8],
        format: &str,
    ) -> Result<TranscriptionResult> {
        #[derive(serde::Serialize)]
        struct GoogleConfig {
            language_code: String,
            encoding: String,
            sample_rate_hertz: i32,
        }

        #[derive(serde::Serialize)]
        struct GoogleAudio {
            content: String,
        }

        #[derive(serde::Serialize)]
        struct GoogleRequest {
            config: GoogleConfig,
            audio: GoogleAudio,
        }

        #[derive(serde::Deserialize)]
        struct GoogleResponse {
            results: Vec<GoogleResult>,
        }

        #[derive(serde::Deserialize)]
        struct GoogleResult {
            alternatives: Vec<GoogleAlternative>,
        }

        #[derive(serde::Deserialize)]
        struct GoogleAlternative {
            transcript: String,
            #[serde(default)]
            confidence: f64,
        }

        let language = google_language_code(&self.inner.language).unwrap_or_else(|| "en-US".to_string());
        let content = base64::engine::general_purpose::STANDARD.encode(audio_data);
        let request = GoogleRequest {
            config: GoogleConfig {
                language_code: language.clone(),
                encoding: google_audio_encoding(format).to_string(),
                sample_rate_hertz: 16000,
            },
            audio: GoogleAudio { content },
        };

        let url = format!("{}/speech:recognize?key={}", base_url.trim_end_matches('/'), api_key);
        let response = self.inner.client.post(url).json(&request).send().await?;

        if !response.status().is_success() {
            return Err(AuraError::Processing {
                message: format!("Google Speech API error: {}", response.status()),
                error_code: "ASR_504".to_string(),
            });
        }

        let result: GoogleResponse = response.json().await?;
        let alternative = result
            .results
            .into_iter()
            .next()
            .and_then(|result| result.alternatives.into_iter().next())
            .unwrap_or(GoogleAlternative {
                transcript: String::new(),
                confidence: 0.0,
            });

        let normalized_text = normalize_to_simplified_chinese(&alternative.transcript);

        Ok(TranscriptionResult {
            text: normalized_text.clone(),
            language,
            confidence: alternative.confidence,
            segments: vec![TranscriptionSegment {
                text: normalized_text,
                start: 0.0,
                end: 0.0,
                confidence: alternative.confidence,
            }],
        })
    }
}

fn azure_language_code(language: &str) -> Option<String> {
    let trimmed = language.trim().to_ascii_lowercase();
    if trimmed.is_empty() || trimmed == "auto" {
        None
    } else if trimmed.starts_with("zh") {
        Some("zh-CN".to_string())
    } else if trimmed.starts_with("en") {
        Some("en-US".to_string())
    } else {
        Some(trimmed)
    }
}

fn google_language_code(language: &str) -> Option<String> {
    let trimmed = language.trim().to_ascii_lowercase();
    if trimmed.is_empty() || trimmed == "auto" {
        None
    } else if trimmed.starts_with("zh") {
        Some("zh-CN".to_string())
    } else if trimmed.starts_with("en") {
        Some("en-US".to_string())
    } else {
        Some(trimmed)
    }
}

fn google_audio_encoding(format: &str) -> &'static str {
    match sanitize_audio_extension(format).as_str() {
        "wav" => "LINEAR16",
        "mp3" => "MP3",
        "ogg" => "OGG_OPUS",
        "webm" => "WEBM_OPUS",
        "m4a" => "ENCODING_UNSPECIFIED",
        _ => "ENCODING_UNSPECIFIED",
    }
}

fn local_model_path(model_name: &str) -> PathBuf {
    let normalized = normalize_local_model_name(model_name);
    let file_name = if normalized.contains("tiny") {
        "ggml-tiny.bin"
    } else if normalized.contains("base") {
        "ggml-base.bin"
    } else if normalized.contains("small") {
        "ggml-small.bin"
    } else if normalized.contains("medium") {
        "ggml-medium.bin"
    } else if normalized.contains("large-v3") {
        "ggml-large-v3.bin"
    } else if normalized.contains("large") {
        "ggml-large-v3.bin"
    } else if normalized.ends_with(".bin") {
        model_name
    } else {
        "ggml-base.bin"
    };

    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".aura")
        .join("models")
        .join(file_name)
}

fn normalize_local_model_name(model_name: &str) -> &str {
    let normalized = model_name.to_ascii_lowercase();
    if normalized.contains("tiny") {
        "whisper-tiny"
    } else if normalized.contains("base") {
        "whisper-base"
    } else if normalized.contains("small") {
        "whisper-small"
    } else if normalized.contains("medium") {
        "whisper-medium"
    } else if normalized.contains("large") {
        "whisper-large-v3"
    } else {
        "whisper-base"
    }
}

fn local_model_expected_size_mb(model_name: &str) -> u32 {
    match normalize_local_model_name(model_name) {
        "whisper-tiny" => 75,
        "whisper-base" => 142,
        "whisper-small" => 466,
        "whisper-medium" => 1500,
        "whisper-large-v3" => 3100,
        _ => 142,
    }
}
