use std::sync::{Arc, atomic::Ordering};

use anyhow::Result;
use log::error;

use crate::{
    context::HelperContext,
    lcu::{event::EventType, LcuClient},
    types::{CellId, ChampionId, PlayerId, SummonerId},
};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectData {
    #[serde(deserialize_with = "deserialize_champion_ids")]
    pub bench_champions: Vec<ChampionId>,
    pub bench_enabled: bool,
    #[serde(deserialize_with = "unwrap_actions")]
    pub actions: Vec<Action>,
    pub local_player_cell_id: PlayerId,
    // pub id: String,
    pub my_team: Vec<ChampSelectPlayer>,
}

#[derive(Debug, Deserialize)]
pub struct Action {
    #[serde(rename = "actorCellId")]
    pub actor_cell_id: CellId,
    #[serde(rename = "championId")]
    pub champion_id: ChampionId,
    pub completed: bool,
    pub id: CellId,
    #[serde(rename = "isInProgress")]
    pub is_in_progress: bool,
    #[serde(rename = "type")]
    pub action_type: String,
}
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectPlayer {
    #[serde(default)]
    pub cell_id: CellId,
    pub puuid: String,
    pub summoner_id: SummonerId,
    pub champion_id: ChampionId,
}

/// Deserialize champion IDs from a JSON array of objects
fn deserialize_champion_ids<'de, D>(deserializer: D) -> Result<Vec<ChampionId>, D::Error>
where
    D: Deserializer<'de>,
{
    // 先反序列化为中间结构
    #[derive(Deserialize)]
    struct ChampWrapper {
        #[serde(rename = "championId")]
        champion_id: ChampionId,
    }

    // 然后提取 champion_id 字段值
    let wrappers = Vec::<ChampWrapper>::deserialize(deserializer)?;
    Ok(wrappers.into_iter().map(|w| w.champion_id).collect())
}

fn unwrap_actions<'de, D>(deserializer: D) -> Result<Vec<Action>, D::Error>
where
    D: Deserializer<'de>,
{
    let actions = Vec::<Vec<Action>>::deserialize(deserializer).unwrap();
    if actions.is_empty() {
        Ok(vec![])
    } else {
        Ok(actions.into_iter().flatten().collect())
    }
}

impl LcuClient {
    pub(crate) async fn handle_champ_select_event(
        &self,
        data: ChampSelectData,
        ctx: Arc<HelperContext>,
    ) -> Result<()> {
        if ctx.my_team.read().unwrap().is_empty() && !data.my_team.is_empty() {
            {
                let mut my_team = ctx.my_team.write().unwrap();
                *my_team = data.my_team.clone();
            }
        }
        if *ctx.auto_send_analysis.read().unwrap() && *ctx.game_mode.read().unwrap() != "TFT" {
            let ctx = ctx.clone();
            self.analyze_team_players(ctx).await.unwrap_or_else(|e| {
                error!("Failed to analyze team players: {e}");
            });
        }
        self.auto_pick(ctx, data).await;
        Ok(())
    }

    pub(crate) async fn handle_subset_champion_list_event(
        &self,
        data: Vec<ChampionId>,
        ctx: Arc<HelperContext>,
    ) -> Result<()> {
        if ctx.subset_champion_list.read().unwrap().is_empty() {
            *ctx.subset_champion_list.write().unwrap() = data;
        }
        Ok(())
    }

    pub(crate) async fn handle_current_champion_event(
        &self,
        event_type: EventType,
        data: ChampionId,
        ctx: Arc<HelperContext>,
    ) -> Result<()> {
        if event_type == EventType::Create {
            ctx.champion_id.store(data, Ordering::Relaxed);
        }
        Ok(())
    }
}
