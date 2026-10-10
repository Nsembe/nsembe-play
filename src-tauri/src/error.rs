use nsembe_core::CoreError;
use serde::Serialize;

/// Error returned from every command. Serialized as `{ code, message }` so the UI
/// can show friendly messages and branch on `code`.
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl From<CoreError> for CommandError {
    fn from(e: CoreError) -> Self {
        if matches!(e, CoreError::Database(_) | CoreError::Migration(_) | CoreError::Io(_)) {
            log::error!("{e}");
        }
        CommandError { code: e.code().to_string(), message: e.to_string() }
    }
}

impl From<tauri::Error> for CommandError {
    fn from(e: tauri::Error) -> Self {
        log::error!("{e}");
        CommandError { code: "internal".into(), message: e.to_string() }
    }
}

pub type CmdResult<T> = Result<T, CommandError>;
