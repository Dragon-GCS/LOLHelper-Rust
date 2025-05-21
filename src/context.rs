use crate::lcu::GamePhase;

use super::lcu::ChampSelectPlayer;
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
    pub selected: HashMap<u16, String>,
    pub unselected: HashMap<u16, String>,
}

#[derive(Debug, Default)]
pub struct HelperContext {
    pub me: RwLock<Me>,
    pub my_team: RwLock<Vec<ChampSelectPlayer>>,
    pub auto_pick: RwLock<AutoPick>,
    pub game_phase: RwLock<GamePhase>,
    pub accepted: RwLock<bool>,
}
