#![allow(dead_code)]
use std::sync::Arc;

use super::{Event, LcuUri, event::EventMessage};
use crate::{
    context::{HelperContext, Me},
    lcu::event::GamePhase,
};
#[cfg(debug_assertions)]
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
                event_type: _,
                data,
            } => {
                #[cfg(feature = "debug_events")]
                if let GamePhase::Other = data.phase {
                    debug!("Unknown GamePhase: {message}")
                }
                {
                    let current_phase = ctx.game_phase.read().unwrap();
                    let accepted = ctx.accepted.read().unwrap();
                    if *accepted && *current_phase != GamePhase::ReadyCheck {
                        *ctx.accepted.write().unwrap() = false;
                    }
                    if matches!(*current_phase, GamePhase::Lobby | GamePhase::None)
                        && *current_phase == data.phase
                    {
                        return;
                    }
                }
                info!(
                    "当前游戏状态{:?}\n己方队伍：{:?}\n地方队伍{:?}",
                    data.phase, &data.game_data.team_one, &data.game_data.team_two
                );
                *ctx.game_phase.write().unwrap() = data.phase;
            }
            Event::MatchmakingReadyCheck {
                event_type: _,
                data,
            } => {
                if data.is_some() && !*ctx.accepted.read().unwrap() {
                    self.auto_accept(ctx).await;
                    info!("ReadyCheck：{data:?}");
                }
            }
            Event::LobbyTeamBuilderMatchmaking {
                event_type: _,
                data,
            } => {
                info!("MatchMaking：{data:?}");
            }
            Event::ChampSelectSession {
                event_type: _,
                data,
            } => {
                info!(
                    "ChampSelect：{:?}\nMy team: {:?}",
                    data.bench_champions, data.my_team
                );
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
