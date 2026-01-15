use std::sync::atomic::Ordering;

use crate::{CONTEXT, Result};
use log::info;
use reqwest::Response;

use crate::{LcuClient, events::champ_select::ChampSelectData};

impl LcuClient {
    async fn swap_champion(&self, champion_id: u16) -> Result<Response> {
        self.post(&format!(
            "/lol-champ-select/v1/session/bench/swap/{champion_id}"
        ))
        .await
    }

    async fn pick_champion(&self, champion_id: u16, action_id: u8) -> Result<Response> {
        self.patch_json(
            &format!("/lol-champ-select/v1/session/actions/{action_id}"),
            &serde_json::json!({"completed": true, "type": "pick", "championId": champion_id}),
        )
        .await
    }

    pub(crate) async fn auto_pick(&self, data: ChampSelectData) {
        if !CONTEXT.auto_pick.read().unwrap().enabled
            || CONTEXT.picked.load(Ordering::Relaxed)
            || CONTEXT.champion_id.load(Ordering::Relaxed) != 0
        {
            return;
        }

        let selected = { CONTEXT.auto_pick.read().unwrap().selected.clone() };
        if !CONTEXT.subset_champion_list.read().unwrap().is_empty() {
            for champion in selected.iter() {
                if CONTEXT
                    .subset_champion_list
                    .read()
                    .unwrap()
                    .contains(&champion.0)
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
        }

        let action = data.actions.iter().find(|action| {
            action.actor_cell_id == data.local_player_cell_id
                && action.action_type == "pick"
                && action.is_in_progress
        });
        if action.is_none() {
            return;
        }
        let action = action.unwrap();

        for champion in selected.into_iter() {
            if self.pick_champion(champion.0, action.id).await.is_ok() {
                info!("自动选择英雄: {}", champion.1);
                CONTEXT.picked.store(true, Ordering::Relaxed);
                return;
            }
        }
    }
}
