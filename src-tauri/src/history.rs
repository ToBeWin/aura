use crate::errors::{AuraError, Result};
use crate::models::HistoryEntry;
use std::path::PathBuf;

const MAX_HISTORY_ITEMS: usize = 50;

fn history_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".aura")
        .join("history.json")
}

pub fn load_history() -> Result<Vec<HistoryEntry>> {
    let path = history_path();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path).map_err(|error| AuraError::Processing {
        message: format!("Cannot read history file: {}", error),
        error_code: "HISTORY_001".to_string(),
    })?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&content).map_err(|error| AuraError::Processing {
        message: format!("Cannot parse history file: {}", error),
        error_code: "HISTORY_002".to_string(),
    })
}

pub fn append_history(entry: HistoryEntry) -> Result<Vec<HistoryEntry>> {
    let mut history = load_history()?;
    history.insert(0, entry);
    history.truncate(MAX_HISTORY_ITEMS);
    save_history(&history)?;
    Ok(history)
}

fn save_history(entries: &[HistoryEntry]) -> Result<()> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| AuraError::Processing {
            message: format!("Cannot create history directory: {}", error),
            error_code: "HISTORY_003".to_string(),
        })?;
    }

    let content = serde_json::to_string_pretty(entries).map_err(|error| AuraError::Processing {
        message: format!("Cannot serialize history: {}", error),
        error_code: "HISTORY_004".to_string(),
    })?;

    std::fs::write(&path, content).map_err(|error| AuraError::Processing {
        message: format!("Cannot write history file: {}", error),
        error_code: "HISTORY_005".to_string(),
    })?;

    Ok(())
}
