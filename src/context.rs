use crate::lcu::{ChampSelectPlayer, GamePhase};
use crate::types::{ChampionId, ChampionName, SummonerId};
use log::debug;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};

const AUTO_PICK_FILE: &str = "auto_pick.json";

#[derive(Debug, Deserialize, Default)]
pub struct Summoner {
    #[serde(rename = "gameName")]
    pub game_name: String,
    #[serde(rename = "summonerId")]
    pub summoner_id: SummonerId,
    #[serde(rename = "summonerLevel")]
    pub summoner_level: u16,
    pub puuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Champion(pub ChampionId, pub ChampionName);

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
    pub my_team: RwLock<Vec<ChampSelectPlayer>>,
    pub game_phase: RwLock<GamePhase>,
    pub game_mode: RwLock<String>,
    pub conversation_id: RwLock<String>,

    // For auto pick champion
    pub subset_champion_list: RwLock<Vec<ChampionId>>,
    // flags
    pub picked: AtomicBool,
    pub accepted: AtomicBool,
    pub analysis_sent_flag: AtomicBool,
    // Settings
    pub auto_pick: RwLock<AutoPick>,
    pub auto_accepted_delay: RwLock<i8>,
    pub auto_send_analysis: AtomicBool,
}

impl HelperContext {
    pub fn new() -> Self {
        Self {
            auto_accepted_delay: RwLock::new(3),
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
        self.analysis_sent_flag.store(false, Ordering::Relaxed);
        (*self.my_team.write().unwrap()).clear();
        (*self.subset_champion_list.write().unwrap()).clear();
        self.conversation_id.write().unwrap().clear();
        self.game_mode.write().unwrap().clear();
        debug!("HelperContext reset");
    }

    pub fn from_storage(storage: &dyn eframe::Storage) -> Self {
        let auto_pick = serde_json::from_str(&storage.get_string("auto_pick").unwrap_or_default())
            .unwrap_or_default();
        let auto_accepted_delay = serde_json::from_str(
            &storage
                .get_string("auto_accepted_delay")
                .unwrap_or_default(),
        )
        .unwrap_or_default();
        let auto_send_analysis =
            serde_json::from_str(&storage.get_string("auto_send_analysis").unwrap_or_default())
                .unwrap_or_default();

        HelperContext {
            auto_pick,
            auto_accepted_delay,
            auto_send_analysis,
            ..Default::default()
        }
    }
}
