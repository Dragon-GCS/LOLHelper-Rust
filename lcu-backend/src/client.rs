use std::sync::Arc;

use super::{Event, LcuMeta};

use crate::{LcuError, Result};

use log::debug;

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
            Err(LcuError::ResponseError(text))
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

    pub async fn handle_message(&self, message: String) -> Result<()> {
        if message.is_empty() {
            return Ok(());
        }

        // [8, "OnJsonEvent", event]
        let (_code, _event_type, event): (u8, String, Event) = serde_json::from_str(&message)?;

        match event {
            Event::GameFlowSession {
                _event_type: _,
                data,
            } => self.handle_game_flow_event(data).await?,
            Event::MatchmakingReadyCheck {
                _event_type: _,
                data,
            } => self.handle_matchmaking_ready_check_event(data).await?,
            Event::LobbyTeamBuilderMatchmaking {
                _event_type: _,
                data,
            } => self.handle_lobby_matchmaking_event(data).await?,
            Event::ChampSelectSession {
                _event_type: _,
                data,
            } => self.handle_champ_select_event(data).await,
            Event::ChatConversation(data) => self.handle_chat_conversation_event(data).await,
            Event::CurrentChampion { event_type, data } => {
                self.handle_current_champion_event(event_type, data).await?
            }
            Event::Other(_event) => {
                #[cfg(feature = "debug_events")]
                debug!("Received an unhandled event: {_event}");
            }
        }
        Ok(())
    }
}
