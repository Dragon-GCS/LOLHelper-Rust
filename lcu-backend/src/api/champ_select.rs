use crate::{LcuClient, Result};

impl LcuClient {
    pub async fn swap_champion(&self, champion_id: u16) -> Result<()> {
        self.post(&format!(
            "/lol-champ-select/v1/session/bench/swap/{champion_id}"
        ))
        .await?;
        Ok(())
    }

    pub async fn pick_champion(&self, champion_id: u16, action_id: u8) -> Result<()> {
        self.patch_json(
            &format!("/lol-champ-select/v1/session/actions/{action_id}"),
            &serde_json::json!({"completed": true, "type": "pick", "championId": champion_id}),
        )
        .await?;
        Ok(())
    }

    pub async fn subset_champion_list(&self) -> Result<Vec<u16>> {
        Ok(self
            .get("/lol-lobby-team-builder/champ-select/v1/subset-champion-list")
            .await?
            .json::<Vec<u16>>()
            .await?)
    }
}
