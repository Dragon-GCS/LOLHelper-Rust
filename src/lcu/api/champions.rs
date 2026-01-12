use crate::lcu::Result;

use crate::{
    context::Champion,
    lcu::LcuClient,
};

const OWNED_CHAMPIONS_API: &str = "/lol-champions/v1/owned-champions-minimal";

#[derive(Debug, serde::Deserialize)]
pub struct OwnedChampionRow {
    pub id: u16,
    pub name: String,
    pub title: String,
}

impl LcuClient {
    pub async fn get_owned_champions(&self) -> Result<Vec<Champion>> {
        let response = self.get(OWNED_CHAMPIONS_API).await?;
        let data = response.json::<Vec<OwnedChampionRow>>().await?;
        let champions = data
            .into_iter()
            .map(|champion| Champion(champion.id, format!("{}-{}", champion.name, champion.title)))
            .collect();
        Ok(champions)
    }
}
