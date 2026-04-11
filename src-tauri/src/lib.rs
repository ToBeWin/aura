pub mod errors;
pub mod models;
pub mod storage;
pub mod llm;
pub mod processing;
pub mod settings;
pub mod history;
pub mod text;
mod core;
pub mod asr;
mod learning;
pub mod monitoring;

use core::AuraCore;
use asr::ASREngine;
use learning::CorrectionManager;
use llm::LocalLLM;
use storage::LocalVectorDB;
use models::{AppliedRule, CorrectionRecord, HistoryEntry, LLMProviderSettings, LocalASRModelStatus, ProviderMode, ProviderSettings, UserContext};
use monitoring::ResourceMonitor;
use history::{append_history, load_history};
use settings::{
    aura_data_dir,
    context_db_path as default_context_db_path,
    load_provider_settings,
    normalize_provider_settings,
    save_provider_settings,
    settings_path,
    vector_db_path as default_vector_db_path,
};
use text::normalize_to_simplified_chinese;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;
use tauri::PhysicalPosition;
use tauri::PhysicalSize;
use tauri::State;
use tauri::utils::config::Color;
use std::sync::{Mutex, OnceLock};
#[cfg(target_os = "macos")]
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
#[cfg(target_os = "macos")]
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

struct AppState {
    aura_core: tokio::sync::Mutex<Option<Arc<AuraCore>>>,
    asr_engine: tokio::sync::Mutex<Option<Arc<ASREngine>>>,
    correction_manager: tokio::sync::Mutex<Option<Arc<CorrectionManager>>>,
    provider_settings: tokio::sync::Mutex<ProviderSettings>,
    preferred_model_name: tokio::sync::Mutex<String>,
    db_path: tokio::sync::Mutex<String>,
    vector_db_path: tokio::sync::Mutex<String>,
    audio_buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    audio_sample_rate: Arc<std::sync::Mutex<u32>>,
    audio_channels: Arc<std::sync::Mutex<u16>>,
    audio_input_name: Arc<std::sync::Mutex<String>>,
}

const CAPSULE_WIDTH: u32 = 312;
const CAPSULE_HEIGHT: u32 = 84;
const CAPSULE_BOTTOM_OFFSET: i32 = 168;
const AURA_BUNDLE_ID: &str = "com.bingo.aura";

static LAST_FOCUSED_APP: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn last_focused_app() -> &'static Mutex<Option<String>> {
    LAST_FOCUSED_APP.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Serialize)]
