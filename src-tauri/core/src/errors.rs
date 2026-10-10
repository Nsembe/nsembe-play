use serde::Serialize;

/// Error type shared by every part of the engine.
///
/// Errors are converted to a stable `code` + human readable `message` pair before
/// they reach a client, so UI code never has to parse error strings.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Database migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("File system error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    MissingFile(String),
    #[error("{0}")]
    ToolUnavailable(String),
    #[error("{0}")]
    Media(String),
    #[error("{0}")]
    Player(String),
    #[error("{0}")]
    Other(String),
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::Database(_) | CoreError::Migration(_) => "database",
            CoreError::Io(_) => "io",
            CoreError::NotFound(_) => "not_found",
            CoreError::InvalidInput(_) => "invalid_input",
            CoreError::MissingFile(_) => "missing_file",
            CoreError::ToolUnavailable(_) => "tool_unavailable",
            CoreError::Media(_) => "media",
            CoreError::Player(_) => "player",
            CoreError::Other(_) => "other",
        }
    }

    pub fn not_found(what: impl Into<String>) -> Self {
        CoreError::NotFound(what.into())
    }

    pub fn invalid(msg: impl Into<String>) -> Self {
        CoreError::InvalidInput(msg.into())
    }
}

/// Serializable form handed to clients.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

impl From<&CoreError> for ErrorPayload {
    fn from(e: &CoreError) -> Self {
        ErrorPayload { code: e.code().to_string(), message: e.to_string() }
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;
