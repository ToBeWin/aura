#![allow(dead_code)]

use crate::errors::{AuraError, Result};
use std::path::PathBuf;

/// Audio recorder using platform-native recording APIs.
/// Planned future integrations: cpal (cross-platform), AVAudioRecorder (macOS).
#[derive(Clone)]
pub struct AudioRecorder {
    sample_rate: u32,
    channels: u16,
    output_dir: PathBuf,
}

impl AudioRecorder {
    pub fn new(output_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&output_dir);

        Self {
            sample_rate: 16000,
            channels: 1,
            output_dir,
        }
    }

    pub fn with_sample_rate(mut self, sample_rate: u32) -> Self {
        self.sample_rate = sample_rate;
        self
    }

    pub fn with_channels(mut self, channels: u16) -> Self {
        self.channels = channels;
        self
    }

    /// Start a recording session and return the handle.
    /// Actual audio capture via cpal/AVAudioRecorder is a planned future integration.
    pub async fn start_recording(&self) -> Result<RecordingSession> {
        let timestamp = chrono::Utc::now().timestamp() as u64;
        let output_path = self.output_dir.join(format!("recording_{}.wav", timestamp));
        log::info!("Starting audio recording to: {:?}", output_path);

        Ok(RecordingSession {
            output_path,
            is_recording: true,
            sample_rate: self.sample_rate,
            channels: self.channels,
            started_at: Some(timestamp),
        })
    }
}

pub struct RecordingSession {
    output_path: PathBuf,
    is_recording: bool,
    sample_rate: u32,
    channels: u16,
    started_at: Option<u64>,
}

impl RecordingSession {
    pub fn stop(&mut self) -> Result<PathBuf> {
        if !self.is_recording {
            return Err(AuraError::Processing {
                message: "Recording not started".to_string(),
                error_code: "AUDIO_001".to_string(),
            });
        }

        self.is_recording = false;
        log::info!("Stopped audio recording: {:?}", self.output_path);

        Ok(self.output_path.clone())
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    pub fn output_path(&self) -> &PathBuf {
        &self.output_path
    }

    pub fn duration(&self) -> f64 {
        self.started_at
            .map(|started_at| {
                (chrono::Utc::now().timestamp() as u64).saturating_sub(started_at) as f64
            })
            .unwrap_or(0.0)
    }
}
