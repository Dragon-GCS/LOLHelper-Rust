use serde::Deserialize;

use crate::{LcuClient, LcuError};

#[derive(Debug, Deserialize)]
pub struct ProcessStatus {
    pub status: String,
}

impl LcuClient {
    pub(crate) async fn handle_process_control_event(
        &self,
        status: ProcessStatus,
    ) -> crate::Result<()> {
        if status.status == "Stopping" {
            return Err(LcuError::ClientExit);
        }
        Ok(())
    }
}
