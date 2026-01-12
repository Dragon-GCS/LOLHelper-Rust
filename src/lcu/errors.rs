use thiserror::Error;

pub type Result<T> = std::result::Result<T, LcuError>;

#[derive(Error, Debug)]
pub enum LcuError {
    #[error("Failed to find LCU process")]
    ClientNotFound,
    #[error("Failed to load LCU commands")]
    ClientCMDLineFailed,
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Response error: {0}")]
    ResponseError(String),
    #[error("Serialization failed: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("WebSocket error: {0}")]
    WebSocketError(String),
}

impl From<reqwest_websocket::Error> for LcuError {
    fn from(e: reqwest_websocket::Error) -> Self {
        LcuError::WebSocketError(format!("{e:?}"))
    }
}
