use std::sync::atomic::Ordering;

use crate::Result;
use log::error;

use crate::{CONTEXT, LcuClient, events::EventType};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectData {
    #[serde(deserialize_with = "deserialize_champion_ids")]
    pub bench_champions: Vec<u16>, // Vec<ChampionId>
    pub bench_enabled: bool,
    #[serde(deserialize_with = "unwrap_actions")]
    pub actions: Vec<Action>,
    pub local_player_cell_id: u8, // cell_id
    // pub id: String,
    pub my_team: Vec<ChampSelectPlayer>,
}

#[derive(Debug, Deserialize)]
pub struct Action {
    #[serde(rename = "actorCellId")]
    pub actor_cell_id: u8,
    #[serde(rename = "championId")]
    pub champion_id: u16,
    pub completed: bool,
    pub id: u8, // cell_id
    #[serde(rename = "isInProgress")]
    pub is_in_progress: bool,
    #[serde(rename = "type")]
    pub action_type: String,
}
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectPlayer {
    #[serde(default)]
    pub cell_id: u8,
    pub puuid: String,
    pub summoner_id: u64,
    pub champion_id: u16,
}

/// Deserialize champion IDs from a JSON array of objects
fn deserialize_champion_ids<'de, D>(deserializer: D) -> std::result::Result<Vec<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    // 先反序列化为中间结构
    #[derive(Deserialize)]
    struct ChampWrapper {
        #[serde(rename = "championId")]
        champion_id: u16,
    }

    // 然后提取 champion_id 字段值
    let wrappers = Vec::<ChampWrapper>::deserialize(deserializer)?;
    Ok(wrappers.into_iter().map(|w| w.champion_id).collect())
}

fn unwrap_actions<'de, D>(deserializer: D) -> std::result::Result<Vec<Action>, D::Error>
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
    pub(crate) async fn handle_champ_select_event(&self, data: ChampSelectData) -> Result<()> {
        if CONTEXT.my_team.read().unwrap().is_empty() && !data.my_team.is_empty() {
            {
                *CONTEXT.my_team.write().unwrap() = data.my_team.clone();
            }
        }
        if CONTEXT.auto_send_analysis.load(Ordering::Relaxed)
            && *CONTEXT.game_mode.read().unwrap() != "TFT"
        {
            self.analyze_team_players().await.unwrap_or_else(|e| {
                error!("Failed to analyze team players: {e}");
            });
        }
        self.auto_pick(data).await;
        Ok(())
    }

    pub(crate) async fn handle_subset_champion_list_event(&self, data: Vec<u16>) -> Result<()> {
        if CONTEXT.subset_champion_list.read().unwrap().is_empty() {
            *CONTEXT.subset_champion_list.write().unwrap() = data;
        }
        Ok(())
    }

    pub(crate) async fn handle_current_champion_event(
        &self,
        event_type: EventType,
        data: u16,
    ) -> Result<()> {
        if event_type == EventType::Create {
            CONTEXT.champion_id.store(data, Ordering::Relaxed);
        }
        Ok(())
    }
}
