use crate::Result;
use log::{debug, error, info};

use crate::{CONTEXT, LcuClient, context::Summoner};

const CURRENT_SUMMONER_API: &str = "/lol-summoner/v1/current-summoner";

impl LcuClient {
    pub async fn update_summoner_info(&self) -> Result<()> {
        let response = self.get(CURRENT_SUMMONER_API).await?;
        let data = response.json::<Summoner>().await;
        if let Err(e) = &data {
            error!("Failed to parse summoner info: {e}");
            return Ok(());
        }
        let data = data.unwrap();
        if data.puuid == CONTEXT.me.read().unwrap().puuid {
            debug!("玩家信息未变更，跳过更新");
            return Ok(());
        }
        info!("当前玩家信息: {data:?}");
        *CONTEXT.me.write().unwrap() = data;
        Ok(())
    }
}
