use std::sync::{Arc, atomic::Ordering};

use super::{
    LcuMeta,
    event::{Event, EventMessage, EventType, GamePhase, MatchReadyResponse},
};

use crate::{context::HelperContext, errors::HelperError};
use anyhow::Result;

use log::{debug, error, info};

use reqwest::Response;

pub struct LcuClient {
    pub client: Arc<reqwest::Client>,
    pub meta: LcuMeta,
}

pub fn default_client() -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        // 添加用户代理以避免发送消息时提示环境异常
        .user_agent(format!("lol-helper/{}", env!("CARGO_PKG_VERSION")).as_str())
        .no_proxy() // 忽略所有代理设置
        .build()
        .expect("Failed to create default LCU client")
}

impl Default for LcuClient {
    fn default() -> Self {
        let client = Arc::new(default_client());
        let meta = LcuMeta::default();
        LcuClient { client, meta }
    }
}
impl LcuClient {
    pub(crate) async fn request<T: serde::Serialize>(
        &self,
        method: reqwest::Method,
        api: &str,
        body: Option<&T>,
    ) -> Result<Response> {
        let url = format!("https://127.0.0.1:{}{}", self.meta.port, api);
        let mut req = self
            .client
            .request(method, url)
            .header("Content-Type", "application/json")
            .basic_auth("riot", Some(&self.meta.token));
        if let Some(body) = body {
            req = req.json(body);
        };
        let r = req.send().await?;
        if !r.status().is_success() {
            let text = r
                .text()
                .await
                .unwrap_or_else(|e| format!("Unknown error: {e}"));
            debug!("请求API({api})失败: {text}");
            Err(HelperError::ResponseError(text).into())
        } else {
            Ok(r)
        }
    }

    pub(crate) async fn get(&self, api: &str) -> Result<Response> {
        self.request(reqwest::Method::GET, api, Option::<&()>::None)
            .await
    }

    pub(crate) async fn post(&self, api: &str) -> Result<Response> {
        self.request(reqwest::Method::POST, api, Option::<&()>::None)
            .await
    }

    pub(crate) async fn post_json<T: serde::Serialize>(
        &self,
        api: &str,
        body: &T,
    ) -> Result<Response> {
        self.request(reqwest::Method::POST, api, Some(body)).await
    }

    pub(crate) async fn patch_json<T: serde::Serialize>(
        &self,
        api: &str,
        body: &T,
    ) -> Result<Response> {
        self.request(reqwest::Method::PATCH, api, Some(body)).await
    }

    pub async fn handle_message(&self, message: String, ctx: Arc<HelperContext>) -> Result<()> {
        if message.is_empty() {
            return Ok(());
        }

        let event = serde_json::from_str::<EventMessage>(&message)?;

        match event.2 {
            Event::GameFlowSession {
                _event_type: _,
                data,
            } => {
                #[cfg(feature = "debug_events")]
                if let GamePhase::Other = data.phase {
                    debug!("Unknown GamePhase: {message}")
                }
                // 在 if 语句中使用 read 锁，避免长时间持有锁导致死锁
                if *ctx.game_phase.read().unwrap() == data.phase {
                    return Ok(());
                }
                match &data.phase {
                    GamePhase::Lobby | GamePhase::None => {
                        ctx.reset();
                    }
                    GamePhase::Matchmaking if ctx.accepted.load(Ordering::Relaxed) => {
                        ctx.accepted.store(false, Ordering::Relaxed);
                    }
                    _ => {}
                }
                info!("当前客户端状态：{:?}", &data.phase);
                *ctx.game_phase.write().unwrap() = data.phase;
                *ctx.game_mode.write().unwrap() = data.map.game_mode;
            }
            Event::MatchmakingReadyCheck {
                _event_type: _,
                data,
            } => {
                if !ctx.accepted.load(Ordering::Relaxed)
                    && data.is_some_and(|data| {
                        matches!(data.player_response, MatchReadyResponse::None)
                    })
                {
                    self.auto_accept(ctx).await;
                }
            }
            Event::LobbyTeamBuilderMatchmaking {
                _event_type: _,
                data: _,
            } => {}
            Event::SubsetChampionList { _event_type, data } => {
                if ctx.subset_champion_list.read().unwrap().is_empty() {
                    *ctx.subset_champion_list.write().unwrap() = data;
                }
            }
            Event::ChampSelectSession {
                _event_type: _,
                data,
            } => {
                if ctx.my_team.read().unwrap().is_empty() && !data.my_team.is_empty() {
                    {
                        let mut my_team = ctx.my_team.write().unwrap();
                        *my_team = data.my_team.clone();
                    }
                }
                if *ctx.auto_send_analysis.read().unwrap()
                    && *ctx.game_mode.read().unwrap() != "TFT"
                {
                    let ctx = ctx.clone();
                    self.analyze_team_players(ctx).await.unwrap_or_else(|e| {
                        error!("Failed to analyze team players: {e}");
                    });
                }
                self.auto_pick(ctx, data).await;
            }
            Event::ChatConversation(data) => match data.event_type {
                EventType::Create => {
                    *ctx.conversation_id.write().unwrap() = data.id;
                    ctx.analysis_sent_flag.store(false, Ordering::Relaxed);
                }
                EventType::Delete => {
                    ctx.conversation_id.write().unwrap().clear();
                }
                _ => {}
            },
            Event::CurrentChampion { event_type, data } => {
                if event_type == EventType::Create {
                    ctx.champion_id.store(data, Ordering::Relaxed);
                }
            }
            Event::Other(_event) => {
                #[cfg(feature = "debug_events")]
                debug!("Received an unhandled event: {_event}");
            }
        }
        Ok(())
    }
}
