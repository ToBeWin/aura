use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuraError {
    #[error("Input validation error: {message}")]
    InputValidation {
        message: String,
        error_code: String,
    },

    #[error("Processing error: {message}")]
    Processing {
        message: String,
        error_code: String,
    },

    #[error("Storage error: {message}")]
    #[allow(dead_code)]
    Storage {
        message: String,
        error_code: String,
    },

    #[error("Model load error: {message}")]
    #[allow(dead_code)]
    ModelLoad {
        message: String,
        error_code: String,
    },

    #[error("Timeout error: {message}")]
    #[allow(dead_code)]
    Timeout {
        message: String,
        error_code: String,
    },

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, AuraError>;
