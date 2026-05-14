use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrmError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("entity not found: {0}")]
    NotFound(String),
}
