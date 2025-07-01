use crate::lcu::GamePhase;

use super::lcu::ChampSelectPlayer;
use log::debug;
use serde::Deserialize;
use std::{collections::HashMap, sync::RwLock};

#[derive(Debug, Deserialize, Default)]
pub struct Me {
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
    pub me: RwLock<Me>,
    pub champions: RwLock<HashMap<u16, String>>,
    pub champion_id: RwLock<u16>,
    pub my_team: RwLock<Vec<ChampSelectPlayer>>,
    pub auto_pick: RwLock<AutoPick>,
    pub game_phase: RwLock<GamePhase>,
    pub accepted: RwLock<bool>,
}

impl HelperContext {
    pub fn reset(&self) {
        *self.champion_id.write().unwrap() = 0;
        (*self.my_team.write().unwrap()).clear();
        debug!("HelperContext reset");
    }
}
