use std::sync::Arc;

use crate::lcu::event::GamePhase;

use super::{Event, event::EventMessage};

use log::{error, info};
pub(super) struct EventHandler {
    host_url: String,
    client: Arc<reqwest::Client>,
}
impl EventHandler {
    pub fn new(host_url: &str, client: Arc<reqwest::Client>) -> Self {
        Self {
            host_url: host_url.into(),
            client,
        }
    }

    pub async fn handle_message(&self, message: &str) {
        if message.is_empty() {
            return;
        }

        let event = serde_json::from_str::<EventMessage>(message);
        if let Err(_) = event {
            // trace!("Unexpected event({e}): {message}.");
            return;
        }
        match event.unwrap().2 {
            Event::GameFlowSession {
                event_type: _,
                data,
            } => {
                let phrase = data.phase;
                if let GamePhase::Other = phrase {
                    error!("Unknown GamePhase: {message}")
                }
                let team_a = data.game_data.team_one;
                let team_b = data.game_data.team_two;
                info!("当前游戏状态{phrase:?}\n己方队伍：{team_a:?}\n地方队伍{team_b:?}");
            }
            Event::ChatMe {
                event_type: _,
                data,
            } => {
                // [6, "OnJsonApiEvent"]
                info!("个人信息：{data:?}")
            }
            Event::MatchmakingReadyCheck {
                event_type: _,
                data,
            } => {
                info!("ReadyCheck：{data:?}");
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
                let bench_champions = data.bench_champions;
                info!("ChampSelect：{bench_champions:?}");
            }
        }
    }
}
