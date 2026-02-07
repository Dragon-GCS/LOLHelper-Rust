use crate::Result;

use crate::LcuClient;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchMaking {
    pub queue_id: u16,
    pub search_state: String,
    pub ready_check: MatchMakingReadyCheck,
}

#[derive(Debug, Deserialize)]

pub enum MatchReadyResponse {
    Accepted,
    Declined,
    None,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchMakingReadyCheck {
    pub player_response: MatchReadyResponse,
    pub timer: f32,
}

impl LcuClient {
    pub(crate) async fn handle_lobby_matchmaking_event(
        &self,
        _data: Option<MatchMaking>,
    ) -> Result<()> {
        // Current implementation is empty
        Ok(())
    }
}
