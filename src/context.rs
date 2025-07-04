use crate::lcu::GamePhase;

use super::lcu::ChampSelectPlayer;
use log::debug;
use serde::Deserialize;
use std::{collections::HashMap, sync::RwLock};

#[derive(Debug, Deserialize, Default)]
pub struct Summoner {
    #[serde(rename = "gameName")]
    pub game_name: String,
    #[serde(rename = "summonerId")]
    pub summoner_id: u64,
    #[serde(rename = "summonerLevel")]
    pub summoner_level: u16,
    pub puuid: String,
}

#[derive(Debug, Default)]
pub struct AutoPick {
    // champion_id: pority
    pub selected: HashMap<u16, u16>,
    pub unselected: HashMap<u16, u16>,
}

#[derive(Debug, Default)]
pub struct HelperContext {
    pub me: RwLock<Summoner>,
    pub champions: RwLock<HashMap<u16, String>>,
    pub champion_id: RwLock<u16>,
    pub my_team: RwLock<Vec<ChampSelectPlayer>>,
    pub auto_pick: RwLock<AutoPick>,
    pub game_phase: RwLock<GamePhase>,
    pub game_mode: RwLock<String>,
    pub conversation_id: RwLock<String>,
    pub accepted: RwLock<bool>,
    pub analysis_sent_flag: RwLock<bool>,
}

impl HelperContext {
    pub fn reset(&self) {
        *self.champion_id.write().unwrap() = 0;
        (*self.my_team.write().unwrap()).clear();
        *self.accepted.write().unwrap() = false;
        self.conversation_id.write().unwrap().clear();
        self.game_mode.write().unwrap().clear();
        *self.analysis_sent_flag.write().unwrap() = false;
        debug!("HelperContext reset");
    }
}
