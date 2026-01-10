use std::sync::Arc;
use std::sync::atomic::Ordering;

use anyhow::Result;
use log::info;
use reqwest::Response;

use crate::{
    context::HelperContext,
    lcu::{event::ChampSelectData, LcuClient, LcuUri},
};

impl LcuClient {
    async fn swap_champion(&self, champion_id: u16) -> Result<Response> {
        self.post(&LcuUri::swap_champion(champion_id)).await
    }

    async fn pick_champion(&self, champion_id: u16, action_id: u8) -> Result<Response> {
        self.patch_json(
            &LcuUri::bp_champions(&action_id.to_string()),
            &serde_json::json!({"completed": true, "type": "pick", "championId": champion_id}),
        )
        .await
    }

    pub(crate) async fn auto_pick(&self, ctx: Arc<HelperContext>, data: ChampSelectData) {
        if !ctx.auto_pick.read().unwrap().enabled
            || ctx.picked.load(Ordering::Relaxed)
            || ctx.champion_id.load(Ordering::Relaxed) != 0
        {
            return;
        }

        let selected = { ctx.auto_pick.read().unwrap().selected.clone() };
        if !ctx.subset_champion_list.read().unwrap().is_empty() {
            for champion in selected.iter() {
                if ctx
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
                    ctx.champion_id.store(champion.0, Ordering::Relaxed);
                    ctx.picked.store(true, Ordering::Relaxed);
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
                    ctx.champion_id.store(champion.0, Ordering::Relaxed);
                    ctx.picked.store(true, Ordering::Relaxed);
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
                ctx.picked.store(true, Ordering::Relaxed);
                return;
            }
        }
    }
}
