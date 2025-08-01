use std::sync::Arc;

use super::{
    LcuMeta, LcuUri,
    api_schema::{Match, Matches, MessageBody, PlayerScore},
    event::{ChampSelectData, Event, EventMessage, EventType, GamePhase, MatchReadyResponse},
};

use crate::{
    context::{Champion, HelperContext, Summoner},
    errors::HelperError,
};
use anyhow::Result;

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use log::{debug, error, info};

use reqwest::Response;
use serde_json::Value;

pub struct LcuClient {
    pub client: Arc<reqwest::Client>,
    pub meta: LcuMeta,
}

impl Default for LcuClient {
    fn default() -> Self {
        let client = Arc::new(
            reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap(),
        );
        let meta = LcuMeta::default();
        LcuClient { client, meta }
    }
}
impl LcuClient {
    pub fn host_url(&self) -> &str {
        &self.meta.host_url
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
                .unwrap_or_else(|e| format!("Unknown error: {e}"));
            debug!("请求API({api})失败: {text}");
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

    async fn patch_json<T: serde::Serialize>(&self, api: &str, body: &T) -> Result<Response> {
        self.request(reqwest::Method::PATCH, api, Some(body)).await
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
                #[cfg(all(debug_assertions, feature = "debug_events"))]
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
                    GamePhase::Matchmaking if *ctx.accepted.read().unwrap() => {
                        *ctx.accepted.write().unwrap() = false;
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
                if !*ctx.accepted.read().unwrap()
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
                    *ctx.analysis_sent_flag.write().unwrap() = false;
                }
                EventType::Delete => {
                    ctx.conversation_id.write().unwrap().clear();
                }
                _ => {}
            },
            Event::CurrentChampion { event_type, data } => {
                if event_type == EventType::Create {
                    *ctx.champion_id.write().unwrap() = data;
                }
            }
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

    pub async fn get_owned_champions(&self) -> Result<Vec<Champion>> {
        let response = self.get(LcuUri::OWNED_CHAMPIONS).await?;
        let data = response.json::<Vec<Value>>().await?;
        let champions = data
            .into_iter()
            .filter_map(|champion| {
                match (
                    champion.get("id").and_then(|v| v.as_u64()),
                    champion.get("name").and_then(|v| v.as_str()),
                ) {
                    (Some(id), Some(name)) => Some(Champion(id as u16, name.to_string())),
                    _ => None,
                }
            })
            .collect();
        Ok(champions)
    }

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

    async fn auto_pick(&self, ctx: Arc<HelperContext>, data: ChampSelectData) {
        if !ctx.auto_pick.read().unwrap().enabled
            || *ctx.picked.read().unwrap()
            || *ctx.champion_id.read().unwrap() != 0
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
                    *ctx.champion_id.write().unwrap() = champion.0;
                    *ctx.picked.write().unwrap() = true;
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
                    *ctx.champion_id.write().unwrap() = champion.0;
                    *ctx.picked.write().unwrap() = true;
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
                *ctx.picked.write().unwrap() = true;
                return;
            }
        }
    }

    async fn auto_accept(&self, ctx: Arc<HelperContext>) {
        let delay = *ctx.auto_accepted_delay.read().unwrap();
        if delay >= 0 {
            info!("将在 {delay} 秒后自动接受对局。");
            tokio::time::sleep(tokio::time::Duration::from_secs(delay as u64)).await;
        }
        let _ = self.post(LcuUri::ACCEPT_GAME).await.map_err(|e| {
            error!("自动接受对局失败: {e}");
        });
        info!("对局已自动接受");
        *ctx.accepted.write().unwrap() = true;
    }

    async fn analyze_team_players(&self, ctx: Arc<HelperContext>) -> Result<()> {
        if *ctx.analysis_sent_flag.read().unwrap()
            || ctx.game_mode.read().unwrap().is_empty()
            || ctx.conversation_id.read().unwrap().is_empty()
        {
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
                .filter_map(|player| {
                    if player.puuid.is_empty() {
                        None
                    } else {
                        Some(player.puuid.clone())
                    }
                })
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
