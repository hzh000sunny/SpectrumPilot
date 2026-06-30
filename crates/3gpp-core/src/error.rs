use thiserror::Error;

pub type Result<T> = std::result::Result<T, GppError>;

#[derive(Debug, Error)]
pub enum GppError {
    #[error("invalid 3GPP URL: {0}")]
    InvalidUrl(String),

    #[error("failed to parse directory listing: {0}")]
    Parse(String),

    #[error("failed to read or write catalog file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to serialize catalog JSON: {0}")]
    Json(#[from] serde_json::Error),
}