struct VoiceResult {
    transcript: String,
    text: String,
    processing_time_ms: f64,
    confidence: f64,
    applied_rules: Vec<AppliedRule>,
    output_mode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PasteResult {
    text: String,
    delivered: bool,
    copied_to_clipboard: bool,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioInputStatus {
    device_name: String,
    sample_rate: u32,
    channels: u16,
    preferred_device_name: Option<String>,
    using_preferred_device: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderSettingsEnvelope {
    settings: ProviderSettings,
    settings_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalLLMModelCatalog {
    models: Vec<String>,
    reachable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticStatus {
    ready: bool,
    title: String,
    detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentDiagnostics {
    speech: DiagnosticStatus,
    refine: DiagnosticStatus,
    delivery: DiagnosticStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordHistoryRequest {
    transcript: String,
    refined: String,
    delivered: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalModelRequest {
    model_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProcessVoiceRequest {
    audio_data: Option<Vec<u8>>,
    audio_format: Option<String>,
    output_mode: Option<String>,
}

#[cfg(target_os = "macos")]
fn paste_into_focused_input_if_possible() -> std::io::Result<Option<bool>> {
    if let Some(bundle_id) = last_focused_app().lock().ok().and_then(|guard| guard.clone()) {
        if bundle_id != AURA_BUNDLE_ID {
            log::info!("[Aura] Auto-paste targeting app {}", bundle_id);
            let _ = std::process::Command::new("osascript")
                .args([
                    "-e",
                    &format!(
                        "tell application \"System Events\" to set frontmost of (first application process whose bundle identifier is \"{}\") to true\n delay 0.05",
                        bundle_id
                    ),
                ])
                .output();
            std::thread::sleep(std::time::Duration::from_millis(180));
        }
    }

    let script = r#"
tell application "System Events"
    try
        set frontApp to first application process whose frontmost is true
        set focusedElement to value of attribute "AXFocusedUIElement" of frontApp
        set roleName to value of attribute "AXRole" of focusedElement
        set editableValue to false
        try
            set editableValue to value of attribute "AXEditable" of focusedElement
        end try

        if editableValue is true or roleName is in {"AXTextField", "AXTextArea", "AXSearchField", "AXComboBox", "AXSecureTextField"} then
            keystroke "v" using command down
            return "pasted"
        end if
        return "not_editable"
    on error errMsg number errNum
        return "error:" & errNum & ":" & errMsg
    end try

    return "not_editable"
end tell
"#;

    // Fallback: attempt a direct paste keystroke even if AX role check fails.
    let fallback = r#"
tell application "System Events"
    try
        keystroke "v" using command down
        return "pasted"
    on error errMsg number errNum
        return "error:" & errNum & ":" & errMsg
    end try
end tell
"#;

    for attempt in 0..3 {
        let output = std::process::Command::new("osascript")
            .args(["-e", script])
            .output()?;

        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if !output.status.success() {
            if !stderr.is_empty() {
                log::warn!("[Aura] Auto-paste script error: {}", stderr);
                let lower = stderr.to_lowercase();
                if lower.contains("not authorized to send apple events")
                    || lower.contains("not authorised to send apple events")
                {
                    return Ok(None);
                }
            }
            return Ok(None);
        }

        if status.starts_with("error:") {
            log::warn!("[Aura] Auto-paste script result (attempt {}): {}", attempt + 1, status);
        } else {
            log::info!("[Aura] Auto-paste script result (attempt {}): {}", attempt + 1, status);
        }
        if status == "pasted" {
            return Ok(Some(true));
        }

        let fallback_output = std::process::Command::new("osascript")
            .args(["-e", fallback])
            .output()?;

        let fallback_status = String::from_utf8_lossy(&fallback_output.stdout).trim().to_string();
        let fallback_err = String::from_utf8_lossy(&fallback_output.stderr).trim().to_string();

        if !fallback_output.status.success() {
            if !fallback_err.is_empty() {
                log::warn!("[Aura] Auto-paste fallback error: {}", fallback_err);
            }
            return Ok(None);
        }

        if fallback_status.starts_with("error:") {
            log::warn!(
                "[Aura] Auto-paste fallback result (attempt {}): {}",
                attempt + 1,
                fallback_status
            );
        } else {
            log::info!(
                "[Aura] Auto-paste fallback result (attempt {}): {}",
                attempt + 1,
                fallback_status
            );
        }
        if fallback_status == "pasted" {
            return Ok(Some(true));
        }

        std::thread::sleep(std::time::Duration::from_millis(180));
    }

    Ok(Some(false))
}

#[cfg(target_os = "macos")]
fn capture_frontmost_app_bundle_id() -> Option<String> {
    let script = r#"
tell application "System Events"
    try
        set frontApp to first application process whose frontmost is true
        return bundle identifier of frontApp
    on error
        return ""
    end try
end tell
"#;

    let output = std::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let bundle_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if bundle_id.is_empty() {
        None
    } else {
        log::info!("[Aura] Captured frontmost app: {}", bundle_id);
        Some(bundle_id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReshapeTextRequest {
    text: String,
    output_mode: Option<String>,
}

async fn build_runtime_components(
    preferred_model_name: &str,
    db_path: &str,
    vector_db_path: &str,
    settings: &ProviderSettings,
    apply_resource_monitor: bool,
) -> Result<(Arc<AuraCore>, Arc<ASREngine>, Arc<CorrectionManager>), String> {
    let local_hint = if settings.llm.local_model.trim().is_empty() {
        preferred_model_name.to_string()
    } else {
        settings.llm.local_model.clone()
    };
    let selected_model = if apply_resource_monitor {
        select_optimal_model(&local_hint).await
    } else {
        local_hint
    };

    let effective_llm_settings = LLMProviderSettings {
        local_model: selected_model.clone(),
        ..settings.llm.clone()
    };
    let effective_asr_settings = crate::models::ASRProviderSettings {
        ..settings.asr.clone()
    };

    let core = AuraCore::new_with_settings(
        &effective_llm_settings,
        db_path.to_string(),
        vector_db_path.to_string(),
    )
    .map_err(|e| format!("Core: {:?}", e))?;
    let asr = ASREngine::from_settings(&effective_asr_settings);
    let llm = LocalLLM::from_settings(&effective_llm_settings);
    let vector_db = LocalVectorDB::new(vector_db_path.to_string())
        .map_err(|e| format!("VectorDB: {:?}", e))?;
    let corr = CorrectionManager::new(llm, vector_db);

    Ok((Arc::new(core), Arc::new(asr), Arc::new(corr)))
}

async fn rebuild_runtime(state: &State<'_, AppState>) -> Result<(), String> {
    let settings = state.provider_settings.lock().await.clone();
    let preferred_model_name = state.preferred_model_name.lock().await.clone();
    let db_path = state.db_path.lock().await.clone();
    let vector_db_path = state.vector_db_path.lock().await.clone();

    let (core, asr, corr) = build_runtime_components(
        &preferred_model_name,
        &db_path,
        &vector_db_path,
        &settings,
        false,
    )
    .await?;

    *state.aura_core.lock().await = Some(core);
    *state.asr_engine.lock().await = Some(asr);
    *state.correction_manager.lock().await = Some(corr);

    Ok(())
}

#[tauri::command]
async fn initialize_aura(
    model_name: String,
    db_path: String,
    vector_db_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let resolved_db_path = if db_path.trim().is_empty() || db_path.starts_with("./") {
        default_context_db_path().display().to_string()
    } else {
        db_path
    };
    let resolved_vector_db_path =
        if vector_db_path.trim().is_empty() || vector_db_path.starts_with("./") {
            default_vector_db_path().display().to_string()
        } else {
            vector_db_path
        };

    {
        let mut preferred = state.preferred_model_name.lock().await;
        *preferred = model_name;
    }
    {
        let mut db = state.db_path.lock().await;
        *db = resolved_db_path;
    }
    {
        let mut vector = state.vector_db_path.lock().await;
        *vector = resolved_vector_db_path;
    }

    let loaded_settings = load_provider_settings().unwrap_or_default();
    let mut effective_settings = loaded_settings;
    let migrated = normalize_provider_settings(&mut effective_settings);
    if !settings_path().exists() && matches!(effective_settings.llm.provider, ProviderMode::Local) {
        effective_settings.llm.local_model = select_optimal_model(&effective_settings.llm.local_model).await;
    }
    if migrated {
        let _ = save_provider_settings(&effective_settings);
    }
    {
        let mut current = state.provider_settings.lock().await;
        *current = effective_settings;
    }
    rebuild_runtime(&state).await?;

    Ok("initialized".to_string())
}

#[tauri::command]
async fn get_provider_settings(state: State<'_, AppState>) -> Result<ProviderSettingsEnvelope, String> {
    let current = state.provider_settings.lock().await.clone();
    Ok(ProviderSettingsEnvelope {
        settings: current,
        settings_path: settings::settings_path().display().to_string(),
    })
}

#[tauri::command]
async fn get_history_entries() -> Result<Vec<HistoryEntry>, String> {
    load_history().map_err(|e| format!("{:?}", e))
}

#[tauri::command]
async fn record_history_entry(
    request: RecordHistoryRequest,
    app: tauri::AppHandle,
) -> Result<Vec<HistoryEntry>, String> {
    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        transcript: request.transcript,
        refined: request.refined,
        delivered: request.delivered,
        timestamp: chrono::Utc::now().timestamp_millis(),
    };

    let history = append_history(entry.clone()).map_err(|e| format!("{:?}", e))?;
    let _ = app.emit("history_updated", &entry);
    Ok(history)
}

#[tauri::command]
async fn get_local_asr_model_status(
    request: LocalModelRequest,
) -> Result<LocalASRModelStatus, String> {
    ASREngine::local_model_status(&request.model_name).map_err(|e| format!("{:?}", e))
}

#[tauri::command]
async fn download_local_asr_model(
    request: LocalModelRequest,
) -> Result<LocalASRModelStatus, String> {
    let model_name = request.model_name;
    tokio::task::spawn_blocking(move || ASREngine::download_local_model(&model_name))
        .await
        .map_err(|e| format!("Download task failed: {:?}", e))?
        .map_err(|e| format!("{:?}", e))
}

#[tauri::command]
async fn update_provider_settings(
    mut settings: ProviderSettings,
    state: State<'_, AppState>,
) -> Result<ProviderSettingsEnvelope, String> {
    normalize_provider_settings(&mut settings);
    let preferred_model_name = state.preferred_model_name.lock().await.clone();
    let db_path = state.db_path.lock().await.clone();
    let vector_db_path = state.vector_db_path.lock().await.clone();
    let (core, asr, corr) = build_runtime_components(
        &preferred_model_name,
        &db_path,
        &vector_db_path,
        &settings,
        false,
    )
    .await?;
    save_provider_settings(&settings).map_err(|e| format!("{:?}", e))?;
    {
        let mut current = state.provider_settings.lock().await;
        *current = settings.clone();
    }
    *state.aura_core.lock().await = Some(core);
    *state.asr_engine.lock().await = Some(asr);
    *state.correction_manager.lock().await = Some(corr);
    Ok(ProviderSettingsEnvelope {
        settings,
        settings_path: settings::settings_path().display().to_string(),
    })
}

async fn select_optimal_model(preferred_model: &str) -> String {
    let monitor = ResourceMonitor::new();
    let suggested = monitor.suggest_model().await;
    let status = monitor.check_resources().await;
    if status.should_downgrade { suggested } else { preferred_model.to_string() }
}

#[tauri::command]
async fn process_voice(
    request: Option<ProcessVoiceRequest>,
    state: State<'_, AppState>,
) -> Result<VoiceResult, String> {
    let start = std::time::Instant::now();

    let asr = {
        let g = state.asr_engine.lock().await;
        g.as_ref().ok_or_else(|| "语音识别还没准备好".to_string())?.clone()
    };

    let selected_mode = request
        .as_ref()
        .and_then(|value| value.output_mode.clone())
        .unwrap_or_else(|| "note".to_string());

    let transcript = if let Some(ProcessVoiceRequest {
        audio_data: Some(audio_data),
        audio_format,
        ..
    }) = request
    {
        if audio_data.len() < 1024 {
            return Err("录音太短了，请至少连续说 2 秒".to_string());
        }

        log::info!("[Aura] {} input bytes from UI, transcribing…", audio_data.len());
        asr.transcribe_bytes(&audio_data, audio_format.as_deref().unwrap_or("webm"))
            .await
            .map_err(|e| format!("ASR: {:?}", e))?
    } else {
        // Grab & clear audio buffer from the global hotkey pipeline.
        let samples = {
            let mut buf = state.audio_buffer.lock().unwrap();
            let s = buf.clone();
            buf.clear();
            s
        };
        let sample_rate = *state.audio_sample_rate.lock().unwrap();

        if samples.len() < 800 {
            return Err("录音太短了，请至少连续说 2 秒".to_string());
        }

        log::info!(
            "[Aura] {} samples from global hotkey at {} Hz, transcribing…",
            samples.len(),
            sample_rate
        );

        let wav_path = std::env::temp_dir()
            .join(format!("aura_{}.wav", uuid::Uuid::new_v4()));
        write_wav_16bit(&samples, sample_rate.max(8_000), &wav_path)
            .map_err(|e| format!("WAV: {:?}", e))?;

        let transcript = asr
            .transcribe(&wav_path)
            .await
            .map_err(|e| format!("ASR: {:?}", e))?;
        let _ = std::fs::remove_file(&wav_path);
        transcript
    };

    let text = transcript.text.trim().to_string();
    if text.is_empty() {
        return Err("已经录到声音了，但没有识别出文字。请检查系统输入设备是不是你的麦克风".to_string());
    }
    if transcript.confidence < 0.55 {
        log::warn!(
            "[Aura] Rejecting low-confidence transcript language={} confidence={:.2} text={}",
            transcript.language,
            transcript.confidence,
            text
        );
        return Err("这次没有听清，请再说一遍".to_string());
    }
    log::info!(
        "[Aura] Transcript language={} confidence={:.2} text={}",
        transcript.language,
        transcript.confidence,
        text
    );

    // Refine
    let core = {
        let g = state.aura_core.lock().await;
        g.as_ref().ok_or_else(|| "Aura 还没准备好".to_string())?.clone()
    };
    let refined = match core.refine_simple(&text, "default", Some(&selected_mode)).await {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "[Aura] Refine failed after successful transcription, falling back to transcript: {:?}",
                error
            );
            core::SimpleRefine {
                text: text.clone(),
                confidence: transcript.confidence.max(0.6),
                applied_rules: vec![AppliedRule {
                    rule_type: "refine_error_fallback".to_string(),
                    from: "refine_error".to_string(),
                    to: text.clone(),
                }],
                output_mode: selected_mode.clone(),
            }
        }
    };
    log::info!(
        "[Aura] Refined output mode={} rules={} text={}",
        refined.output_mode,
        refined.applied_rules.len(),
        refined.text
    );

    let elapsed = start.elapsed().as_secs_f64();
    log::info!("[Aura] Done in {:.2}s", elapsed);

    Ok(VoiceResult {
        transcript: text,
        text: refined.text,
        processing_time_ms: elapsed * 1000.0,
        confidence: refined.confidence,
        applied_rules: refined.applied_rules,
        output_mode: refined.output_mode,
    })
}

#[tauri::command]
async fn get_audio_input_status(state: State<'_, AppState>) -> Result<AudioInputStatus, String> {
    let device_name = state.audio_input_name.lock().unwrap().clone();
    let sample_rate = *state.audio_sample_rate.lock().unwrap();
    let channels = *state.audio_channels.lock().unwrap();
    let preferred_device_name = std::env::var("AURA_INPUT_DEVICE_NAME").ok();
    let using_preferred_device = preferred_device_name
        .as_ref()
        .map(|preferred| !preferred.trim().is_empty() && device_name == *preferred)
        .unwrap_or(false);

    Ok(AudioInputStatus {
        device_name,
        sample_rate,
        channels,
        preferred_device_name,
        using_preferred_device,
    })
}

#[tauri::command]
async fn get_environment_diagnostics(
    state: State<'_, AppState>,
) -> Result<EnvironmentDiagnostics, String> {
    let settings = state.provider_settings.lock().await.clone();
    let device_name = state.audio_input_name.lock().unwrap().clone();
    let sample_rate = *state.audio_sample_rate.lock().unwrap();
    let channels = *state.audio_channels.lock().unwrap();

    let speech = match settings.asr.provider {
        ProviderMode::Local => {
            let model_status = ASREngine::local_model_status(&settings.asr.local_model)
                .map_err(|e| format!("{:?}", e))?;
            DiagnosticStatus {
                ready: model_status.downloaded && !device_name.trim().is_empty(),
                title: "Speech recognition".to_string(),
                detail: if model_status.downloaded {
                    format!(
                        "Local {} is ready on {} · {}Hz · {}ch",
                        model_status.model_name, device_name, sample_rate, channels
                    )
                } else {
                    format!(
                        "Local {} is not downloaded yet. Input device: {}",
                        model_status.model_name, device_name
                    )
                },
            }
        }
        ProviderMode::Cloud => {
            let configured = !settings.asr.cloud_api_key.trim().is_empty()
                && !settings.asr.cloud_base_url.trim().is_empty()
                && !settings.asr.cloud_model.trim().is_empty();
            DiagnosticStatus {
                ready: configured,
                title: "Speech recognition".to_string(),
                detail: if configured {
                    format!(
                        "Cloud ASR configured: {} ({})",
                        settings.asr.cloud_model, settings.asr.cloud_base_url
                    )
                } else {
                    "Cloud ASR is missing base URL, model, or API key".to_string()
                },
            }
        }
    };

    let refine = match settings.llm.provider {
        ProviderMode::Local => {
            let base_url = if settings.llm.local_base_url.trim().is_empty() {
                "http://127.0.0.1:11434".to_string()
            } else {
                settings.llm.local_base_url.trim_end_matches('/').to_string()
            };

            let client = reqwest::Client::builder()
                .connect_timeout(tokio::time::Duration::from_millis(450))
                .timeout(tokio::time::Duration::from_millis(1200))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            let reachable = client
                .get(format!("{}/api/tags", base_url))
                .send()
                .await
                .map(|response| response.status().is_success())
                .unwrap_or(false);

            DiagnosticStatus {
                ready: reachable,
                title: "Refine model".to_string(),
                detail: if reachable {
                    format!("Local {} is reachable via Ollama", settings.llm.local_model)
                } else {
                    format!(
                        "Local {} is not responding. Check Ollama at {}",
                        settings.llm.local_model, base_url
                    )
                },
            }
        }
        ProviderMode::Cloud => {
            let configured = !settings.llm.cloud_api_key.trim().is_empty()
                && !settings.llm.cloud_base_url.trim().is_empty()
                && !settings.llm.cloud_model.trim().is_empty();
            DiagnosticStatus {
                ready: configured,
                title: "Refine model".to_string(),
                detail: if configured {
                    format!(
                        "Cloud refine configured: {} ({})",
                        settings.llm.cloud_model, settings.llm.cloud_base_url
                    )
                } else {
                    "Cloud refine is missing base URL, model, or API key".to_string()
                },
            }
        }
    };

    #[cfg(target_os = "macos")]
    let delivery = {
        let permission_state = paste_into_focused_input_if_possible().ok().flatten();
        DiagnosticStatus {
            ready: permission_state.is_some(),
            title: "Auto-paste".to_string(),
            detail: match permission_state {
                Some(true) => "Accessibility permission is available, and a text field is focused".to_string(),
                Some(false) => "Accessibility permission is available. Focus a text input to verify auto-paste".to_string(),
                None => "Automation permission is missing (System Events). Aura will fall back to clipboard only".to_string(),
            },
        }
    };

    #[cfg(not(target_os = "macos"))]
    let delivery = DiagnosticStatus {
        ready: true,
        title: "Auto-paste".to_string(),
        detail: "Auto-paste diagnostics are currently optimized for macOS".to_string(),
    };

    Ok(EnvironmentDiagnostics {
        speech,
        refine,
        delivery,
    })
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelItem>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelItem {
    name: String,
}

#[tauri::command]
async fn get_local_llm_models(
    state: State<'_, AppState>,
) -> Result<LocalLLMModelCatalog, String> {
    let settings = state.provider_settings.lock().await.clone();
    let base_url = if settings.llm.local_base_url.trim().is_empty() {
        "http://127.0.0.1:11434".to_string()
    } else {
        settings.llm.local_base_url.trim_end_matches('/').to_string()
    };

    let client = reqwest::Client::builder()
        .connect_timeout(tokio::time::Duration::from_millis(500))
        .timeout(tokio::time::Duration::from_secs(2))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let response = client
        .get(format!("{}/api/tags", base_url))
        .send()
        .await;

    let fallback = vec![
        "qwen3.5:2b".to_string(),
        "qwen2.5:7b".to_string(),
        "llama3.2:3b".to_string(),
        "gemma3:4b".to_string(),
        "mistral:7b".to_string(),
    ];

    match response {
        Ok(response) if response.status().is_success() => {
            let body: OllamaTagsResponse = response.json().await.map_err(|e| format!("{:?}", e))?;
            let mut models = body
                .models
                .into_iter()
                .map(|item| item.name)
                .collect::<Vec<_>>();
            if models.is_empty() {
                models = fallback;
            }
            models.sort();
            models.dedup();
            Ok(LocalLLMModelCatalog {
                models,
                reachable: true,
            })
        }
        _ => Ok(LocalLLMModelCatalog {
            models: fallback,
            reachable: false,
        }),
    }
}

#[tauri::command]
async fn reshape_text(
    request: ReshapeTextRequest,
    state: State<'_, AppState>,
) -> Result<VoiceResult, String> {
    let start = std::time::Instant::now();
    let selected_mode = request.output_mode.unwrap_or_else(|| "note".to_string());
    let transcript = request.text.trim().to_string();

    if transcript.is_empty() {
        return Err("没有可处理的文本".to_string());
    }

    let core = {
        let g = state.aura_core.lock().await;
        g.as_ref().ok_or_else(|| "Aura 还没准备好".to_string())?.clone()
    };
    let refined = core
        .refine_simple(&transcript, "default", Some(&selected_mode))
        .await
        .map_err(|e| format!("Refine: {:?}", e))?;

    let elapsed = start.elapsed().as_secs_f64();

    Ok(VoiceResult {
        transcript,
        text: refined.text,
        processing_time_ms: elapsed * 1000.0,
        confidence: refined.confidence,
        applied_rules: refined.applied_rules,
        output_mode: refined.output_mode,
    })
}

/// Set clipboard + platform-specific paste shortcut
#[tauri::command]
async fn type_text(text: String) -> Result<PasteResult, String> {
    let delivered_text = normalize_to_simplified_chinese(&text);
    log::info!(
        "[Aura] Preparing delivery for {} chars",
        delivered_text.chars().count()
    );
    let mut cb = arboard::Clipboard::new()
        .map_err(|e| format!("Clipboard: {:?}", e))?;
    cb.set_text(&delivered_text)
        .map_err(|e| format!("Set clipboard: {:?}", e))?;

    let _ = tokio::time::Duration::from_millis(60);

    #[cfg(target_os = "macos")]
    {
        tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;
        if send_paste_shortcut().is_ok() {
            log::info!("[Aura] Delivery: pasted via CGEvent");
            return Ok(PasteResult {
                text: delivered_text,
                delivered: true,
                copied_to_clipboard: true,
                message: "Pasted into the active input.".to_string(),
            });
        }

        log::warn!("[Aura] Delivery: clipboard only because CGEvent paste failed");
        return Ok(PasteResult {
            text: delivered_text,
            delivered: false,
            copied_to_clipboard: true,
            message: "Copied to clipboard. Enable Accessibility permission for Aura and focus a text input.".to_string(),
        });
    }

    #[cfg(target_os = "windows")]
    {
        tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;
        use enigo::{Enigo, Direction, Key};
        let mut enigo = Enigo::new(&Default::default())
            .map_err(|e: enigo::EnigoError| format!("{:?}", e))?;
        if enigo.key(Key::Unicode('v'), Direction::Click).is_err() {
            return Ok(PasteResult {
                text: delivered_text,
                delivered: false,
                copied_to_clipboard: true,
                message: "Copied to clipboard. Auto-paste did not complete.".to_string(),
            });
        }

        return Ok(PasteResult {
            text: delivered_text,
            delivered: true,
            copied_to_clipboard: true,
            message: "Pasted into the active app.".to_string(),
        });
    }

    #[cfg(target_os = "linux")]
    {
        if std::process::Command::new("xdotool")
            .args(["key", "ctrl+v"]).output().is_err() {
            return Ok(PasteResult {
                text: delivered_text,
                delivered: false,
                copied_to_clipboard: true,
                message: "Copied to clipboard. Install xdotool to enable auto-paste.".to_string(),
            });
        }

        return Ok(PasteResult {
            text: delivered_text,
            delivered: true,
            copied_to_clipboard: true,
            message: "Pasted into the active app.".to_string(),
        });
    }

    #[allow(unreachable_code)]
    Ok(PasteResult {
        text: delivered_text,
        delivered: false,
        copied_to_clipboard: true,
        message: "Copied to clipboard.".to_string(),
    })
}

#[cfg(target_os = "macos")]
fn send_paste_shortcut() -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|_| "CGEventSource init failed".to_string())?;
    let keycode_v: CGKeyCode = 0x09;
    let mut key_down = CGEvent::new_keyboard_event(source, keycode_v, true)
        .map_err(|_| "Failed to create key down event".to_string())?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);
    key_down.post(CGEventTapLocation::HID);

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|_| "CGEventSource init failed".to_string())?;
    let key_up = CGEvent::new_keyboard_event(source, keycode_v, false)
        .map_err(|_| "Failed to create key up event".to_string())?;
    key_up.post(CGEventTapLocation::HID);
    Ok(())
}

#[tauri::command]
async fn save_correction(
    user_id: String, original: String, corrected: String,
    state: State<'_, AppState>,
) -> Result<CorrectionRecord, String> {
    let mgr = {
        let g = state.correction_manager.lock().await;
        g.as_ref().ok_or_else(|| "Not ready".to_string())?.clone()
    };
    mgr.save_correction(&user_id, &original, &corrected, Default::default())
        .await.map_err(|e| format!("{:?}", e))
}

#[tauri::command]
async fn get_corrections(
    user_id: String, state: State<'_, AppState>,
) -> Result<Vec<CorrectionRecord>, String> {
    let mgr = {
        let g = state.correction_manager.lock().await;
        g.as_ref().ok_or_else(|| "Not ready".to_string())?.clone()
    };
    mgr.retrieve_corrections(&user_id, "", 100)
        .await.map_err(|e| format!("{:?}", e))
}

#[tauri::command]
async fn get_user_context(
    user_id: String, state: State<'_, AppState>,
) -> Result<UserContext, String> {
    let core = {
        let g = state.aura_core.lock().await;
        g.as_ref().ok_or_else(|| "Not ready".to_string())?.clone()
    };
    core.context_store.get_context(&user_id)
        .map_err(|e| format!("{:?}", e))
}

#[tauri::command]
async fn update_user_context(
    context: UserContext, state: State<'_, AppState>,
) -> Result<(), String> {
    let core = {
        let g = state.aura_core.lock().await;
        g.as_ref().ok_or_else(|| "Not ready".to_string())?.clone()
    };
    core.context_store.save_user_context(&context)
        .map_err(|e| format!("{:?}", e))
}

fn write_wav_16bit(samples: &[f32], sample_rate: u32, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use hound::{WavSpec, WavWriter, SampleFormat};
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)?;
    for &s in samples {
        let v = (s.max(-1.0).min(1.0) * i16::MAX as f32) as i16;
        writer.write_sample(v)?;
    }
    writer.finalize()?;
    Ok(())
}

fn setup_global_hotkeys(
    app: &tauri::AppHandle,
    _audio_buf: Arc<std::sync::Mutex<Vec<f32>>>,
    audio_sample_rate: Arc<std::sync::Mutex<u32>>,
    audio_channels: Arc<std::sync::Mutex<u16>>,
    audio_input_name: Arc<std::sync::Mutex<String>>,
) {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };
    use cpal::{
        traits::{DeviceTrait, HostTrait},
    };

    // Global shortcut: Option+Shift+Space
    let shortcut = Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::Space);

    // We need a shared recording flag and a shared cpal stream
    let is_recording = Arc::new(std::sync::Mutex::new(false));
    let app_handle_for_plugin = app.clone();
    let app_handle_for_registration = app.clone();
    let app_handle_for_emit = app.clone();
    let app_handle_for_window = app.clone();

    if let Err(e) = app_handle_for_plugin.clone().plugin(
        tauri_plugin_global_shortcut::Builder::new().with_handler(move |_app, _id, event| {
            if !matches!(event.state, ShortcutState::Pressed) {
                return;
            }

            let mut rec = is_recording.lock().unwrap();
            if *rec {
                *rec = false;
                let _ = app_handle_for_emit.emit("recording_level", &serde_json::json!({ "level": 0.0 }));
                let _ = app_handle_for_emit.emit("voice_done", &serde_json::json!({}));
                return;
            }

            *rec = true;

            if let Some(window) = app_handle_for_window.get_webview_window("capsule") {
                if let Some(bundle_id) = capture_frontmost_app_bundle_id() {
                    if let Ok(mut guard) = last_focused_app().lock() {
                        *guard = Some(bundle_id);
                    }
                }
                let _ = window.set_always_on_top(true);
                let _ = window.set_background_color(Some(Color(0, 0, 0, 0)));
                let _ = window.set_shadow(false);
                let _ = window.set_size(PhysicalSize::new(CAPSULE_WIDTH, CAPSULE_HEIGHT));
                let monitor = if let Ok(cursor) = app_handle_for_window.cursor_position() {
                    app_handle_for_window
                        .monitor_from_point(cursor.x, cursor.y)
                        .ok()
                        .flatten()
                } else {
                    None
                };
                let monitor = monitor.or_else(|| window.current_monitor().ok().flatten());
                if let Some(monitor) = monitor {
                    let monitor_size = monitor.size();
                    let monitor_pos = monitor.position();
                    let x = monitor_pos.x + ((monitor_size.width as i32 - CAPSULE_WIDTH as i32) / 2);
                    let y = monitor_pos.y
                        + monitor_size.height as i32
                        - CAPSULE_HEIGHT as i32
                        - CAPSULE_BOTTOM_OFFSET;
                    let _ = window.set_position(PhysicalPosition::new(x, y));
                }
                let should_show = window.is_visible().ok().map(|v| !v).unwrap_or(true);
                if should_show {
                    let _ = window.show();
                }
                // Keep the previously active app focused so auto-paste can still
                // target the user's current text field after the pipeline finishes.
            }

            let _ = app_handle_for_emit.emit("recording_started", &serde_json::json!({}));
            let _ = app_handle_for_emit.emit("recording_level", &serde_json::json!({ "level": 0.0 }));
        }).build()
    ) {
        log::error!("global-shortcut plugin error: {:?}", e);
    }

    // Register the shortcut
    if let Err(e) = app_handle_for_registration.global_shortcut().register(shortcut) {
        log::warn!("Failed to register global shortcut: {:?}", e);
    }

    // Probe the preferred/default microphone once for settings UI.
    // We no longer keep a CPAL input stream alive for the whole app lifetime,
    // because that makes macOS think Aura is actively using the microphone
    // even when the user has not started a recording.
    let host = cpal::default_host();
    let preferred_device_name = std::env::var("AURA_INPUT_DEVICE_NAME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let device = preferred_device_name
        .as_ref()
        .and_then(|preferred| {
            host.input_devices().ok()?.find(|candidate| {
                candidate
                    .name()
                    .map(|name| name == *preferred || name.contains(preferred))
                    .unwrap_or(false)
            })
        })
        .or_else(|| host.default_input_device())
        .expect("No audio input device");

    let device_name = device.name().unwrap_or_else(|_| "Unknown microphone".to_string());
    {
        let mut current_name = audio_input_name.lock().unwrap();
        *current_name = device_name.clone();
    }
    log::info!("[Audio] Using device: {}", device_name);

    let config = device.default_input_config().expect("Default input config");
    let stream_sample_rate = config.sample_rate();
    let stream_channel_count = config.channels();
    {
        let mut sample_rate = audio_sample_rate.lock().unwrap();
        *sample_rate = stream_sample_rate;
    }
    {
        let mut channels = audio_channels.lock().unwrap();
        *channels = stream_channel_count;
    }
    log::info!("[Audio] Sample rate: {}, channels: {}, format: {:?}",
               config.sample_rate(), config.channels(), config.sample_format());
}

#[tauri::command]
async fn hide_capsule_window(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("capsule")
        .ok_or_else(|| "Capsule window not found".to_string())?;
    window.hide().map_err(|e| format!("Hide capsule: {:?}", e))
}

#[tauri::command]
async fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .status()
            .map_err(|e| format!("Open settings: {:?}", e))?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("Accessibility settings shortcut is currently available on macOS only".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut logger = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    );
    logger.format_timestamp_millis();
    if let Some(log_file) = open_log_file() {
        logger.target(env_logger::Target::Pipe(Box::new(log_file)));
    }
    let _ = logger.try_init();

    let audio_buf: Arc<std::sync::Mutex<Vec<f32>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let audio_buf_for_setup = audio_buf.clone();
    let audio_sample_rate: Arc<std::sync::Mutex<u32>> = Arc::new(std::sync::Mutex::new(16_000));
    let audio_sample_rate_for_setup = audio_sample_rate.clone();
    let audio_channels: Arc<std::sync::Mutex<u16>> = Arc::new(std::sync::Mutex::new(1));
    let audio_channels_for_setup = audio_channels.clone();
    let audio_input_name: Arc<std::sync::Mutex<String>> =
        Arc::new(std::sync::Mutex::new("未检测到麦克风".to_string()));
    let audio_input_name_for_setup = audio_input_name.clone();
    let initial_provider_settings = load_provider_settings().unwrap_or_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            if let Some(window) = app.get_webview_window("capsule") {
                let _ = window.set_background_color(Some(Color(0, 0, 0, 0)));
                let _ = window.set_shadow(false);
                let _ = window.set_size(PhysicalSize::new(CAPSULE_WIDTH, CAPSULE_HEIGHT));
            }
            setup_global_hotkeys(
                app.handle(),
                audio_buf_for_setup,
                audio_sample_rate_for_setup,
                audio_channels_for_setup,
                audio_input_name_for_setup,
            );
            Ok(())
        })
        .manage(AppState {
            aura_core: tokio::sync::Mutex::new(None),
            asr_engine: tokio::sync::Mutex::new(None),
            correction_manager: tokio::sync::Mutex::new(None),
            provider_settings: tokio::sync::Mutex::new(initial_provider_settings),
            preferred_model_name: tokio::sync::Mutex::new("qwen3.5:2b".to_string()),
            db_path: tokio::sync::Mutex::new(default_context_db_path().display().to_string()),
            vector_db_path: tokio::sync::Mutex::new(default_vector_db_path().display().to_string()),
            audio_buffer: audio_buf,
            audio_sample_rate,
            audio_channels,
            audio_input_name,
        })
        .invoke_handler(tauri::generate_handler![
            initialize_aura,
            get_provider_settings,
            update_provider_settings,
            get_history_entries,
            record_history_entry,
            get_local_asr_model_status,
            download_local_asr_model,
            get_local_llm_models,
            process_voice,
            get_audio_input_status,
            get_environment_diagnostics,
            reshape_text,
            type_text,
            hide_capsule_window,
            open_accessibility_settings,
            get_log_path,
            open_logs_folder,
            save_correction,
            get_corrections,
            get_user_context,
            update_user_context,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn log_path() -> std::path::PathBuf {
    aura_data_dir().join("logs").join("aura.log")
}

fn open_log_file() -> Option<std::fs::File> {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()
}

#[tauri::command]
async fn get_log_path() -> Result<String, String> {
    Ok(log_path().display().to_string())
}

#[tauri::command]
async fn open_logs_folder() -> Result<(), String> {
    let path = log_path();
    let folder = path
        .parent()
        .ok_or_else(|| "Log folder not found".to_string())?;
    open::that(folder).map_err(|e| format!("Open logs folder failed: {:?}", e))
}
