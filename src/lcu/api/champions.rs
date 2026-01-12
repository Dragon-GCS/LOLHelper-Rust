use crate::lcu::Result;

use crate::{
    context::Champion,
    lcu::{LcuClient, LcuUri},
};

#[derive(Debug, serde::Deserialize)]
pub struct OwnedChampionRow {
    pub id: u16,
    pub name: String,
    pub title: String,
}

impl LcuClient {
    pub async fn get_owned_champions(&self) -> Result<Vec<Champion>> {
        let response = self.get(LcuUri::OWNED_CHAMPIONS).await?;
        let data = response.json::<Vec<OwnedChampionRow>>().await?;
        let champions = data
            .into_iter()
            .map(|champion| Champion(champion.id, format!("{}-{}", champion.name, champion.title)))
            .collect();
        Ok(champions)
    }
}
