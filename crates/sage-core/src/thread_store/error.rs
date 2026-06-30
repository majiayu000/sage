use thiserror::Error;

pub type ThreadStoreResult<T> = Result<T, ThreadStoreError>;

#[derive(Debug, Error)]
pub enum ThreadStoreError {
    #[error("thread not found: {0}")]
    ThreadNotFound(String),
    #[error("thread already exists: {0}")]
    ThreadAlreadyExists(String),
    #[error("thread store schema version {found} is newer than supported version {supported}")]
    SchemaVersionMismatch { found: i64, supported: i64 },
    #[error("thread store is not writable: {0}")]
    NotWritable(String),
    #[error("invalid thread store input: {0}")]
    InvalidInput(String),
    #[error("invalid stored {table}.{field} for {id}: {message}")]
    InvalidStoredData {
        table: &'static str,
        field: &'static str,
        id: String,
        message: String,
    },
    #[error("legacy import failed for {path}:{line}: {message}")]
    LegacyImport {
        path: String,
        line: usize,
        message: String,
    },
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Time(#[from] chrono::ParseError),
}
