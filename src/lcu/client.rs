#![allow(dead_code)]
use std::sync::Arc;

use super::{Event, LcuUri, event::EventMessage};
use crate::{
    context::{HelperContext, Me},
    lcu::event::GamePhase,
};

#[cfg(all(debug_assertions, feature = "debug_events"))]
use log::debug;
use log::{error, info};

use reqwest::RequestBuilder;

use super::LcuMeta;

pub struct LcuClient {
    pub client: Arc<reqwest::Client>,
    pub meta: LcuMeta,
}

impl LcuClient {
    pub fn new() -> anyhow::Result<Self> {
        let client = Arc::new(
            reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap(),
        );
        let meta = LcuMeta::new()?;
        Ok(LcuClient { client, meta })
    }

    pub fn host_url(&self) -> &str {
        self.meta.host_url.as_ref().unwrap()
    }

    fn get(&self, api: &str) -> RequestBuilder {
        let url = self.host_url();
        self.client.get(format!("https://{url}{api}"))
    }

    fn post(&self, uri: &str) -> RequestBuilder {
        let url = self.host_url();
        self.client.post(format!("https://{url}{uri}"))
    }

    pub async fn handle_message(&self, message: &str, ctx: Arc<HelperContext>) {
        if message.is_empty() {
            return;
        }

        let event = serde_json::from_str::<EventMessage>(message);

        match event.unwrap().2 {
            Event::GameFlowSession {
                _event_type: _,
                data,
            } => {
                #[cfg(all(debug_assertions, feature = "debug_events"))]
                if let GamePhase::Other = data.phase {
                    debug!("Unknown GamePhase: {message}")
                }
                {
                    // Use block to ensure we don't hold the lock for too long
                    let current_phase = ctx.game_phase.read().unwrap();
                    let accepted = ctx.accepted.read().unwrap();
                    if *accepted && *current_phase != GamePhase::ReadyCheck {
                        *ctx.accepted.write().unwrap() = false;
                    }
                    if matches!(*current_phase, GamePhase::Lobby | GamePhase::None) {
                        if *current_phase == data.phase {
                            return;
                        } else {
                            ctx.reset();
                        }
                    }
                }
                info!("当前游戏状态{:?}", data.phase);
                *ctx.game_phase.write().unwrap() = data.phase;
            }
            Event::MatchmakingReadyCheck {
                _event_type: _,
                data,
            } => {
                if data.is_some() && !*ctx.accepted.read().unwrap() {
                    self.auto_accept(ctx).await;
                    info!("ReadyCheck：{data:?}");
                }
            }
            Event::LobbyTeamBuilderMatchmaking {
                _event_type: _,
                data,
            } => {
                info!("MatchMaking：{data:?}");
            }
            Event::ChampSelectSession {
                _event_type: _,
                data,
            } => {
                info!(
                    "ChampSelect：{:?}\nMy team: {:?}",
                    data.bench_champions, data.my_team
                );
                self.auto_select(ctx, data.bench_champions).await;
            }
            #[cfg(debug_assertions)]
            Event::Other(_event) => {
                #[cfg(feature = "debug_events")]
                debug!("Received an unhandled event: {_event}");
            }
        }
    }

    pub async fn update_summoner_info(&self, ctx: Arc<HelperContext>) {
        match self.get(LcuUri::ME).send().await {
            Ok(response) => {
                let data = response.json::<Me>().await;
                if let Err(e) = &data {
                    error!("Failed to parse summoner info: {e}");
                    return;
                }
                let mut info = ctx.me.write().unwrap();
                *info = data.unwrap();
                info!("当前玩家信息: {info:?}");
            }
            Err(e) => {
                error!("Failed to get summoner info: {e}");
            }
        }
    }

    async fn auto_select(&self, ctx: Arc<HelperContext>, bench_champions: Vec<u16>) {
        if bench_champions.is_empty() {
            return;
        }
        let select_champion = {
            let selected = &ctx.auto_pick.read().unwrap().selected;
            let priority_champion = bench_champions
                .iter()
                .filter(|&champion| selected.contains_key(champion))
                .max_by_key(|&champion| selected[champion]);

            let champion_id = ctx.champion_id.read().unwrap();
            // 可选英雄中没有自动选择的英雄
            // 当前英雄优先级大于自动选择的英雄
            if priority_champion.is_none()
                || selected
                    .get(&*champion_id)
                    .map(|&current_priority| {
                        current_priority > selected[priority_champion.unwrap()]
                    })
                    .unwrap_or(false)
            {
                return;
            }
            priority_champion.unwrap()
        };

        match self
            .post(&LcuUri::swap_champion(*select_champion))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => {
                info!("自动选择英雄: {select_champion}");
            }
            Ok(r) => {
                error!("选择英雄响应异常: {}", r.text().await.unwrap());
            }
            Err(e) => {
                error!("选择英雄请求失败: {e}");
            }
        }
    }

    async fn auto_accept(&self, ctx: Arc<HelperContext>) {
        if *ctx.accepted.read().unwrap() {
            return;
        }

        match self.post(LcuUri::ACCEPT_GAME).send().await {
            Ok(r) => {
                if !r.status().is_success() {
                    error!("自动接受准备检查失败: {}", r.text().await.unwrap());
                    return;
                }
                *ctx.accepted.write().unwrap() = true;
                info!("自动接受对局");
            }
            Err(e) => {
                error!("Failed to auto accept: {e}");
            }
        }
    }
}
