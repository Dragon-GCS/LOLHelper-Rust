use reqwest::Response;

use crate::{LcuClient, Result};

impl LcuClient {
    pub async fn swap_champion(&self, champion_id: u16) -> Result<Response> {
        self.post(&format!(
            "/lol-champ-select/v1/session/bench/swap/{champion_id}"
        ))
        .await
    }

    pub async fn pick_champion(&self, champion_id: u16, action_id: u8) -> Result<Response> {
        self.patch_json(
            &format!("/lol-champ-select/v1/session/actions/{action_id}"),
            &serde_json::json!({"completed": true, "type": "pick", "championId": champion_id}),
        )
        .await
    }
}
