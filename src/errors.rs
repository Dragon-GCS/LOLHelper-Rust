use thiserror::Error;

#[derive(Error, Debug)]
pub enum HelperError {
    #[error("Failed to find LCU process")]
    ClientNotFound,
    #[error("Failed to load LCU start commands")]
    ClientCMDLineFailed,
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Response error: {0}")]
    ResponseError(String),
}
