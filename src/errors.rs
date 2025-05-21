use thiserror::Error;

#[derive(Error, Debug)]
pub enum HelperError {
    #[error("Failed to find LCU process")]
    ClientNotFound,
    #[error("Failed to load LCU start commands")]
    ClientCMDLineFailed,
}
