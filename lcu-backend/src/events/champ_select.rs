use std::sync::atomic::Ordering;

use log::info;

use crate::{CONTEXT, LcuClient, Result, events::EventType};
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
    pub my_team: Vec<ChampSelectPlayer>, // 队友列表
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
    /// 处理英雄选择事件，保存队伍信息并尝试自动选人
    pub(crate) async fn handle_champ_select_event(&self, data: ChampSelectData) {
        if !CONTEXT.auto_pick.read().unwrap().enabled || CONTEXT.picked.load(Ordering::Relaxed) {
            return;
        }
        // 当前玩家不在英雄选择阶段
        if !data.actions.iter().any(|action| {
            action.actor_cell_id == data.local_player_cell_id
                && action.action_type == "pick"
                && action.is_in_progress
        }) {
            return;
        }

        let selected = { CONTEXT.auto_pick.read().unwrap().selected.clone() };
        // 大乱斗英雄选择
        if !CONTEXT.subset_champion_list.read().unwrap().is_empty() {
            // 清空subset_champion_list，避免重复使用
            let subset_champions = {
                let mut list = CONTEXT.subset_champion_list.write().unwrap();
                std::mem::take(&mut *list)
            };
            for champion in selected.iter() {
                if subset_champions.contains(&champion.0)
                    && self
                        .pick_champion(champion.0, data.local_player_cell_id)
                        .await
                        .is_ok()
                {
                    info!("自动选择英雄: {}", champion.1);
                    CONTEXT.champion_id.store(champion.0, Ordering::Relaxed);
                    CONTEXT.picked.store(true, Ordering::Relaxed);
                    return;
                }
            }
        }

        if data.bench_enabled {
            for champion in selected.iter() {
                if data.bench_champions.contains(&champion.0)
                    && self.swap_champion(champion.0).await.is_ok()
                {
                    info!("自动选择英雄: {}", champion.1);
                    CONTEXT.champion_id.store(champion.0, Ordering::Relaxed);
                    CONTEXT.picked.store(true, Ordering::Relaxed);
                    return;
                }
            }
        } else {
            for champion in selected.into_iter() {
                if self
                    .pick_champion(champion.0, data.local_player_cell_id)
                    .await
                    .is_ok()
                {
                    info!("自动选择英雄: {}", champion.1);
                    CONTEXT.picked.store(true, Ordering::Relaxed);
                    return;
                }
            }
        }
    }

    /// 保存备选英雄列表
    pub(crate) async fn handle_subset_champion_list_event(&self, data: Vec<u16>) {
        if CONTEXT.subset_champion_list.read().unwrap().is_empty() {
            *CONTEXT.subset_champion_list.write().unwrap() = data;
        }
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
