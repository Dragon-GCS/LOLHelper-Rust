use std::sync::atomic::Ordering;

use crate::Result;
use log::info;

use crate::{CONTEXT, ChampSelectPlayer, LcuClient};
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
    pub(crate) async fn handle_game_flow_event(&self, data: GameFlowSession) -> Result<()> {
        // 在 if 语句中使用 read 锁，避免长时间持有锁导致死锁
        if *CONTEXT.game_phase.read().unwrap() == data.phase {
            return Ok(());
        }
        match &data.phase {
            GamePhase::Lobby | GamePhase::None => {
                CONTEXT.reset();
            }
            GamePhase::Matchmaking if CONTEXT.accepted.load(Ordering::Relaxed) => {
                CONTEXT.accepted.store(false, Ordering::Relaxed);
            }
            _ => {}
        }
        info!("当前客户端状态：{:?}", &data.phase);
        if !data.map.game_mode.is_empty()
            && data.map.game_mode != *CONTEXT.game_mode.read().unwrap()
        {
            info!("当前游戏模式: {}", &data.map.game_mode);
        }
        *CONTEXT.game_phase.write().unwrap() = data.phase;
        *CONTEXT.game_mode.write().unwrap() = data.map.game_mode;
        Ok(())
    }
}
