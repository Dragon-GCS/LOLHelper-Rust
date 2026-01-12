use std::sync::{Arc, atomic::Ordering};

use crate::lcu::Result;
use log::info;

use crate::{context::HelperContext, lcu::ChampSelectPlayer, lcu::LcuClient};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFlowSession {
    pub phase: GamePhase,
    pub game_data: GameFlowGameData,
    pub map: Map,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFlowGameData {
    // pub game_id: u64,
    pub team_one: Vec<ChampSelectPlayer>,
    pub team_two: Vec<ChampSelectPlayer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Map {
    pub game_mode: String,
    // pub name: String,
}

#[derive(Default, Debug, Deserialize, PartialEq, Eq)]
pub enum GamePhase {
    ChampSelect,
    GameStart,
    InProgress,
    Lobby,
    Matchmaking,
    #[default]
    None,
    PreEndOfGame,
    ReadyCheck,
    #[serde(other)]
    Other,
}

impl LcuClient {
    pub(crate) async fn handle_game_flow_event(
        &self,
        data: GameFlowSession,
        ctx: Arc<HelperContext>,
    ) -> Result<()> {
        #[cfg(feature = "debug_events")]
        if let GamePhase::Other = data.phase {
            debug!("Unknown GamePhase in session data: {:?}", data);
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
        Ok(())
    }
}
