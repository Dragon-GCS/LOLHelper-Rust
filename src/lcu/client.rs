use std::{collections::HashMap, sync::Arc};

use super::{Event, LcuUri, event::EventMessage};
use crate::{
    context::{HelperContext, Summoner},
    errors::HelperError,
    lcu::{
        api_schema::{Match, Matches},
        event::{EventType, GamePhase},
    },
};
use anyhow::Result;

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
#[cfg(all(debug_assertions, feature = "debug_events"))]
use log::debug;
use log::{error, info};

use reqwest::Response;
use serde_json::Value;

use super::LcuMeta;
use super::api_schema::{MessageBody, PlayerScore};

pub struct LcuClient {
    pub client: Arc<reqwest::Client>,
    pub meta: LcuMeta,
}

impl LcuClient {
    pub fn new() -> Result<Self> {
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

    async fn request<T: serde::Serialize>(
        &self,
        method: reqwest::Method,
        api: &str,
        body: Option<&T>,
    ) -> Result<Response> {
        let url = self.host_url();
        let mut req = self.client.request(method, format!("https://{url}{api}"));
        if let Some(body) = body {
            req = req.header("Content-Type", "application/json").json(body);
        };
        let r = req.send().await?;
        if !r.status().is_success() {
            let text = r
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("请求API({api})失败: {text}");
            Err(HelperError::ResponseError(text).into())
        } else {
            Ok(r)
        }
    }

    async fn get(&self, api: &str) -> Result<Response> {
        self.request(reqwest::Method::GET, api, Option::<&()>::None)
            .await
    }

    async fn post(&self, api: &str) -> Result<Response> {
        self.request(reqwest::Method::POST, api, Option::<&()>::None)
            .await
    }

    async fn post_json<T: serde::Serialize>(&self, api: &str, body: &T) -> Result<Response> {
        self.request(reqwest::Method::POST, api, Some(body)).await
    }

    async fn send_message(&self, conversation_id: &str, message: &str) {
        let _ = self
            .post_json(
                &LcuUri::conversation_message(conversation_id),
                &MessageBody::message(message),
            )
            .await
            .map(|_| {
                info!("发送消息到对话({conversation_id}):\n{message}");
            });
    }

    pub async fn handle_message(&self, message: &str, ctx: Arc<HelperContext>) -> Result<()> {
        if message.is_empty() {
            return Ok(());
        }

        let event = serde_json::from_str::<EventMessage>(message)?;

        match event.2 {
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
                    if *ctx.game_phase.read().unwrap() == data.phase {
                        return Ok(());
                    }
                }
                if matches!(&data.phase, GamePhase::Lobby | GamePhase::None) {
                    ctx.reset();
                }
                info!("当前客户端状态：{:?}", &data.phase);
                *ctx.game_phase.write().unwrap() = data.phase;
                *ctx.game_mode.write().unwrap() = data.map.game_mode;
            }
            Event::MatchmakingReadyCheck {
                _event_type: _,
                data,
            } => {
                if data.is_some() && !*ctx.accepted.read().unwrap() {
                    self.auto_accept(ctx).await;
                    info!("Auto accept game");
                }
            }
            Event::LobbyTeamBuilderMatchmaking {
                _event_type: _,
                data: _,
            } => {}
            Event::ChampSelectSession {
                _event_type: _,
                data,
            } => {
                if ctx.my_team.read().unwrap().is_empty() && !data.my_team.is_empty() {
                    {
                        let mut my_team = ctx.my_team.write().unwrap();
                        *my_team = data.my_team;
                    }
                }
                if !ctx.game_mode.read().unwrap().is_empty()
                    && !ctx.conversation_id.read().unwrap().is_empty()
                {
                    let ctx = ctx.clone();
                    self.analyze_team_players(ctx).await.unwrap_or_else(|e| {
                        error!("Failed to analyze team players: {e}");
                    });
                }
                self.auto_select(ctx, data.bench_champions).await;
            }
            Event::ChatConversation(data) => match data.event_type {
                EventType::Create => {
                    *ctx.conversation_id.write().unwrap() = data.id;
                    *ctx.analysis_sent_flag.write().unwrap() = false;
                }
                EventType::Delete => {
                    ctx.conversation_id.write().unwrap().clear();
                }
                _ => {}
            },
            #[cfg(debug_assertions)]
            Event::Other(_event) => {
                #[cfg(feature = "debug_events")]
                debug!("Received an unhandled event: {_event}");
            }
        }
        Ok(())
    }

    pub async fn update_summoner_info(&self, ctx: Arc<HelperContext>) -> Result<()> {
        let response = self.get(LcuUri::ME).await?;
        let data = response.json::<Summoner>().await;
        if let Err(e) = &data {
            error!("Failed to parse summoner info: {e}");
            return Ok(());
        }
        let mut info = ctx.me.write().unwrap();
        *info = data.unwrap();
        info!("当前玩家信息: {info:?}");
        Ok(())
    }

    pub async fn get_owned_champions(&self) -> Result<HashMap<u16, String>> {
        let response = self.get(LcuUri::OWNED_CHAMPIONS).await?;
        let data = response.json::<Vec<Value>>().await?;
        let mut champions = HashMap::new();
        data.into_iter().for_each(|champion| {
            if let (Some(id), Some(name)) = (
                champion.get("id").and_then(|v| v.as_u64()),
                champion.get("name").and_then(|v| v.as_str()),
            ) {
                champions.insert(id as u16, name.to_string());
            }
        });
        Ok(champions)
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
        if self
            .post(&LcuUri::swap_champion(*select_champion))
            .await
            .is_ok()
        {
            info!("自动选择英雄: {select_champion}");
        }
    }

    async fn auto_accept(&self, ctx: Arc<HelperContext>) {
        if *ctx.accepted.read().unwrap() {
            return;
        }
        if self.post(LcuUri::ACCEPT_GAME).await.is_err() {
            return;
        }
        *ctx.accepted.write().unwrap() = true;
    }

    async fn analyze_team_players(&self, ctx: Arc<HelperContext>) -> Result<()> {
        if *ctx.analysis_sent_flag.read().unwrap() {
            return Ok(());
        }
        let game_mode = { ctx.game_mode.read().unwrap().clone() };
        let conversation_id = { ctx.conversation_id.read().unwrap().clone() };
        info!("当前游戏模式: {game_mode}");
        let puuids = {
            ctx.my_team
                .read()
                .unwrap()
                .iter()
                .map(|player| player.puuid.clone())
                .collect::<Vec<String>>()
        };
        let mut tasks = puuids
            .iter()
            .map(|puuid| self.analyze_player(puuid, &game_mode))
            .collect::<FuturesUnordered<_>>();

        while let Some(player_score) = tasks.next().await {
            let player_score = player_score?;
            let msg = format!("{player_score}");
            self.send_message(&conversation_id, &msg).await
        }
        *ctx.analysis_sent_flag.write().unwrap() = true;
        Ok(())
    }

    async fn get_matches(&self, puuid: &str, begin: usize, num: usize) -> Result<Matches> {
        let response = self
            .get(&LcuUri::matches(puuid, begin, begin + num))
            .await?;
        Ok(response.json::<Matches>().await?)
    }

    async fn analyze_player(&self, puuid: &str, game_mode: &str) -> Result<PlayerScore> {
        let summoner = self
            .get(&LcuUri::summoners_by_puuid(puuid))
            .await?
            .json::<Summoner>()
            .await?;
        let matches = self.get_matches(puuid, 0, 20).await?;
        let mut score = matches.calculate_player_score(game_mode);
        score.set_name(&summoner.game_name);
        Ok(score)
    }
}

impl Matches {
    fn calculate_player_score(self, game_mode: &str) -> PlayerScore {
        let matches = self
            .0
            .into_iter()
            .filter(|m| m.game_mode == game_mode)
            .collect::<Vec<Match>>();

        PlayerScore::calculate(matches)
    }
}
