use crate::lcu::{ChampSelectPlayer, ChampionId, ChampionName, GamePhase};
use log::debug;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, sync::RwLock};

const AUTO_PICK_FILE: &str = "auto_pick.json";

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Champion(pub ChampionId, pub ChampionName);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AutoPick {
    // champion_id: pority
    pub selected: Vec<Champion>,
    pub unselected: Vec<Champion>,
    pub enabled: bool,
}

impl AutoPick {
    pub fn save(&self) {
        let file = File::create(AUTO_PICK_FILE).expect("保存自动选择数据失败");
        serde_json::to_writer(file, self).expect("序列化自动选择数据失败");
    }

    pub fn load(&mut self) {
        let path = std::path::Path::new(AUTO_PICK_FILE);
        if !path.exists() {
            return;
        }
        let file = File::open(path).expect("打开自动选择数据文件失败");
        let data: AutoPick = serde_json::from_reader(file).expect("反序列化自动选择数据失败");
        self.enabled = data.enabled;
        self.selected = data.selected;
        self.unselected = data.unselected;
    }
}

#[derive(Debug, Default)]
pub struct HelperContext {
    pub listening: RwLock<bool>,
    pub me: RwLock<Summoner>,
    pub champions: RwLock<HashMap<u16, String>>,
    pub champion_id: RwLock<u16>,
    pub my_team: RwLock<Vec<ChampSelectPlayer>>,
    pub game_phase: RwLock<GamePhase>,
    pub game_mode: RwLock<String>,
    pub conversation_id: RwLock<String>,

    // For auto pick champion
    pub subset_champion_list: RwLock<Vec<u16>>,
    pub auto_pick: RwLock<AutoPick>,
    pub picked: RwLock<bool>,
    // For auto accept
    pub accepted: RwLock<bool>,
    pub auto_accepted_delay: RwLock<isize>,
    // For auto send analysis
    pub auto_send_analysis: RwLock<bool>,
    pub analysis_sent_flag: RwLock<bool>,
}

impl HelperContext {
    pub fn new() -> Self {
        let ctx = Self::default();
        *ctx.auto_accepted_delay.write().unwrap() = 3;
        *ctx.auto_send_analysis.write().unwrap() = true;
        ctx.auto_pick.write().unwrap().enabled = true;
        ctx
    }

    pub fn reset(&self) {
        *self.champion_id.write().unwrap() = 0;
        (*self.my_team.write().unwrap()).clear();
        (*self.subset_champion_list.write().unwrap()).clear();
        self.conversation_id.write().unwrap().clear();
        self.game_mode.write().unwrap().clear();
        *self.analysis_sent_flag.write().unwrap() = false;
        debug!("HelperContext reset");
    }
}
