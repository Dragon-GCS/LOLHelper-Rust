use crate::GamePhase;
use log::debug;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU16, Ordering};
use std::sync::{LazyLock, RwLock};

pub static CONTEXT: LazyLock<HelperContext> = LazyLock::new(HelperContext::new);

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Summoner {
    #[serde(rename = "gameName")]
    pub game_name: String,
    #[serde(rename = "summonerId")]
    pub summoner_id: u64,
    #[serde(rename = "summonerLevel")]
    pub summoner_level: u16,
    pub puuid: String,
}

// champion id and champion name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Champion(pub u16, pub String);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AutoPick {
    pub selected: Vec<Champion>,
    pub unselected: Vec<Champion>,
    pub enabled: bool,
}

#[derive(Debug, Default)]
pub struct HelperContext {
    // game state
    pub listening: AtomicBool,
    pub champion_id: AtomicU16,
    pub me: RwLock<Summoner>,
    pub game_phase: RwLock<GamePhase>,
    pub game_mode: RwLock<String>,

    // For auto pick champion
    pub subset_champion_list: RwLock<Vec<u16>>,
    // flags
    pub picked: AtomicBool,
    pub accepted: AtomicBool,
    // Settings
    pub auto_pick: RwLock<AutoPick>,
    pub auto_accepted_delay: AtomicU8,
    pub auto_send_analysis: AtomicBool,
}

impl HelperContext {
    pub fn new() -> Self {
        Self {
            auto_accepted_delay: AtomicU8::new(3),
            auto_send_analysis: AtomicBool::new(true),
            auto_pick: RwLock::new(AutoPick {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub fn reset(&self) {
        self.champion_id.store(0, Ordering::Relaxed);
        (*self.subset_champion_list.write().unwrap()).clear();
        self.game_mode.write().unwrap().clear();
        debug!("HelperContext reset");
    }
}
