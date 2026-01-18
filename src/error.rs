use std::fmt;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "Taskwarrior not found or not executable. Please install Taskwarrior 3.4.2 or higher."
    )]
    TaskwarriorNotFound,

    #[error("Taskwarrior version {0} is too old. Version 3.4.2 or higher is required.")]
    TaskwarriorVersionTooOld(String),

    #[error("Failed to execute Taskwarrior command: {0}")]
    TaskwarriorCommandFailed(String),

    #[error("Failed to parse Taskwarrior output: {0}")]
    TaskwarriorParseFailed(String),

    #[error("Missing or malformed Habitica credentials in .taskrc. Please set habitica.user_id and habitica.api_key")]
    InvalidHabiticaCredentials,

    #[error("Habitica API error: {0}")]
    HabiticaApiError(String),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Failed to serialize/deserialize JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Invalid UUID: {0}")]
    InvalidUuid(#[from] uuid::Error),

    #[error("Invalid task status: {0}")]
    InvalidTaskStatus(String),

    #[error("Sync conflict: {0}")]
    SyncConflict(String),

    #[error("{0}")]
    Custom(String),
}

impl Error {
    /// Create a custom error with a message
    pub fn custom(msg: impl Into<String>) -> Self {
        Error::Custom(msg.into())
    }

    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Error::ConfigError(msg.into())
    }

    /// Check if this is a user-facing error that should be shown without
    /// backtrace
    pub const fn is_user_error(&self) -> bool {
        matches!(
            self,
            Error::TaskwarriorNotFound
                | Error::TaskwarriorVersionTooOld(_)
                | Error::InvalidHabiticaCredentials
                | Error::ConfigError(_)
        )
    }
}

/// Helper trait for converting Results with context
pub trait ResultExt<T> {
    fn context(self, msg: &str) -> Result<T>;
}

impl<T, E: fmt::Display> ResultExt<T> for std::result::Result<T, E> {
    fn context(self, msg: &str) -> Result<T> {
        self.map_err(|e| Error::custom(format!("{}: {}", msg, e)))
    }
}
